import { GripVertical } from "lucide-react"
import { Drawer } from "../../components/Drawer"
import type { ColumnState } from "./useColumnState"

type ColumnsDrawerProps = {
    columnState: ColumnState
    activeAccountName: string
    activeUserScope: string
    triggerReload: () => void
}

export function ColumnsDrawer({
    columnState,
    activeAccountName,
    activeUserScope,
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
                    Toggle visibility and drag to reorder columns. New fields from imported Excel files will appear here automatically.
                </p>
                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 px-3 py-2 text-xs text-[var(--text-secondary)]">
                    Active profile: <span className="font-semibold text-[var(--text-primary)]">{activeAccountName}</span> ({activeUserScope})
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
                        const currentIndex = col.effectiveColumnPreferences.order.indexOf(key)
                        const isFirst = currentIndex <= 0
                        const isLast = currentIndex === col.effectiveColumnPreferences.order.length - 1

                        return (
                            <div
                                key={key}
                                className="flex items-center gap-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/35 px-3 py-2"
                                draggable
                                onDragStart={() => col.setDraggingColumnKey(key)}
                                onDragOver={(event) => { event.preventDefault() }}
                                onDrop={() => {
                                    if (col.draggingColumnKey) {
                                        col.reorderColumns(col.draggingColumnKey, key)
                                    }
                                    col.setDraggingColumnKey(null)
                                }}
                                onDragEnd={() => col.setDraggingColumnKey(null)}
                            >
                                <GripVertical size={14} className="text-[var(--text-secondary)]" />
                                <input type="checkbox" checked={visible} onChange={() => col.toggleColumnVisibility(key)} />
                                <div className="flex-1">
                                    <div className="text-sm font-medium">{column.label}</div>
                                    <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">{column.source}</div>
                                </div>
                                <div className="flex items-center gap-1">
                                    <button
                                        className="rounded-[6px] border border-[var(--border)] px-2 py-1 text-xs disabled:opacity-50"
                                        onClick={() => col.moveColumnByOffset(key, -1)}
                                        type="button"
                                        disabled={isFirst}
                                        title="Move up"
                                    >
                                        Up
                                    </button>
                                    <button
                                        className="rounded-[6px] border border-[var(--border)] px-2 py-1 text-xs disabled:opacity-50"
                                        onClick={() => col.moveColumnByOffset(key, 1)}
                                        type="button"
                                        disabled={isLast}
                                        title="Move down"
                                    >
                                        Down
                                    </button>
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
                        className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                        onClick={col.handleSaveColumnProfile}
                        type="button"
                        disabled={!col.hasUnsavedColumnProfileChanges}
                    >
                        Save for {activeAccountName}
                    </button>
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
                        {col.hasUnsavedColumnProfileChanges ? "Unsaved changes" : "All changes saved"}
                    </div>
                </div>
            </div>
        </Drawer>
    )
}
