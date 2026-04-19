import {
    AlertTriangle,
    ArrowUpDown,
    Check,
    ChevronDown,
    LoaderCircle,
    Search,
    Settings,
    Upload,
    Users,
} from "lucide-react"
import { useState, useCallback } from "react"
import { staffApi } from "../../services/staff-api"
import type { EmployeeState } from "./useEmployeeState"
import type { TableEditState } from "./useTableEdit"
import type { ColumnState } from "../columns/useColumnState"
import type { StaffGroupKey } from "../../types/app"
import { STAFF_GROUP_BUTTONS, DATE_COLUMN_KEYS } from "../../lib/constants"
import { formatDate } from "../../lib/utils"
import { formatEmployeeIdForDisplay } from "./employeeIdDisplay"
import { collectDuplicateComputerEmployeeIds } from "./employeeTableRules"

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
    canSeedEmployeeAssets: boolean
    onOpenEmployeeAssetSeedDrawer: () => void
    selectedGroupLabel: string
    selectedGroupTotal: number
}

export function EmployeeView({
    employeeState,
    tableEdit,
    columnState,
    canEditEmployeeTable,
    canEditEmployeeComputerName,
    canSeedEmployeeAssets,
    onOpenEmployeeAssetSeedDrawer,
    selectedGroupLabel,
    selectedGroupTotal,
}: EmployeeViewProps) {
    const emp = employeeState
    const edit = tableEdit
    const col = columnState
    const firstVisibleColumnKey = col.visibleColumns[0]?.key ?? ""

    // ── Duplicate computer check ─────────────────────────────────────────────────
    const [duplicateEmpIds, setDuplicateEmpIds] = useState<Set<number>>(new Set())
    const [isCheckingDuplicates, setIsCheckingDuplicates] = useState(false)

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
                            className="absolute left-4 top-1/2 -translate-y-1/2 text-[var(--text-secondary)]"
                            size={18}
                        />
                        <input
                            className="h-10 w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] pl-11 pr-4 text-sm text-[var(--text-primary)] outline-none transition focus:border-[var(--primary)]"
                            placeholder="Search name, EE.ID, email, computer (e.g. ASWVNLAP)..."
                            value={emp.searchTerm}
                            onChange={(event) => emp.setSearchTerm(event.target.value)}
                        />
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
                </div>
            </div>

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
                                        <div className="font-semibold text-[var(--text-primary)]">{employee.fullName}</div>
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
                            {canSeedEmployeeAssets && (
                                <button
                                    className="inline-flex items-center gap-1.5 rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-sm font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                                    onClick={onOpenEmployeeAssetSeedDrawer}
                                    type="button"
                                    title="Preview serialized asset creation from the current Employee List filters"
                                >
                                    <Upload size={14} />
                                    Seed Assets
                                </button>
                            )}
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
                                            edit.setMoveTargetGroup(event.target.value as StaffGroupKey)
                                        }
                                        disabled={!canEditEmployeeTable || edit.isMovingEmployees || edit.isSavingTableEdits}
                                        title="Target list for selected EE IDs"
                                    >
                                        {STAFF_GROUP_BUTTONS.map((item) => (
                                            <option key={item.key} value={item.key}>
                                                {item.label}
                                            </option>
                                        ))}
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
                                        title="Move selected EE IDs to target list"
                                    >
                                        {edit.isMovingEmployees ? "Moving..." : "Move"}
                                    </button>
                                    <button
                                        className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-sm font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] disabled:opacity-50"
                                        onClick={edit.toggleSelectCurrentPageEmployees}
                                        type="button"
                                        disabled={
                                            !canEditEmployeeTable ||
                                            edit.isMovingEmployees ||
                                            edit.isSavingTableEdits ||
                                            edit.currentPageEmployeeIds.length === 0
                                        }
                                        title="Select or unselect all rows on this page."
                                    >
                                        {edit.isCurrentPageFullySelected ? "Unselect Page" : "Select Page"}
                                    </button>
                                    <button
                                        className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-sm font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] disabled:opacity-50"
                                        onClick={edit.clearMoveSelection}
                                        type="button"
                                        disabled={
                                            !canEditEmployeeTable ||
                                            edit.isMovingEmployees ||
                                            edit.isSavingTableEdits ||
                                            edit.selectedMoveEmployeeIds.length === 0
                                        }
                                    >
                                        Clear Selected
                                    </button>
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
