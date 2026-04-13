import { useCallback, useMemo, useRef, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { EmployeeRecord } from "../../types/staff"
import type { StaffGroupKey, ActiveTableEditCell, TableEditDrafts } from "../../types/app"
import { getErrorMessage } from "../../lib/utils"
import { DATE_COLUMN_KEYS } from "../../lib/constants"
import {
    buildEmployeePayloadForSave,
    isEmployeeTableEditableColumn,
    readEmployeeEditableCellText,
    readEmployeeCellText,
} from "./employeeTableRules"

type UseTableEditOptions = {
    employees: EmployeeRecord[]
    staffGroupFilter: StaffGroupKey
    canEditEmployeeTable: boolean
    canEditEmployeeComputerName: boolean
    triggerReload: () => void
    setGlobalError: (msg: string | null) => void
}

export type TableEditState = ReturnType<typeof useTableEdit>
const INACTIVITY_MS = 2 * 60 * 1000

// ── Cell text helpers ─────────────────────────────────────────────────────────

export function readRawCellText(employee: EmployeeRecord, key: string): string {
    return readEmployeeCellText(employee, key)
}

export function readCellValue(
    _employee: EmployeeRecord,
    key: string,
    index: number,
    draftValue: string,
    formatDate: (v: string | null) => string,
): string | number {
    if (key === "rowNumber") return index + 1
    if (DATE_COLUMN_KEYS.has(key)) return formatDate(draftValue || null)
    return draftValue || "-"
}

function isColumnSortable(key: string): boolean {
    const SORTABLE_KEYS = new Set([
        "employeeId",
        "fullName",
        "teamName",
        "email",
        "project",
        "aswStartDate",
        "clientStartDate",
    ])
    return SORTABLE_KEYS.has(key)
}

// ── Hook ─────────────────────────────────────────────────────────────────────

export function useTableEdit({
    employees,
    staffGroupFilter,
    canEditEmployeeTable,
    canEditEmployeeComputerName,
    triggerReload,
    setGlobalError,
}: UseTableEditOptions) {
    const [isTableEditMode, setTableEditMode] = useState(false)
    const [tableEditDrafts, setTableEditDrafts] = useState<TableEditDrafts>({})
    const [activeTableEditCell, setActiveTableEditCell] = useState<ActiveTableEditCell | null>(null)
    const [isSavingTableEdits, setSavingTableEdits] = useState(false)
    const [selectedMoveEmployeeIds, setSelectedMoveEmployeeIds] = useState<number[]>([])
    const [moveTargetGroup, setMoveTargetGroup] = useState<StaffGroupKey>("employee_list")
    const [isMovingEmployees, setMovingEmployees] = useState(false)

    // ── Inactivity auto-exit ──────────────────────────────────────────────────────
    const inactivityTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

    const clearInactivityTimer = useCallback(() => {
        if (inactivityTimer.current) {
            clearTimeout(inactivityTimer.current)
            inactivityTimer.current = null
        }
    }, [])

    const resetInactivityTimer = useCallback(() => {
        clearInactivityTimer()
        inactivityTimer.current = setTimeout(() => {
            setTableEditMode(false)
            setTableEditDrafts({})
            setActiveTableEditCell(null)
            setSelectedMoveEmployeeIds([])
        }, INACTIVITY_MS)
    }, [clearInactivityTimer])

    const selectedMoveEmployeeIdSet = useMemo(
        () => new Set(selectedMoveEmployeeIds),
        [selectedMoveEmployeeIds],
    )

    const currentPageEmployeeIds = useMemo(
        () => employees.map((emp) => emp.id),
        [employees],
    )

    const isCurrentPageFullySelected = useMemo(
        () =>
            currentPageEmployeeIds.length > 0 &&
            currentPageEmployeeIds.every((id) => selectedMoveEmployeeIdSet.has(id)),
        [currentPageEmployeeIds, selectedMoveEmployeeIdSet],
    )

    const selectedOnCurrentPageCount = useMemo(
        () => currentPageEmployeeIds.filter((id) => selectedMoveEmployeeIdSet.has(id)).length,
        [currentPageEmployeeIds, selectedMoveEmployeeIdSet],
    )

    const editedRowCount = useMemo(
        () => Object.keys(tableEditDrafts).length,
        [tableEditDrafts],
    )

    const hasPendingTableEdits = editedRowCount > 0

    const canMoveSelectedRows = useMemo(
        () => selectedMoveEmployeeIds.length > 0 && moveTargetGroup !== staffGroupFilter,
        [moveTargetGroup, selectedMoveEmployeeIds.length, staffGroupFilter],
    )

    // ── Cell draft helpers ───────────────────────────────────────────────────────

    const getDraftCellText = useCallback(
        (employee: EmployeeRecord, key: string): string => {
            const draft = tableEditDrafts[employee.id]?.[key]
            if (draft !== undefined) return draft
            return readEmployeeEditableCellText(employee, key, canEditEmployeeComputerName)
        },
        [canEditEmployeeComputerName, tableEditDrafts],
    )

    const setDraftCellText = useCallback(
        (employee: EmployeeRecord, key: string, value: string) => {
            resetInactivityTimer()  // reset 2-min timer on every keystroke
            setTableEditDrafts((prev) => {
                const rowDrafts = { ...(prev[employee.id] ?? {}) }
                const originalValue = readEmployeeEditableCellText(
                    employee,
                    key,
                    canEditEmployeeComputerName,
                )
                if (value === originalValue) {
                    delete rowDrafts[key]
                } else {
                    rowDrafts[key] = value
                }
                if (Object.keys(rowDrafts).length === 0) {
                    const next = { ...prev }
                    delete next[employee.id]
                    return next
                }
                return { ...prev, [employee.id]: rowDrafts }
            })
        },
        [canEditEmployeeComputerName, resetInactivityTimer],
    )

    const startTableCellEdit = useCallback(
        (employee: EmployeeRecord, key: string) => {
            if (
                !canEditEmployeeTable ||
                !isTableEditMode ||
                !isEmployeeTableEditableColumn(key, canEditEmployeeComputerName)
            ) {
                return
            }
            resetInactivityTimer()  // reset timer when user clicks a cell
            setActiveTableEditCell({ employeeId: employee.id, columnKey: key })
        },
        [canEditEmployeeComputerName, canEditEmployeeTable, isTableEditMode, resetInactivityTimer],
    )

    // ── Toggle edit mode ─────────────────────────────────────────────────────────

    const handleToggleTableEditMode = () => {
        if (isTableEditMode) {
            clearInactivityTimer()
            setTableEditMode(false)
            setTableEditDrafts({})
            setActiveTableEditCell(null)
            setSelectedMoveEmployeeIds([])
        } else {
            setTableEditMode(true)
            resetInactivityTimer()  // start 2-min countdown when entering edit mode
        }
    }

    // ── Save edits ───────────────────────────────────────────────────────────────

    const handleSaveTableEdits = async () => {
        if (!hasPendingTableEdits || isSavingTableEdits) return

        const updates = Object.entries(tableEditDrafts).map(([rawId, drafts]) => ({
            id: Number(rawId),
            drafts,
        }))

        try {
            setSavingTableEdits(true)
            for (const { id, drafts } of updates) {
                const employee = employees.find((emp) => emp.id === id)
                if (!employee) continue

                const payload = buildEmployeePayloadForSave(
                    employee,
                    drafts,
                    canEditEmployeeComputerName,
                )
                await staffApi.updateEmployee(id, payload)
            }
            setTableEditDrafts({})
            setActiveTableEditCell(null)
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setSavingTableEdits(false)
        }
    }

    // ── Move employees ───────────────────────────────────────────────────────────

    const handleMoveSelectedEmployees = async () => {
        if (!canMoveSelectedRows || isMovingEmployees) return

        try {
            setMovingEmployees(true)
            await staffApi.moveEmployeesGroup({
                employeeIds: selectedMoveEmployeeIds,
                targetStaffGroup: moveTargetGroup,
            })
            setSelectedMoveEmployeeIds([])
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setMovingEmployees(false)
        }
    }

    const toggleMoveEmployeeSelection = (id: number) => {
        setSelectedMoveEmployeeIds((prev) => {
            const set = new Set(prev)
            if (set.has(id)) {
                set.delete(id)
            } else {
                set.add(id)
            }
            return [...set]
        })
    }

    const toggleSelectCurrentPageEmployees = () => {
        if (isCurrentPageFullySelected) {
            setSelectedMoveEmployeeIds((prev) =>
                prev.filter((id) => !currentPageEmployeeIds.includes(id)),
            )
        } else {
            setSelectedMoveEmployeeIds((prev) => {
                const merged = new Set([...prev, ...currentPageEmployeeIds])
                return [...merged]
            })
        }
    }

    const clearMoveSelection = () => setSelectedMoveEmployeeIds([])

    const resetTableEditStateOnLogout = useCallback(() => {
        clearInactivityTimer()
        setTableEditMode(false)
        setTableEditDrafts({})
        setActiveTableEditCell(null)
        setSelectedMoveEmployeeIds([])
    }, [clearInactivityTimer])

    return {
        isTableEditMode,
        tableEditDrafts,
        activeTableEditCell,
        setActiveTableEditCell,
        isSavingTableEdits,
        selectedMoveEmployeeIds,
        selectedMoveEmployeeIdSet,
        currentPageEmployeeIds,
        isCurrentPageFullySelected,
        selectedOnCurrentPageCount,
        editedRowCount,
        hasPendingTableEdits,
        moveTargetGroup,
        setMoveTargetGroup,
        isMovingEmployees,
        canMoveSelectedRows,
        getDraftCellText,
        setDraftCellText,
        startTableCellEdit,
        handleToggleTableEditMode,
        handleSaveTableEdits,
        handleMoveSelectedEmployees,
        toggleMoveEmployeeSelection,
        toggleSelectCurrentPageEmployees,
        clearMoveSelection,
        resetTableEditStateOnLogout,
        // utilities re-exported for use in views
        readRawCellText,
        readCellValue,
        isTableEditableColumn: (key: string) =>
            isEmployeeTableEditableColumn(key, canEditEmployeeComputerName),
        isColumnSortable,
    }
}
