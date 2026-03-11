import { GripVertical } from "lucide-react"
import { Drawer } from "../../components/Drawer"
import type { ColumnState } from "./useColumnState"

type ColumnsDrawerProps = {
    columnState: ColumnState
    activeAccountName: string
    activeUserScope: string
    activeTableLabel: string
    triggerReload: () => void
}

export function ColumnsDrawer({
    columnState,
    activeAccountName,
    activeUserScope,
    activeTableLabel,
    triggerReload,
}: ColumnsDrawerProps) {
    const col = columnState

    return (
        <Drawer
            open={col.isColumnsDrawerOpen}
            onClose={() => col.setColumnsDrawerOpen(false)}
            title="Column Preferences"
            widthClass="w-[460px]"
        >
            <div className="space-y-4">
                <p className="text-sm text-[var(--text-secondary)]">
                    Toggle visibility and drag the handle to reorder columns. Changes save automatically for this table profile, and new imported fields will appear here by themselves.
                </p>
                <div className="space-y-1 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 px-3 py-2 text-xs text-[var(--text-secondary)]">
                    <div>
                        Active profile: <span className="font-semibold text-[var(--text-primary)]">{activeAccountName}</span> ({activeUserScope})
                    </div>
                    <div>
                        Current table: <span className="font-semibold text-[var(--text-primary)]">{activeTableLabel}</span>
                    </div>
                </div>

                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 p-3">
                    <div className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">Add Dynamic Column</div>
                    <div className="mt-2 flex items-center gap-2">
                        <input
                            className="form-input"
                            placeholder="New column title..."
                            value={col.newColumnTitle}
                            onChange={(event) => col.setNewColumnTitle(event.target.value)}
                        />
                        <button
                            className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold disabled:opacity-50"
                            onClick={() => void col.handleAddColumn(triggerReload)}
                            type="button"
                            disabled={col.isMutatingColumns}
                        >
                            Add
                        </button>
                    </div>
                </div>

                <input
                    className="form-input"
                    placeholder="Search columns..."
                    value={col.columnSearchTerm}
                    onChange={(event) => col.setColumnSearchTerm(event.target.value)}
                />

                <div className="text-xs text-[var(--text-secondary)]">
                    Visible columns: {col.effectiveColumnPreferences.order.length - col.effectiveColumnPreferences.hidden.length}/{col.effectiveColumnPreferences.order.length}
                </div>

                <div className="space-y-2">
                    {col.filteredColumnKeys.map((key) => {
                        const column = col.configurableColumnMap.get(key)
                        if (!column) return null

                        const visible = !col.effectiveColumnPreferences.hidden.includes(key)
                        const isDragging = col.draggingColumnKey === key
                        const dropBefore = col.dropTarget?.key === key && col.dropTarget.position === "before"
                        const dropAfter = col.dropTarget?.key === key && col.dropTarget.position === "after"

                        return (
                            <div
                                key={key}
                                data-column-key={key}
                                className={`column-pref-row relative flex items-center gap-3 rounded-[8px] border px-3 py-2 ${
                                    isDragging
                                        ? "border-[var(--primary)]/45 bg-[var(--primary)]/10 opacity-75"
                                        : "border-[var(--border)] bg-[var(--surface-hover)]/35"
                                }`}
                            >
                                {dropBefore && <span className="column-drop-indicator column-drop-indicator-top" />}
                                {dropAfter && <span className="column-drop-indicator column-drop-indicator-bottom" />}

                                <button
                                    className="column-drag-handle"
                                    type="button"
                                    onPointerDown={(event) => col.startColumnDrag(event, key)}
                                    title="Drag to reorder"
                                    aria-label={`Drag to reorder ${column.label}`}
                                >
                                    <GripVertical size={14} className="text-[var(--text-secondary)]" />
                                </button>
                                <input type="checkbox" checked={visible} onChange={() => col.toggleColumnVisibility(key)} />
                                <div className="flex-1">
                                    <div className="text-sm font-medium">{column.label}</div>
                                    <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">{column.source}</div>
                                </div>
                                <button
                                    className="rounded-[6px] border border-[var(--border)] px-2 py-1 text-xs"
                                    onClick={() => void col.handleRenameColumn(column, triggerReload)}
                                    type="button"
                                    disabled={col.isMutatingColumns}
                                >
                                    Rename
                                </button>
                                {column.source === "dynamic" && (
                                    <button
                                        className="rounded-[6px] border border-[var(--border)] px-2 py-1 text-xs text-[var(--error)]"
                                        onClick={() => void col.handleDeleteColumn(column, triggerReload)}
                                        type="button"
                                        disabled={col.isMutatingColumns}
                                    >
                                        Delete
                                    </button>
                                )}
                            </div>
                        )
                    })}
                    {col.filteredColumnKeys.length === 0 && (
                        <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 px-3 py-2 text-sm text-[var(--text-secondary)]">
                            No columns match your search.
                        </div>
                    )}
                </div>

                <div className="flex flex-wrap items-center gap-2 border-t border-[var(--border)] pt-3">
                    <button
                        className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium"
                        onClick={col.resetColumnPreferences}
                        type="button"
                    >
                        Reset Default
                    </button>
                    {col.undoColumnResetSnapshot && (
                        <button
                            className="rounded-[8px] border border-amber-400/50 px-3 py-2 text-sm font-medium text-amber-300"
                            onClick={col.undoResetColumnPreferences}
                            type="button"
                        >
                            Undo Reset
                        </button>
                    )}
                    <div className="ml-auto text-xs text-[var(--text-secondary)]">
                        {col.isAutoSavingColumnProfile
                            ? "Saving changes..."
                            : "Saved automatically for this table"}
                    </div>
                </div>
            </div>
        </Drawer>
    )
}
