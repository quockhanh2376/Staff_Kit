import { useCallback, useMemo, useRef, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { EmployeeRecord, EmployeePayload } from "../../types/staff"
import type { StaffGroupKey, ActiveTableEditCell, TableEditDrafts } from "../../types/app"
import { getErrorMessage } from "../../lib/utils"
import { DATE_COLUMN_KEYS } from "../../lib/constants"

type UseTableEditOptions = {
    employees: EmployeeRecord[]
    staffGroupFilter: StaffGroupKey
    canEditEmployeeTable: boolean
    triggerReload: () => void
    setGlobalError: (msg: string | null) => void
}

export type TableEditState = ReturnType<typeof useTableEdit>

// ── Cell text helpers ─────────────────────────────────────────────────────────

export function readRawCellText(employee: EmployeeRecord, key: string): string {
    if (key === "rowNumber") return ""
    const record = employee as Record<string, unknown>
    // Try top-level property first, then fall back to dynamicFields
    const value = key in record ? record[key] : (employee.dynamicFields?.[key] ?? undefined)
    if (value === null || value === undefined) return ""
    return String(value)
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

function isTableEditableColumn(key: string): boolean {
    if (key === "rowNumber") return false
    if (key === "employeeId") return false
    return true
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
    const INACTIVITY_MS = 2 * 60 * 1000 // 2 minutes
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
            return readRawCellText(employee, key)
        },
        [tableEditDrafts],
    )

    const setDraftCellText = useCallback(
        (employee: EmployeeRecord, key: string, value: string) => {
            resetInactivityTimer()  // reset 2-min timer on every keystroke
            setTableEditDrafts((prev) => {
                const rowDrafts = { ...(prev[employee.id] ?? {}) }
                const originalValue = readRawCellText(employee, key)
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
        [resetInactivityTimer],
    )

    const startTableCellEdit = useCallback(
        (employee: EmployeeRecord, key: string) => {
            if (!canEditEmployeeTable || !isTableEditMode || !isTableEditableColumn(key)) return
            resetInactivityTimer()  // reset timer when user clicks a cell
            setActiveTableEditCell({ employeeId: employee.id, columnKey: key })
        },
        [canEditEmployeeTable, isTableEditMode, resetInactivityTimer],
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

                // Top-level keys that map directly to EmployeePayload fields
                const TOP_LEVEL_KEYS = new Set([
                    "employeeId", "fullName", "nickName", "teamName", "project",
                    "jobTitle", "email", "cellphone", "dateOfBirth", "gender",
                    "aswStartDate", "clientStartDate", "contractEndDate",
                    "clientYearOfServices", "computerName", "notes",
                ])

                // Separate drafts: top-level vs dynamic fields
                const dynamicDrafts: Record<string, string> = {}
                for (const [key, value] of Object.entries(drafts)) {
                    if (!TOP_LEVEL_KEYS.has(key)) {
                        dynamicDrafts[key] = value
                    }
                }

                // Build a valid EmployeePayload by merging drafts over existing data
                const payload: EmployeePayload = {
                    employeeId: (drafts.employeeId ?? employee.employeeId) || "",
                    fullName: (drafts.fullName ?? employee.fullName) || "",
                    nickName: drafts.nickName ?? employee.nickName ?? null,
                    teamName: drafts.teamName ?? employee.teamName ?? null,
                    project: drafts.project ?? employee.project ?? null,
                    jobTitle: drafts.jobTitle ?? employee.jobTitle ?? null,
                    email: drafts.email ?? employee.email ?? null,
                    cellphone: drafts.cellphone ?? employee.cellphone ?? null,
                    dateOfBirth: drafts.dateOfBirth ?? employee.dateOfBirth ?? null,
                    gender: drafts.gender ?? employee.gender ?? null,
                    aswStartDate: drafts.aswStartDate ?? employee.aswStartDate ?? null,
                    clientStartDate: drafts.clientStartDate ?? employee.clientStartDate ?? null,
                    contractEndDate: drafts.contractEndDate ?? employee.contractEndDate ?? null,
                    clientYearOfServices: drafts.clientYearOfServices ?? employee.clientYearOfServices ?? null,
                    computerName: drafts.computerName ?? employee.computerName ?? null,
                    notes: drafts.notes ?? employee.notes ?? null,
                    // Merge dynamic field drafts (e.g. computer_2) into existing dynamicFields
                    dynamicFields: Object.keys(dynamicDrafts).length > 0
                        ? { ...(employee.dynamicFields ?? {}), ...dynamicDrafts }
                        : (employee.dynamicFields ?? null),
                }


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
        isTableEditableColumn,
        isColumnSortable,
    }
}
