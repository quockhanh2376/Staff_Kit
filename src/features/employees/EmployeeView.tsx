import {
    AlertTriangle,
    ArrowUpDown,
    Check,
    ChevronDown,
    LoaderCircle,
    Search,
    Settings,
    Trash2,
    Users,
    X,
} from "lucide-react"
import { useEffect, useState, useCallback, useRef } from "react"
import { staffApi } from "../../services/staff-api"
import type { EmployeeState } from "./useEmployeeState"
import type { TableEditState } from "./useTableEdit"
import type { ColumnState } from "../columns/useColumnState"
import { DATE_COLUMN_KEYS } from "../../lib/constants"
import { formatDate } from "../../lib/utils"
import { formatEmployeeIdForDisplay } from "./employeeIdDisplay"
import { collectDuplicateComputerEmployeeIds } from "./employeeTableRules"
import { EmployeeCellContent } from "./EmployeeCellContent"
import { downloadCsv, downloadXlsx } from "./employeeListExport"
import { getUserErrorMessage } from "../../lib/errorHandling"

// ── Diacritic-insensitive highlight helper ─────────────────────────────────────
function stripDiacritics(str: string): string {
    return str.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase()
}

function hasSearchMatch(text: string, query: string): boolean {
    const trimmedQuery = query.trim()
    if (!trimmedQuery) return false

    return stripDiacritics(text).includes(stripDiacritics(trimmedQuery))
}

function HighlightText({ text, query }: { text: string; query: string }) {
    const trimmedQuery = query.trim()
    if (!trimmedQuery) return <>{text}</>

    const normQuery = stripDiacritics(trimmedQuery)
    const normText = stripDiacritics(text)
    const parts: React.ReactNode[] = []
    let cursor = 0
    let searchFrom = 0
    while (searchFrom < normText.length) {
        const idx = normText.indexOf(normQuery, searchFrom)
        if (idx === -1) break
        if (idx > cursor) parts.push(text.slice(cursor, idx))
        parts.push(
            <mark
                key={idx}
                className="search-highlight-mark"
            >
                {text.slice(idx, idx + normQuery.length)}
            </mark>
        )
        cursor = idx + normQuery.length
        searchFrom = cursor
    }
    if (cursor < text.length) parts.push(text.slice(cursor))
    return <>{parts}</>
}

type EmployeeViewProps = {
    employeeState: EmployeeState
    tableEdit: TableEditState
    columnState: ColumnState
    canEditEmployeeTable: boolean
    canEditEmployeeComputerName: boolean
    isAdminAccount: boolean
    setGlobalError: (message: string | null) => void
    selectedGroupLabel: string
    selectedGroupTotal: number
}

