import React, { useMemo, useState } from "react"
import { AlertTriangle, Check, LoaderCircle, Search, Users, X } from "lucide-react"
import type { EmployeeRecord } from "../../types/staff"
import { formatEmployeeIdForDisplay } from "./employeeIdDisplay"

export type MissingEmployeesModalProps = {
    open: boolean
    onClose: () => void
    missingRows: Array<{
        code: string
        localName: string | null
    }>
    onOpenEditEmployee: (employee: EmployeeRecord) => void
    onBulkMoveGroup: (employeeIds: number[], targetGroup: string) => Promise<void>
    fetchAllFilteredEmployees: () => Promise<EmployeeRecord[]>
    canEdit: boolean
}

const VALID_TARGET_GROUPS = [
    { key: "offboarding", label: "Offboard (Resigned)" },
    { key: "internal_movement", label: "Move to Internal Movement" },
    { key: "onboarding", label: "Move to Onboarding" },
    { key: "set_floating", label: "Mark as Floating (In Transition)" },
]

export function MissingEmployeesModal({
    open,
    onClose,
    missingRows,
    onOpenEditEmployee,
    onBulkMoveGroup,
    fetchAllFilteredEmployees,
    canEdit,
}: MissingEmployeesModalProps) {
    const [searchTerm, setSearchTerm] = useState("")
    const [selectedIds, setSelectedIds] = useState<number[]>([])
    const [targetGroup, setTargetGroup] = useState<string>("offboarding")
    const [allEmployees, setAllEmployees] = useState<EmployeeRecord[]>([])
    const [hasLoadedEmployees, setHasLoadedEmployees] = useState(false)
    const [isLoadingEmployees, setIsLoadingEmployees] = useState(false)
    const [isMoving, setIsMoving] = useState(false)
    const [errorMessage, setErrorMessage] = useState<string | null>(null)
    const [showConfirmation, setShowConfirmation] = useState(false)

    // Load full EmployeeRecords to map missing rows to local records (id, current staff_group)
    React.useEffect(() => {
        if (!open) {
            setSearchTerm("")
            setSelectedIds([])
            setTargetGroup("offboarding")
            setErrorMessage(null)
            setShowConfirmation(false)
            setHasLoadedEmployees(false)
            return
        }

        let disposed = false
        setIsLoadingEmployees(true)
        void (async () => {
            try {
                const emps = await fetchAllFilteredEmployees()
                if (!disposed) {
                    setAllEmployees(emps)
                    setHasLoadedEmployees(true)
                }
            } catch (err) {
                if (!disposed) {
                    setErrorMessage(err instanceof Error ? err.message : String(err))
                }
            } finally {
                if (!disposed) setIsLoadingEmployees(false)
            }
        })()

        return () => {
            disposed = true
        }
    }, [open, fetchAllFilteredEmployees])

    // Match missingRows (by normalized code) with local employees
    const missingEmployees = useMemo(() => {
        const empByCode = new Map<string, EmployeeRecord>()
        for (const emp of allEmployees) {
            empByCode.set(emp.employeeId.trim().toUpperCase(), emp)
        }

        return missingRows.map((row) => {
            const matched = empByCode.get(row.code.trim().toUpperCase())
            return {
                id: matched?.id ?? -1,
                code: row.code,
                fullName: matched?.fullName ?? row.localName ?? "Unknown",
                staffGroup: matched?.staffGroup ?? "employee_list",
                isFloating: matched?.isFloating ?? false,
                record: matched ?? null,
            }
        })
    }, [missingRows, allEmployees])

    // Filter by search
    const filteredRows = useMemo(() => {
        const query = searchTerm.trim().toLowerCase()
        if (!query) return missingEmployees
        return missingEmployees.filter((item) =>
            item.code.toLowerCase().includes(query) ||
            item.fullName.toLowerCase().includes(query)
        )
    }, [missingEmployees, searchTerm])

    // Selection handlers
    const toggleSelect = (id: number) => {
        if (id <= 0) return
        setSelectedIds((prev) =>
            prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
        )
    }

    const selectAllFiltered = () => {
        const validFilteredIds = filteredRows.map((r) => r.id).filter((id) => id > 0)
        const allSelected = validFilteredIds.length > 0 && validFilteredIds.every((id) => selectedIds.includes(id))

        if (allSelected) {
            setSelectedIds((prev) => prev.filter((id) => !validFilteredIds.includes(id)))
        } else {
            setSelectedIds((prev) => Array.from(new Set([...prev, ...validFilteredIds])))
        }
    }

    const selectedEmployees = useMemo(() => {
        const idSet = new Set(selectedIds)
        return missingEmployees.filter((e) => idSet.has(e.id))
    }, [selectedIds, missingEmployees])

    const targetGroupLabel = useMemo(() => {
        return VALID_TARGET_GROUPS.find((g) => g.key === targetGroup)?.label ?? targetGroup
    }, [targetGroup])

    const handleConfirmApply = async () => {
        if (selectedIds.length === 0 || isMoving) return
        setIsMoving(true)
        setErrorMessage(null)
        try {
            await onBulkMoveGroup(selectedIds, targetGroup)
            setShowConfirmation(false)
            onClose()
        } catch (err) {
            setErrorMessage(err instanceof Error ? err.message : String(err))
            setShowConfirmation(false)
        } finally {
            setIsMoving(false)
        }
    }

    if (!open) return null

    return (
        <div
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm"
            role="dialog"
            aria-modal="true"
            aria-label="Missing Employees Drill-down"
        >
            <div className="relative flex max-h-[90vh] w-full max-w-3xl flex-col rounded-[16px] border border-[var(--border)] bg-[var(--surface)] shadow-2xl">
                {/* Header */}
                <div className="flex items-center justify-between border-b border-[var(--border)] px-6 py-4">
                    <div className="flex items-center gap-2">
                        <Users className="text-amber-400" size={20} />
                        <h2 className="text-lg font-semibold text-[var(--text-primary)]">
                            Missing Employees ({missingRows.length})
                        </h2>
                    </div>
                    <button
                        type="button"
                        onClick={onClose}
                        className="rounded-md p-1.5 text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                        aria-label="Close"
                    >
                        <X size={18} />
                    </button>
                </div>

                {/* Subtitle Warning Banner */}
                <div className="border-b border-[var(--border)] bg-amber-500/10 px-6 py-2.5 text-xs text-amber-300">
                    <div className="flex items-center gap-1.5 font-medium">
                        <AlertTriangle size={14} className="shrink-0" />
                        <span>Missing in remote HRMS Query B does not automatically mean an employee has resigned. Review records and transfer manually if needed.</span>
                    </div>
                </div>

                {errorMessage && (
                    <div className="mx-6 mt-4 rounded-md border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-300">
                        {errorMessage}
                    </div>
                )}

                {/* Body / Table Area */}
                {!showConfirmation ? (
                    <>
                        {/* Search and Action Toolbar */}
                        <div className="flex flex-wrap items-center justify-between gap-3 px-6 py-3 border-b border-[var(--border)]">
                            <div className="relative min-w-[240px] flex-1">
                                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-secondary)]" size={14} />
                                <input
                                    type="text"
                                    aria-label="Search Missing Employees"
                                    placeholder="Search by Staff ID or Name..."
                                    className="w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/40 pl-8 pr-3 py-1.5 text-xs text-[var(--text-primary)] placeholder-[var(--text-secondary)]/60 outline-none focus:border-[var(--primary)]"
                                    value={searchTerm}
                                    onChange={(e) => setSearchTerm(e.target.value)}
                                />
                            </div>

                            <div className="flex items-center gap-2">
                                <select
                                    aria-label="Target Group"
                                    value={targetGroup}
                                    onChange={(e) => setTargetGroup(e.target.value)}
                                    disabled={!canEdit || isMoving || selectedIds.length === 0}
                                    className="rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-2.5 py-1.5 text-xs text-[var(--text-primary)] disabled:opacity-50"
                                >
                                    {VALID_TARGET_GROUPS.map((group) => (
                                        <option key={group.key} value={group.key}>
                                            {group.label}
                                        </option>
                                    ))}
                                </select>

                                <button
                                    type="button"
                                    disabled={!canEdit || isMoving || selectedIds.length === 0}
                                    onClick={() => setShowConfirmation(true)}
                                    className="rounded-[8px] bg-[var(--primary)] px-3 py-1.5 text-xs font-semibold text-white hover:opacity-90 disabled:opacity-40"
                                >
                                    Move Group ({selectedIds.length})
                                </button>
                            </div>
                        </div>

                        {/* List / Table */}
                        <div className="flex-1 overflow-auto px-6 py-2">
                            {isLoadingEmployees && !hasLoadedEmployees ? (
                                <div className="flex items-center justify-center py-12 text-xs text-[var(--text-secondary)]">
                                    <LoaderCircle className="mr-2 animate-spin" size={16} />
                                    Loading employee details...
                                </div>
                            ) : filteredRows.length === 0 ? (
                                <div className="py-12 text-center text-xs text-[var(--text-secondary)]">
                                    No missing employees matched your filter.
                                </div>
                            ) : (
                                <table className="w-full text-left text-xs">
                                    <thead className="sticky top-0 bg-[var(--surface)] text-[var(--text-secondary)] border-b border-[var(--border)]">
                                        <tr>
                                            <th className="py-2 px-2 w-10">
                                                <input
                                                    type="checkbox"
                                                    aria-label="Select all filtered missing employees"
                                                    checked={
                                                        filteredRows.length > 0 &&
                                                        filteredRows.every((r) => r.id > 0 && selectedIds.includes(r.id))
                                                    }
                                                    onChange={selectAllFiltered}
                                                    disabled={!canEdit}
                                                />
                                            </th>
                                            <th className="py-2 px-2 font-semibold">Staff ID</th>
                                            <th className="py-2 px-2 font-semibold">Full Name</th>
                                            <th className="py-2 px-2 font-semibold">Current Group</th>
                                            <th className="py-2 px-2 text-right font-semibold">Action</th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-[var(--border)]">
                                        {filteredRows.map((row) => {
                                            const isSelected = selectedIds.includes(row.id)
                                            return (
                                                <tr
                                                    key={row.code}
                                                    className={`hover:bg-[var(--surface-hover)] ${isSelected ? "bg-[var(--primary)]/10" : ""}`}
                                                >
                                                    <td className="py-2.5 px-2">
                                                        <input
                                                            type="checkbox"
                                                            aria-label={`Select ${row.code}`}
                                                            checked={isSelected}
                                                            onChange={() => toggleSelect(row.id)}
                                                            disabled={!canEdit || row.id <= 0}
                                                        />
                                                    </td>
                                                    <td className="py-2.5 px-2 font-mono font-medium text-[var(--primary)]">
                                                        {formatEmployeeIdForDisplay(row.code)}
                                                    </td>
                                                    <td className="py-2.5 px-2 font-medium text-[var(--text-primary)]">
                                                        {row.fullName}
                                                    </td>
                                                    <td className="py-2.5 px-2 text-[var(--text-secondary)]">
                                                        <span className="rounded bg-[var(--surface-hover)] px-2 py-0.5 text-[11px] capitalize">
                                                            {row.isFloating ? "Floating (EE List)" : row.staffGroup.replace("_", " ")}
                                                        </span>
                                                    </td>
                                                    <td className="py-2.5 px-2 text-right">
                                                        {row.record && (
                                                            <button
                                                                type="button"
                                                                onClick={() => {
                                                                    onClose()
                                                                    onOpenEditEmployee(row.record!)
                                                                }}
                                                                className="rounded-[6px] border border-[var(--border)] px-2.5 py-1 text-xs text-[var(--text-secondary)] hover:border-[var(--primary)] hover:text-[var(--primary)]"
                                                            >
                                                                Edit
                                                            </button>
                                                        )}
                                                    </td>
                                                </tr>
                                            )
                                        })}
                                    </tbody>
                                </table>
                            )}
                        </div>

                        {/* Footer */}
                        <div className="flex items-center justify-between border-t border-[var(--border)] px-6 py-3 text-xs text-[var(--text-secondary)]">
                            <div>
                                {selectedIds.length} of {missingEmployees.length} selected
                            </div>
                            <button
                                type="button"
                                onClick={onClose}
                                className="rounded-[8px] border border-[var(--border)] px-4 py-1.5 font-medium hover:bg-[var(--surface-hover)]"
                            >
                                Close
                            </button>
                        </div>
                    </>
                ) : (
                    /* Confirmation Dialog Step */
                    <div className="p-6">
                        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface-hover)]/30 p-4">
                            <h3 className="text-sm font-semibold text-[var(--text-primary)]">
                                Confirm Group Transition
                            </h3>
                            <p className="mt-2 text-xs text-[var(--text-secondary)] leading-relaxed">
                                You are about to move <b>{selectedEmployees.length}</b> employee(s) to <b>{targetGroupLabel}</b>:
                            </p>

                            <div className="mt-3 max-h-48 overflow-auto rounded-[8px] border border-[var(--border)] bg-[var(--surface)] p-2">
                                <ul className="space-y-1.5 text-xs divide-y divide-[var(--border)]">
                                    {selectedEmployees.map((emp) => (
                                        <li key={emp.id} className="pt-1.5 first:pt-0 flex items-center justify-between">
                                            <span>
                                                <strong className="font-mono text-[var(--primary)]">{emp.code}</strong> — {emp.fullName}
                                            </span>
                                            <span className="text-[11px] text-[var(--text-secondary)] capitalize">
                                                {emp.isFloating ? "floating" : emp.staffGroup.replace("_", " ")} → {targetGroup.replace("_", " ")}
                                            </span>
                                        </li>
                                    ))}
                                </ul>
                            </div>

                            <div className="mt-4 rounded-md border border-amber-500/30 bg-amber-500/10 p-2.5 text-xs text-amber-300">
                                <strong>Notice:</strong> Moving an employee from the active EE List removes them from current EE Sync comparison. Please verify that this transition is intentional and adheres to HR policies.
                            </div>
                        </div>

                        <div className="mt-6 flex items-center justify-end gap-3">
                            <button
                                type="button"
                                disabled={isMoving}
                                onClick={() => setShowConfirmation(false)}
                                className="rounded-[8px] border border-[var(--border)] px-4 py-2 text-xs font-medium text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                            >
                                Cancel
                            </button>
                            <button
                                type="button"
                                disabled={isMoving}
                                onClick={() => void handleConfirmApply()}
                                className="inline-flex items-center gap-1.5 rounded-[8px] bg-[var(--primary)] px-4 py-2 text-xs font-semibold text-white hover:opacity-90 disabled:opacity-50"
                            >
                                {isMoving ? (
                                    <>
                                        <LoaderCircle className="animate-spin" size={14} />
                                        Applying changes...
                                    </>
                                ) : (
                                    <>
                                        <Check size={14} className="stroke-[3]" />
                                        Approve & Move
                                    </>
                                )}
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    )
}