export function EmployeeView({
    employeeState,
    tableEdit,
    columnState,
    canEditEmployeeTable,
    canEditEmployeeComputerName,
    isAdminAccount,
    setGlobalError,
    selectedGroupLabel,
    selectedGroupTotal,
}: EmployeeViewProps) {
    const emp = employeeState
    const edit = tableEdit
    const col = columnState
    const firstVisibleColumnKey = col.visibleColumns[0]?.key ?? ""
    const clearMoveSelection = edit.clearMoveSelection

    // ── Duplicate computer check ─────────────────────────────────────────────────
    const [duplicateEmpIds, setDuplicateEmpIds] = useState<Set<number>>(new Set())
    const [isCheckingDuplicates, setIsCheckingDuplicates] = useState(false)
    const [isExporting, setIsExporting] = useState(false)
    const [isUnselectMenuOpen, setIsUnselectMenuOpen] = useState(false)
    const [isExportMenuOpen, setIsExportMenuOpen] = useState(false)
    const unselectMenuRef = useRef<HTMLDetailsElement | null>(null)
    const exportMenuRef = useRef<HTMLDetailsElement | null>(null)
    const unselectTimerRef = useRef<number | null>(null)
    const exportTimerRef = useRef<number | null>(null)

    const clearMenuTimer = useCallback((timerRef: React.MutableRefObject<number | null>) => {
        if (timerRef.current !== null) {
            window.clearTimeout(timerRef.current)
            timerRef.current = null
        }
    }, [])

    const closeUnselectMenu = useCallback(() => {
        clearMenuTimer(unselectTimerRef)
        setIsUnselectMenuOpen(false)
    }, [clearMenuTimer])

    const closeExportMenu = useCallback(() => {
        clearMenuTimer(exportTimerRef)
        setIsExportMenuOpen(false)
    }, [clearMenuTimer])

    const resetUnselectTimer = useCallback(() => {
        clearMenuTimer(unselectTimerRef)
        unselectTimerRef.current = window.setTimeout(closeUnselectMenu, 3000)
    }, [clearMenuTimer, closeUnselectMenu])

    const resetExportTimer = useCallback(() => {
        clearMenuTimer(exportTimerRef)
        exportTimerRef.current = window.setTimeout(closeExportMenu, 3000)
    }, [clearMenuTimer, closeExportMenu])

    const openUnselectMenu = useCallback(() => {
        closeExportMenu()
        setIsUnselectMenuOpen(true)
        resetUnselectTimer()
    }, [closeExportMenu, resetUnselectTimer])

    const openExportMenu = useCallback(() => {
        closeUnselectMenu()
        setIsExportMenuOpen(true)
        resetExportTimer()
    }, [closeUnselectMenu, resetExportTimer])

    useEffect(() => {
        const closeOnOutsidePointer = (event: PointerEvent) => {
            const target = event.target as Node | null
            if (target && !unselectMenuRef.current?.contains(target) && !exportMenuRef.current?.contains(target)) {
                closeUnselectMenu()
                closeExportMenu()
            }
        }
        document.addEventListener("pointerdown", closeOnOutsidePointer)
        return () => {
            document.removeEventListener("pointerdown", closeOnOutsidePointer)
            clearMenuTimer(unselectTimerRef)
            clearMenuTimer(exportTimerRef)
        }
    }, [clearMenuTimer, closeExportMenu, closeUnselectMenu])

    useEffect(() => {
        clearMoveSelection()
    }, [clearMoveSelection, emp.searchTerm, emp.teamFilter, emp.startDateFilter])

    const handleSelectAllResults = useCallback(async () => {
        try {
            const employees = await emp.fetchAllFilteredEmployees()
            edit.selectEmployeeIds(employees.map((employee) => employee.id))
        } catch {
            // The existing global employee-loading error path remains authoritative.
        }
    }, [edit, emp])

    const handleExport = useCallback(async (format: "csv" | "xlsx") => {
        if (edit.selectedMoveEmployeeIds.length === 0 || isExporting) return
        setIsExporting(true)
        try {
            const allFilteredEmployees = await emp.fetchAllFilteredEmployees()
            const selectedIds = new Set(edit.selectedMoveEmployeeIds)
            const selectedEmployees = allFilteredEmployees.filter((employee) => selectedIds.has(employee.id))
            const columns = col.visibleColumns.map(({ key, label }) => ({ key, label }))
            const count = selectedEmployees.length
            const date = new Date().toISOString().slice(0, 10)
            const filename = `Employee_List_Selected_${count}_${date}`
            const options = { employees: selectedEmployees, columns, drafts: edit.tableEditDrafts }
            if (format === "csv") {
                await downloadCsv(options, `${filename}.csv`)
            } else {
                await downloadXlsx(options, `${filename}.xlsx`)
            }
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setIsExporting(false)
        }
    }, [col.visibleColumns, edit, emp, isExporting, setGlobalError])

    const handleCheckDuplicates = useCallback(async () => {
        setIsCheckingDuplicates(true)
        try {
            // Fetch all employees (no limit) to detect cross-page duplicates
            const response = await staffApi.searchEmployees({ limit: 99999, offset: 0 })
            const all = response.items

            // Identify computer-related column keys from visible columns
            const computerKeys = col.visibleColumns
                .filter(
                    (c) =>
                        c.key === "computerName" ||
                        c.label.toLowerCase().includes("computer"),
                )
                .map((c) => c.key)

            if (computerKeys.length === 0) {
                // Fallback: always include computerName
                computerKeys.push("computerName")
            }

            // Build map: normalized computer value → list of employee IDs
            setDuplicateEmpIds(collectDuplicateComputerEmployeeIds(all, computerKeys))
        } catch {
            // ignore
        } finally {
            setIsCheckingDuplicates(false)
        }
    }, [col.visibleColumns])

    // Auto-clear highlight when all duplicates are resolved (re-check after save)
    const clearDuplicates = useCallback(() => setDuplicateEmpIds(new Set()), [])


    return (
        <section>
            {/* Header */}
            <div className="flex flex-col gap-4 border-b border-[var(--border)] px-4 py-7 md:px-8">
                <div className="flex items-start gap-4">
                    <div>
                        <h1 className="text-[34px] font-bold leading-tight">{selectedGroupLabel}</h1>
                        <p className="mt-1 text-[15px] text-[var(--text-secondary)]">
                            Current employees in this group:{" "}
                            <span className="font-semibold text-[var(--text-primary)]">{selectedGroupTotal}</span>
                            . User can choose visible columns and reorder them by drag-and-drop.
                        </p>
                    </div>
                </div>

                {/* Filters */}
                <div className="flex flex-wrap items-center gap-3 xl:flex-nowrap">
                    <div className="relative min-w-[280px] flex-1 xl:max-w-[560px]">
                        <Search
                            className="pointer-events-none absolute left-4 top-1/2 -translate-y-1/2 text-[var(--text-secondary)]"
                            size={18}
                        />
                        <input
                            className="h-10 w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] pl-11 pr-11 text-sm text-[var(--text-primary)] outline-none transition focus:border-[var(--primary)]"
                            placeholder="Search name, EE.ID, email, computer (e.g. ASWVNLAP)..."
                            value={emp.searchTerm}
                            onChange={(event) => emp.setSearchTerm(event.target.value)}
                        />
                        <button
                            aria-label="Clear employee search"
                            className={`absolute right-2 top-1/2 inline-flex h-7 w-7 -translate-y-1/2 items-center justify-center rounded-[6px] text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] disabled:cursor-default disabled:hover:bg-transparent ${
                                emp.searchTerm.trim() ? "opacity-100" : "opacity-35"
                            }`}
                            disabled={!emp.searchTerm.trim()}
                            onClick={() => emp.setSearchTerm("")}
                            title="Clear"
                            type="button"
                        >
                            <X size={14} />
                        </button>
                    </div>

                    {/* Team filter */}
                    <div className="relative" ref={emp.teamFilterMenuRef}>
                        <div className={`filter-chip ${emp.isTeamFilterMenuOpen ? "filter-chip-open" : ""}`}>
                            <Users size={15} />
                            <input
                                className="filter-select-input"
                                value={emp.teamFilterSearchTerm}
                                onChange={(event) => {
                                    emp.setTeamFilterSearchTerm(event.target.value)
                                    emp.setTeamFilterMenuOpen(true)
                                }}
                                onFocus={() => emp.setTeamFilterMenuOpen(true)}
                                onKeyDown={(event) => {
                                    if (event.key === "Enter") {
                                        event.preventDefault()
                                        emp.commitTypedTeamSelection()
                                    }
                                    if (event.key === "Escape") {
                                        emp.setTeamFilterMenuOpen(false)
                                        emp.setTeamFilterSearchTerm("")
                                    }
                                }}
                                placeholder={emp.teamFilter}
                                aria-label="Search and choose team"
                            />
                            <button
                                className="inline-flex items-center justify-center rounded-[6px] p-0.5 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                                onClick={() => emp.setTeamFilterMenuOpen((v) => !v)}
                                type="button"
                                aria-label="Toggle team options"
                            >
                                <ChevronDown
                                    size={14}
                                    className={`transition ${emp.isTeamFilterMenuOpen ? "rotate-180" : ""}`}
                                />
                            </button>
                        </div>

                        {emp.isTeamFilterMenuOpen && (
                            <div className="filter-menu">
                                {emp.filteredTeamFilterOptions.map((name) => {
                                    const isActive = emp.teamFilter === name
                                    return (
                                        <button
                                            key={name}
                                            className={`filter-menu-item ${isActive ? "filter-menu-item-active" : ""}`}
                                            onClick={() => emp.selectTeamFilter(name)}
                                            type="button"
                                        >
                                            <span className="truncate">{name}</span>
                                            {isActive ? <Check size={14} /> : null}
                                        </button>
                                    )
                                })}
                                {emp.filteredTeamFilterOptions.length === 0 && (
                                    <div className="px-3 py-2 text-xs text-[var(--text-secondary)]">No team found</div>
                                )}
                            </div>
                        )}
                    </div>

                    <button
                        className="text-sm font-semibold text-[var(--primary)] transition hover:opacity-80"
                        onClick={emp.clearFilters}
                        type="button"
                    >
                        Clear Filters
                    </button>

                    <button
                        className="inline-flex h-8 items-center rounded-[8px] border border-emerald-500/70 bg-emerald-500/10 px-3 text-sm font-bold text-emerald-300 shadow-[0_0_0_1px_rgba(16,185,129,0.2)] transition hover:bg-emerald-500/20 disabled:cursor-not-allowed disabled:opacity-60"
                        onClick={() => void emp.updateEmployeeListFromMssql()}
                        type="button"
                        disabled={emp.isUpdatingEmployeeListFromMssql}
                        title="Update Employee list from MSSQL"
                    >
                        {emp.isUpdatingEmployeeListFromMssql ? (
                            <span className="inline-flex items-center gap-2">
                                <LoaderCircle className="animate-spin" size={14} />
                                updating
                            </span>
                        ) : (
                            "update"
                        )}
                    </button>

                    {emp.mssqlUpdateMessage && (
                        <span
                            className={`max-w-[520px] break-words text-xs leading-snug ${
                                emp.mssqlUpdateStatus === "success"
                                    ? "text-emerald-300"
                                    : "text-red-300"
                            }`}
                            title={emp.mssqlUpdateMessage}
                            aria-live="polite"
                        >
                            {emp.mssqlUpdateMessage}
                        </span>
                    )}
                </div>
            </div>

            {emp.mssqlSyncPreview && (
                <div className="mx-4 mt-4 rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 md:mx-8">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                        <div>
                            <h2 className="text-sm font-bold">EE List sync preview</h2>
                            <p className="mt-1 text-xs text-[var(--text-secondary)]">
                                Query B includes non-resigned Staff with an AspNetUsers account. Missing rows are report-only; nothing is written until you approve.
                            </p>
                            <p className="mt-1 text-xs text-[var(--text-secondary)]">Previewed {new Date(emp.mssqlSyncPreview.createdAt).toLocaleString()}</p>
                        </div>
                        <div className="flex gap-2">
                            <button
                                className="rounded-md border border-[var(--border)] px-3 py-1.5 text-xs hover:bg-[var(--surface-hover)] disabled:opacity-50"
                                onClick={emp.skipEmployeeListMssqlSync}
                                type="button"
                                disabled={emp.isApplyingMssqlSync}
                            >Skip</button>
                            {(() => {
                                const hasImportableChanges =
                                    emp.mssqlSyncPreview.counts.new > 0 || emp.mssqlSyncPreview.counts.changed > 0
                                const hasConflicts = emp.mssqlSyncPreview.counts.conflicts > 0
                                const canApprove = hasImportableChanges && !hasConflicts

                                return (
                                    <button
                                        className="rounded-md bg-emerald-600 px-3 py-1.5 text-xs font-bold text-white disabled:cursor-not-allowed disabled:opacity-50"
                                        onClick={() => void emp.applyEmployeeListMssqlSync()}
                                        type="button"
                                        disabled={emp.isApplyingMssqlSync || !canApprove}
                                        title={
                                            hasConflicts
                                                ? "Resolve Staff Code group conflicts before approving"
                                                : !hasImportableChanges
                                                  ? "No new or changed records to approve"
                                                  : undefined
                                        }
                                    >{emp.isApplyingMssqlSync ? "Applying…" : "Approve"}</button>
                                )
                            })()}
                        </div>
                    </div>
                    <div className="mt-3 flex flex-wrap gap-3 text-xs">
                        <span>New <b>{emp.mssqlSyncPreview.counts.new}</b></span>
                        <span>Changed <b>{emp.mssqlSyncPreview.counts.changed}</b></span>
                        <span>Unchanged <b>{emp.mssqlSyncPreview.counts.unchanged}</b></span>
                        <span>Missing <b>{emp.mssqlSyncPreview.counts.missing}</b></span>
                        <span>Conflicts <b>{emp.mssqlSyncPreview.counts.conflicts}</b></span>
                    </div>
                    {emp.mssqlSyncPreview.conflicts.length > 0 && (
                        <p className="mt-2 text-xs text-red-300">{emp.mssqlSyncPreview.conflicts.map((item) => `${item.code}: ${item.reason}`).join("; ")}</p>
                    )}
                    {(() => {
                        const diffRows = emp.mssqlSyncPreview.rows.filter((row) => row.status !== "unchanged")
                        const unchangedRows = emp.mssqlSyncPreview.rows.filter((row) => row.status === "unchanged")

                        const getStatusBadge = (status: string) => {
                            switch (status) {
                                case "new":
                                    return <span className="rounded bg-emerald-500/20 px-2 py-0.5 text-[10px] font-bold uppercase text-emerald-400">New</span>
                                case "changed":
                                    return <span className="rounded bg-sky-500/20 px-2 py-0.5 text-[10px] font-bold uppercase text-sky-400">Changed</span>
                                case "missing":
                                    return <span className="rounded bg-amber-500/20 px-2 py-0.5 text-[10px] font-bold uppercase text-amber-400">Missing (Report-only)</span>
                                case "conflict":
                                    return <span className="rounded bg-rose-500/20 px-2 py-0.5 text-[10px] font-bold uppercase text-rose-400">Conflict</span>
                                default:
                                    return <span className="rounded bg-gray-500/20 px-2 py-0.5 text-[10px] font-medium text-gray-400">{status}</span>
                            }
                        }

                        const getRowHighlight = (status: string) => {
                            switch (status) {
                                case "new":
                                    return "bg-emerald-500/5 hover:bg-emerald-500/10"
                                case "changed":
                                    return "bg-sky-500/5 hover:bg-sky-500/10"
                                case "missing":
                                    return "bg-amber-500/5 hover:bg-amber-500/10"
                                case "conflict":
                                    return "bg-rose-500/5 hover:bg-rose-500/10"
                                default:
                                    return ""
                            }
                        }

                        return (
                            <div className="mt-3 space-y-3">
                                {diffRows.length > 0 ? (
                                    <div className="max-h-64 overflow-auto rounded-md border border-[var(--border)]">
                                        <table className="w-full text-left text-xs">
                                            <thead className="sticky top-0 bg-[var(--surface)] text-[var(--text-secondary)] shadow-sm">
                                                <tr>
                                                    <th className="px-3 py-2">Staff Code</th>
                                                    <th className="px-3 py-2">Result</th>
                                                    <th className="px-3 py-2">Local Name</th>
                                                    <th className="px-3 py-2">Source Name</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {diffRows.map((row, index) => (
                                                    <tr key={`diff-${row.code}-${row.status}-${index}`} className={`border-t border-[var(--border)] transition ${getRowHighlight(row.status)}`}>
                                                        <td className="px-3 py-1.5 font-mono font-medium">{row.code}</td>
                                                        <td className="px-3 py-1.5">{getStatusBadge(row.status)}</td>
                                                        <td className="px-3 py-1.5">{row.localName ?? "—"}</td>
                                                        <td className="px-3 py-1.5">{row.sourceName ?? "—"}</td>
                                                    </tr>
                                                ))}
                                            </tbody>
                                        </table>
                                    </div>
                                ) : (
                                    <div className="rounded-md border border-[var(--border)] bg-[var(--surface-hover)]/20 px-3 py-2 text-xs text-[var(--text-secondary)]">
                                        No diff rows detected. All employee records match HRMS.
                                    </div>
                                )}

                                {unchangedRows.length > 0 && (
                                    <details className="rounded-md border border-[var(--border)] bg-[var(--surface-hover)]/20 text-xs">
                                        <summary className="cursor-pointer px-3 py-2 font-medium text-[var(--text-secondary)] select-none hover:text-[var(--text-primary)]">
                                            Unchanged rows ({unchangedRows.length}) — Click to expand
                                        </summary>
                                        <div className="max-h-48 overflow-auto border-t border-[var(--border)]">
                                            <table className="w-full text-left text-xs">
                                                <tbody>
                                                    {unchangedRows.map((row, index) => (
                                                        <tr key={`unchanged-${row.code}-${index}`} className="border-b border-[var(--border)]/30 hover:bg-[var(--surface-hover)]/40">
                                                            <td className="px-3 py-1 font-mono text-[var(--text-secondary)]">{row.code}</td>
                                                            <td className="px-3 py-1 text-[var(--text-secondary)]">Unchanged</td>
                                                            <td className="px-3 py-1 text-[var(--text-secondary)]">{row.localName ?? "—"}</td>
                                                            <td className="px-3 py-1 text-[var(--text-secondary)]">{row.sourceName ?? "—"}</td>
                                                        </tr>
                                                    ))}
                                                </tbody>
                                            </table>
                                        </div>
                                    </details>
                                )}
                            </div>
                        )
                    })()}
                </div>
            )}

            {/* Table area */}
            <div className="px-4 py-6 md:px-8">
                <div className="overflow-hidden rounded-[12px] border border-[var(--border)] bg-[var(--surface)] shadow-[0_4px_6px_-1px_rgba(0,0,0,0.1)]">
                    {/* Desktop table */}
                    <div className="hidden max-h-[calc(100vh-350px)] overflow-auto xl:block">
                        <table className="min-w-[1500px] text-left text-sm">
                            <thead className="table-head text-xs uppercase tracking-[0.04em] text-[var(--text-primary)]">
                                <tr>
                                    {col.visibleColumns.map((column) => (
                                        <th
                                            key={column.key}
                                            className="relative px-3 py-3"
                                            style={{
                                                width: `${col.resolvedColumnWidths[column.key] ?? 170}px`,
                                                minWidth: `${col.resolvedColumnWidths[column.key] ?? 170}px`,
                                            }}
                                        >
                                            <div className="flex items-center justify-between gap-2">
                                                <div className="inline-flex items-center gap-2">
                                                    {column.key === firstVisibleColumnKey && (
                                                        <button
                                                            className="inline-flex items-center justify-center rounded-[8px] border border-[var(--border)] p-1.5 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                                                            onClick={() => col.setColumnsDrawerOpen(true)}
                                                            type="button"
                                                            aria-label="Open column settings"
                                                            title="Column Settings"
                                                        >
                                                            <Settings size={16} />
                                                        </button>
                                                    )}
                                                    {edit.isColumnSortable(column.key) ? (
                                                        <button
                                                            className="inline-flex items-center gap-1 rounded-[6px] px-1 py-0.5 text-left text-inherit transition hover:bg-[var(--surface-hover)]"
                                                            onClick={() => emp.toggleColumnSort(column.key, edit.isColumnSortable(column.key))}
                                                            type="button"
                                                            title={`Sort by ${column.label}`}
                                                        >
                                                            <span>{column.label}</span>
                                                            {emp.employeeSort?.key === column.key && emp.employeeSort.direction === "asc" ? (
                                                                <ChevronDown size={12} className="rotate-180 text-[var(--primary)]" />
                                                            ) : emp.employeeSort?.key === column.key && emp.employeeSort.direction === "desc" ? (
                                                                <ChevronDown size={12} className="text-[var(--primary)]" />
                                                            ) : (
                                                                <ArrowUpDown size={11} className="text-[var(--text-secondary)]/80" />
                                                            )}
                                                        </button>
                                                    ) : (
                                                        <span>{column.label}</span>
                                                    )}
                                                </div>
                                                <button
                                                    className="column-resize-handle"
                                                    onMouseDown={(event) => col.startColumnResize(event, column.key)}
                                                    type="button"
                                                    aria-label={`Resize ${column.label} column`}
                                                />
                                            </div>
                                        </th>
                                    ))}
                                </tr>
                            </thead>
                            <tbody>
                                {emp.employees.map((employee, index) => {
                                    const isMoveSelected = edit.selectedMoveEmployeeIdSet.has(employee.id)
                                    const isDuplicate = duplicateEmpIds.has(employee.id)
                                    return (
                                        <tr
                                            key={employee.id}
                                            className={`table-row border-t border-[var(--border)] transition ${isMoveSelected ? "bg-[var(--primary)]/8" : "hover:bg-[var(--surface-hover)]"
                                                }`}
                                            style={isDuplicate ? { background: "var(--highlight-bg)" } : undefined}
                                        >

                                            {col.visibleColumns.map((column) => (
                                                (() => {
                                                    const isEditable =
                                                        canEditEmployeeTable &&
                                                        edit.isTableEditMode &&
                                                        edit.isTableEditableColumn(column.key)
                                                    const isMoveSelector =
                                                        canEditEmployeeTable &&
                                                        edit.isTableEditMode &&
                                                        column.key === "employeeId"
                                                    const draftValue = edit.getDraftCellText(employee, column.key)
                                                    const isActiveCell =
                                                        edit.activeTableEditCell?.employeeId === employee.id &&
                                                        edit.activeTableEditCell?.columnKey === column.key
                                                    const rawCellValue = edit.readCellValue(employee, column.key, index, draftValue, formatDate)
                                                    const isSearchMatchedCell =
                                                        typeof rawCellValue === "string" &&
                                                        hasSearchMatch(rawCellValue, emp.searchTerm) &&
                                                        !isDuplicate &&
                                                        !isMoveSelected &&
                                                        !isActiveCell

                                                    return (
                                                        <td
                                                            key={`${employee.id}-${column.key}`}
                                                            className={`px-3 py-3 ${column.key === "employeeId"
                                                                ? "font-semibold text-[var(--primary)]"
                                                                : column.key === "fullName"
                                                                    ? "table-cell-full-name font-medium"
                                                                    : "table-cell-body"
                                                                } ${isSearchMatchedCell ? "search-highlight-cell" : ""}`}
                                                            style={{
                                                                width: `${col.resolvedColumnWidths[column.key] ?? 170}px`,
                                                                minWidth: `${col.resolvedColumnWidths[column.key] ?? 170}px`,
                                                            }}
                                                            onClick={() => edit.startTableCellEdit(employee, column.key)}
                                                            onDoubleClick={() => edit.startTableCellEdit(employee, column.key)}
                                                        >
                                                            {(() => {

                                                                if (isMoveSelector) {
                                                                    return (
                                                                        <button
                                                                            className={`inline-flex items-center gap-2 rounded-[6px] px-1.5 py-1 text-left text-sm transition ${isMoveSelected
                                                                                ? "bg-[var(--primary)]/20 text-[var(--primary)]"
                                                                                : "text-[var(--primary)] hover:bg-[var(--surface-hover)]"
                                                                                }`}
                                                                            onClick={() => edit.toggleMoveEmployeeSelection(employee.id)}
                                                                            type="button"
                                                                            title="Select this EE ID to move row to another list"
                                                                        >
                                                                            <span
                                                                                className={`inline-flex h-4 w-4 items-center justify-center rounded-[4px] border ${isMoveSelected
                                                                                    ? "border-[var(--primary)] bg-[var(--primary)]/25"
                                                                                    : "border-[var(--border)]"
                                                                                    }`}
                                                                            >
                                                                                {isMoveSelected ? <Check size={11} /> : null}
                                                                            </span>
                                                                            <span>{formatEmployeeIdForDisplay(employee.employeeId)}</span>
                                                                        </button>
                                                                    )
                                                                }

                                                                if (isEditable && isActiveCell) {
                                                                    return (
                                                                        <input
                                                                            autoFocus
                                                                            className="form-input h-8 px-2 py-1 text-xs"
                                                                            type={DATE_COLUMN_KEYS.has(column.key) ? "date" : "text"}
                                                                            value={edit.getEditableCellText(employee, column.key)}
                                                                            onChange={(event) =>
                                                                                edit.setDraftCellText(employee, column.key, event.target.value)
                                                                            }
                                                                            onBlur={() => edit.setActiveTableEditCell(null)}
                                                                            onKeyDown={(event) => {
                                                                                if (event.key === "Enter") {
                                                                                    event.currentTarget.blur()
                                                                                }
                                                                                if (event.key === "Escape") {
                                                                                    edit.setDraftCellText(
                                                                                        employee,
                                                                                        column.key,
                                                                                        edit.readRawCellText(employee, column.key),
                                                                                    )
                                                                                    edit.setActiveTableEditCell(null)
                                                                                }
                                                                            }}
                                                                        />
                                                                    )
                                                                }

                                                                if (!isEditable && column.dataType === "email") {
                                                                    const emailDisplayValue =
                                                                        typeof rawCellValue === "string" && hasSearchMatch(rawCellValue, emp.searchTerm)
                                                                            ? <HighlightText text={rawCellValue} query={emp.searchTerm} />
                                                                            : rawCellValue
                                                                    return (
                                                                        <EmployeeCellContent
                                                                            column={column}
                                                                            rawValue={rawCellValue}
                                                                            displayValue={emailDisplayValue}
                                                                            isEditable={false}
                                                                            isActiveCell={false}
                                                                            editor={null}
                                                                        />
                                                                    )
                                                                }

                                                                const displayCellValue =
                                                                    column.key === "employeeId" && typeof rawCellValue === "string"
                                                                        ? formatEmployeeIdForDisplay(rawCellValue)
                                                                        : rawCellValue

                                                                return (
                                                                    <span
                                                                        className={`${isEditable ? "cursor-cell select-none" : ""} ${
                                                                            column.key === "computerName" ? "whitespace-pre-line" : ""
                                                                        }`}
                                                                    >
                                                                        {typeof displayCellValue === "string" && hasSearchMatch(displayCellValue, emp.searchTerm) ? (
                                                                            <HighlightText text={displayCellValue} query={emp.searchTerm} />
                                                                        ) : displayCellValue}
                                                                        {column.key === "fullName" && employee.isFloating && (
                                                                            <span className="ml-2 inline-flex items-center rounded-full border border-indigo-500/30 bg-indigo-500/15 px-2 py-0.5 text-[10px] font-medium text-indigo-400">
                                                                                Floating
                                                                            </span>
                                                                        )}
                                                                        {column.key === "fullName" && employee.lifecycleStatus === "offer_rejected" && (
                                                                            <span className="ml-2 inline-flex items-center rounded-full border border-rose-500/30 bg-rose-500/15 px-2 py-0.5 text-[10px] font-medium text-rose-400">
                                                                                Offer Rejected
                                                                            </span>
                                                                        )}
                                                                        {column.key === "fullName" && employee.lifecycleStatus === "rehired" && (
                                                                            <span className="ml-2 inline-flex items-center rounded-full border border-emerald-500/30 bg-emerald-500/15 px-2 py-0.5 text-[10px] font-medium text-emerald-400">
                                                                                Rehired
                                                                            </span>
                                                                        )}
                                                                    </span>
                                                                )
                                                            })()}
                                                        </td>
                                                    )
                                                })()
                                            ))}
                                        </tr>
                                    )
                                })}
                            </tbody>
                        </table>
                    </div>

                    {/* Mobile cards */}
                    <div className="space-y-3 p-4 xl:hidden">
                        {emp.employees.map((employee, index) => (
                            <article
                                key={employee.id}
                                className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/35 p-4"
                            >
                                <div className="flex items-start justify-between gap-3">
                                    <div>
                                        <div className="flex flex-wrap items-center gap-1.5">
                                            <span className="font-semibold text-[var(--text-primary)]">{employee.fullName}</span>
                                            {employee.isFloating && (
                                                <span className="rounded-full border border-indigo-500/30 bg-indigo-500/15 px-1.5 py-0.5 text-[10px] text-indigo-400">
                                                    Floating
                                                </span>
                                            )}
                                            {employee.lifecycleStatus === "offer_rejected" && (
                                                <span className="rounded-full border border-rose-500/30 bg-rose-500/15 px-1.5 py-0.5 text-[10px] text-rose-400">
                                                    Offer Rejected
                                                </span>
                                            )}
                                            {employee.lifecycleStatus === "rehired" && (
                                                <span className="rounded-full border border-emerald-500/30 bg-emerald-500/15 px-1.5 py-0.5 text-[10px] text-emerald-400">
                                                    Rehired
                                                </span>
                                            )}
                                        </div>
                                        <div className="text-xs text-[var(--text-secondary)]">{formatEmployeeIdForDisplay(employee.employeeId)}</div>
                                    </div>
                                    <span className="text-xs text-[var(--text-secondary)]">
                                        #{(emp.currentPage - 1) * emp.rowsPerPage + index + 1}
                                    </span>
                                </div>
                                <div className="mt-3 space-y-1 text-xs text-[var(--text-secondary)]">
                                    <div>{employee.teamName ?? "-"}</div>
                                    <div>{employee.project ?? "-"}</div>
                                    <div>{employee.email ?? "-"}</div>
                                </div>
                            </article>
                        ))}
                    </div>
                </div>

                {/* Pagination + Edit bar */}
                <div className="mt-3 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] px-4 py-3">
                    <div className="flex flex-wrap items-center gap-3">
                        <div className="text-[var(--text-secondary)]">
                            Showing{" "}
                            <span className="text-[var(--text-primary)]">{emp.employees.length}</span> of{" "}
                            <span className="text-[var(--text-primary)]">{emp.totalEmployees}</span> employees
                        </div>

                        <div className="flex items-center gap-2">
                            <button
                                className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-sm font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] disabled:opacity-50"
                                onClick={edit.handleToggleTableEditMode}
                                type="button"
                                disabled={!canEditEmployeeTable || edit.isSavingTableEdits}
                                title={canEditEmployeeTable ? "Enable row edit mode" : "Login required"}
                            >
                                {edit.isTableEditMode ? "Cancel Edit" : "Edit"}
                            </button>
                            <button
                                className="rounded-[8px] bg-[var(--primary)] px-3 py-1.5 text-sm font-semibold text-[#00131c] transition hover:brightness-110 disabled:opacity-50"
                                onClick={() => void edit.handleSaveTableEdits()}
                                type="button"
                                disabled={
                                    !canEditEmployeeTable ||
                                    !edit.isTableEditMode ||
                                    !edit.hasPendingTableEdits ||
                                    edit.isSavingTableEdits ||
                                    edit.isMovingEmployees
                                }
                                title={canEditEmployeeTable ? "Save edited rows" : "Login required"}
                            >
                                {edit.isSavingTableEdits ? "Saving..." : "Save"}
                            </button>
                            {/* ── Check Duplicates ── */}
                            <button
                                className={`inline-flex items-center gap-1.5 rounded-[8px] border px-3 py-1.5 text-sm font-medium transition disabled:opacity-50 ${duplicateEmpIds.size > 0
                                    ? "border-yellow-400/60 bg-yellow-400/15 text-yellow-300 hover:bg-yellow-400/25"
                                    : "border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                                    }`}
                                onClick={() => {
                                    if (duplicateEmpIds.size > 0) {
                                        clearDuplicates()
                                    } else {
                                        void handleCheckDuplicates()
                                    }
                                }}
                                type="button"
                                disabled={isCheckingDuplicates}
                                title={duplicateEmpIds.size > 0 ? `${duplicateEmpIds.size} employees with duplicate computer values — click to clear` : "Check Computer(1) & Computer(2) for duplicate values across all employees"}
                            >
                                {isCheckingDuplicates ? (
                                    <LoaderCircle className="animate-spin" size={14} />
                                ) : duplicateEmpIds.size > 0 ? (
                                    <AlertTriangle size={14} />
                                ) : null}
                                {isCheckingDuplicates
                                    ? "Checking..."
                                    : duplicateEmpIds.size > 0
                                        ? `${duplicateEmpIds.size} Duplicates`
                                        : "Check"}
                            </button>

                            {edit.isTableEditMode && (
                                <>
                                    <select
                                        className="rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1.5 text-sm text-[var(--text-primary)] disabled:opacity-50"
                                        value={edit.moveTargetGroup}
                                        onChange={(event) =>
                                            edit.setMoveTargetGroup(event.target.value)
                                        }
                                        disabled={!canEditEmployeeTable || edit.isMovingEmployees || edit.isSavingTableEdits}
                                        title="Target list or action for selected EE IDs"
                                    >
                                        {emp.staffGroupFilter === "onboarding" && (
                                            <>
                                                <option value="employee_list">Move to Employee List (Active)</option>
                                                <option value="reject_offer">Reject Offer (Remain in Onboarding)</option>
                                                <option value="internal_movement">Move to Internal Movement</option>
                                                <option value="offboarding">Move to Offboarding</option>
                                            </>
                                        )}
                                        {emp.staffGroupFilter === "offboarding" && (
                                            <>
                                                <option value="employee_list">Rehire to Employee List</option>
                                                <option value="internal_movement">Move to Internal Movement</option>
                                                <option value="onboarding">Move to Onboarding</option>
                                            </>
                                        )}
                                        {emp.staffGroupFilter === "employee_list" && (
                                            <>
                                                <option value="set_floating">Mark as Floating</option>
                                                <option value="clear_floating">Clear Floating</option>
                                                <option value="internal_movement">Move to Internal Movement</option>
                                                <option value="offboarding">Offboard (Resigned)</option>
                                                <option value="onboarding">Move to Onboarding</option>
                                            </>
                                        )}
                                        {emp.staffGroupFilter === "internal_movement" && (
                                            <>
                                                <option value="employee_list">Complete Movement (Employee List)</option>
                                                <option value="set_floating">Mark as Floating</option>
                                                <option value="offboarding">Offboard (Resigned)</option>
                                                <option value="onboarding">Move to Onboarding</option>
                                            </>
                                        )}
                                    </select>
                                    <button
                                        className="rounded-[8px] border border-[var(--primary)]/50 px-3 py-1.5 text-sm font-medium text-[var(--primary)] transition hover:bg-[var(--primary)]/10 disabled:opacity-50"
                                        onClick={() => void edit.handleMoveSelectedEmployees()}
                                        type="button"
                                        disabled={
                                            !canEditEmployeeTable ||
                                            !edit.canMoveSelectedRows ||
                                            edit.isMovingEmployees ||
                                            edit.isSavingTableEdits
                                        }
                                        title="Execute selected HR transition"
                                    >
                                        {edit.isMovingEmployees
                                            ? "Processing..."
                                            : edit.moveTargetGroup === "reject_offer"
                                            ? "Reject Offer"
                                            : edit.moveTargetGroup === "employee_list" && emp.staffGroupFilter === "offboarding"
                                            ? "Rehire"
                                            : edit.moveTargetGroup === "set_floating"
                                            ? "Set Floating"
                                            : edit.moveTargetGroup === "clear_floating"
                                            ? "Clear Floating"
                                            : "Move"}
                                    </button>
                                    {emp.staffGroupFilter === "offboarding" && (
                                        <button
                                            className="inline-flex items-center gap-1 rounded-[8px] border border-red-500/60 px-3 py-1.5 text-sm font-medium text-red-400 transition hover:bg-red-500/10 disabled:opacity-50"
                                            onClick={() => void edit.handleDeleteSelectedEmployee()}
                                            type="button"
                                            disabled={
                                                !canEditEmployeeTable ||
                                                !edit.canDeleteSelectedEmployee ||
                                                edit.isDeletingSelectedEmployee
                                            }
                                            title={
                                                edit.selectedMoveEmployeeIds.length === 1
                                                    ? "Delete selected Offboarding employee"
                                                    : "Select exactly one Offboarding employee to delete."
                                            }
                                        >
                                            {edit.isDeletingSelectedEmployee ? (
                                                <LoaderCircle className="animate-spin" size={14} />
                                            ) : (
                                                <Trash2 size={14} />
                                            )}
                                            {edit.isDeletingSelectedEmployee ? "Deleting..." : "Delete"}
                                        </button>
                                    )}
                                    <button
                                        className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-sm font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] disabled:opacity-50"
                                        onClick={edit.selectCurrentPageEmployees}
                                        type="button"
                                        disabled={edit.isMovingEmployees || edit.isSavingTableEdits || edit.currentPageEmployeeIds.length === 0}
                                        title="Select or unselect all rows on this page."
                                    >
                                        Select Page
                                    </button>
                                    <button
                                        className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-sm font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] disabled:opacity-50"
                                        onClick={() => void handleSelectAllResults()}
                                        type="button"
                                        disabled={edit.isMovingEmployees || edit.isSavingTableEdits || emp.totalEmployees === 0}
                                    >
                                        Select All ({emp.totalEmployees})
                                    </button>
                                    <details
                                        className="relative"
                                        ref={unselectMenuRef}
                                        open={isUnselectMenuOpen}
                                        onToggle={(event) => event.currentTarget.open ? openUnselectMenu() : closeUnselectMenu()}
                                        onPointerMove={isUnselectMenuOpen ? resetUnselectTimer : undefined}
                                        onFocusCapture={isUnselectMenuOpen ? resetUnselectTimer : undefined}
                                        onKeyDownCapture={isUnselectMenuOpen ? resetUnselectTimer : undefined}
                                    >
                                        <summary className="cursor-pointer list-none rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-sm font-medium text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]">
                                            Unselect {isUnselectMenuOpen ? "▲" : "▼"}
                                        </summary>
                                        <div className="absolute bottom-full right-0 z-20 mb-1 max-h-[min(60vh,240px)] min-w-max overflow-auto rounded-[8px] border border-[var(--border)] bg-[var(--surface)] p-1 shadow-lg" onPointerMove={resetUnselectTimer} onFocusCapture={resetUnselectTimer} onKeyDownCapture={resetUnselectTimer}>
                                            <button className="block w-full rounded px-3 py-1.5 text-left text-sm hover:bg-[var(--surface-hover)]" onClick={() => { edit.unselectCurrentPageEmployees(); closeUnselectMenu() }} type="button">
                                                Unselect Page
                                            </button>
                                            <button className="block w-full rounded px-3 py-1.5 text-left text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50" onClick={() => { edit.clearMoveSelection(); closeUnselectMenu() }} type="button" disabled={edit.selectedMoveEmployeeIds.length === 0}>
                                                Clear All ({edit.selectedMoveEmployeeIds.length})
                                            </button>
                                        </div>
                                    </details>
                                    {isAdminAccount && (
                                        <details
                                            className="relative"
                                            ref={exportMenuRef}
                                            open={isExportMenuOpen}
                                            onToggle={(event) => event.currentTarget.open ? openExportMenu() : closeExportMenu()}
                                            onPointerMove={isExportMenuOpen ? resetExportTimer : undefined}
                                            onFocusCapture={isExportMenuOpen ? resetExportTimer : undefined}
                                            onKeyDownCapture={isExportMenuOpen ? resetExportTimer : undefined}
                                        >
                                            <summary className="cursor-pointer list-none rounded-[8px] border border-[var(--primary)]/50 px-3 py-1.5 text-sm font-medium text-[var(--primary)] hover:bg-[var(--primary)]/10">
                                                Export ({edit.selectedMoveEmployeeIds.length}) {isExportMenuOpen ? "▲" : "▼"}
                                            </summary>
                                            <div className="absolute bottom-full right-0 z-20 mb-1 max-h-[min(60vh,240px)] min-w-max overflow-auto rounded-[8px] border border-[var(--border)] bg-[var(--surface)] p-1 shadow-lg" onPointerMove={resetExportTimer} onFocusCapture={resetExportTimer} onKeyDownCapture={resetExportTimer}>
                                                <button className="block w-full rounded px-3 py-1.5 text-left text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50" onClick={() => { closeExportMenu(); void handleExport("csv") }} type="button" disabled={edit.selectedMoveEmployeeIds.length === 0 || isExporting}>
                                                    Export as CSV
                                                </button>
                                                <button className="block w-full rounded px-3 py-1.5 text-left text-sm hover:bg-[var(--surface-hover)] disabled:opacity-50" onClick={() => { closeExportMenu(); void handleExport("xlsx") }} type="button" disabled={edit.selectedMoveEmployeeIds.length === 0 || isExporting}>
                                                    Export as Excel (.xlsx)
                                                </button>
                                            </div>
                                        </details>
                                    )}
                                    <span className="text-xs text-[var(--text-secondary)]">
                                        Edited:{" "}
                                        <span className="font-semibold text-[var(--text-primary)]">{edit.editedRowCount}</span>
                                    </span>
                                    <span className="text-xs text-[var(--text-secondary)]">
                                        Selected:{" "}
                                        <span className="font-semibold text-[var(--text-primary)]">
                                            {edit.selectedMoveEmployeeIds.length}
                                        </span>{" "}
                                        (this page:{" "}
                                        <span className="font-semibold text-[var(--text-primary)]">
                                            {edit.selectedOnCurrentPageCount}
                                        </span>
                                        )
                                    </span>
                                    <span className="text-xs text-[var(--text-secondary)]">
                                        Click an editable cell to update it.{" "}
                                        <span className="text-[var(--text-primary)]">
                                            {canEditEmployeeComputerName
                                                ? "Computer Name is editable for Super Admin."
                                                : "Computer Name is editable for Super Admin only."}
                                        </span>
                                    </span>
                                </>
                            )}
                        </div>

                        <div className="ml-auto flex items-center gap-2">
                            {(emp.isLoadingEmployees || col.isLoadingColumns) && (
                                <LoaderCircle className="animate-spin text-[var(--text-secondary)]" size={16} />
                            )}
                            <label className="text-xs text-[var(--text-secondary)]">Rows:</label>
                            <select
                                className="rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1 text-sm"
                                value={emp.rowsPerPage}
                                onChange={(event) => emp.setRowsPerPage(Number(event.target.value))}
                            >
                                <option value={15}>15</option>
                                <option value={30}>30</option>
                                <option value={50}>50</option>
                                <option value={100}>100</option>
                                <option value={500}>500</option>
                            </select>

                            <div className="ml-3 flex items-center gap-1">
                                <button
                                    className="pager-button"
                                    disabled={emp.currentPage <= 1}
                                    onClick={() => emp.setCurrentPage((v) => Math.max(1, v - 1))}
                                    type="button"
                                >
                                    Previous
                                </button>
                                <span className="px-2 text-sm text-[var(--text-secondary)]">
                                    {emp.currentPage}/{emp.totalPages}
                                </span>
                                <button
                                    className="pager-button"
                                    disabled={emp.currentPage >= emp.totalPages}
                                    onClick={() => emp.setCurrentPage((v) => Math.min(emp.totalPages, v + 1))}
                                    type="button"
                                >
                                    Next
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    )
}
