import { GripVertical, PencilLine } from "lucide-react"
import { Drawer } from "../../components/Drawer"

type ColumnDefinition<Key extends string> = {
  key: Key
  label: string
}

type ColumnStateLike<Key extends string> = {
  orderedColumns: ColumnDefinition<Key>[]
  orderedColumnKeys: Key[]
  columnLabels: Record<Key, string>
  hiddenColumns: Key[]
  filteredColumnKeys: Key[]
  drawerDraggingColumnKey: Key | null
  drawerDropTarget: { key: Key; position: "before" | "after" } | null
  isColumnsDrawerOpen: boolean
  columnSearchTerm: string
  undoResetSnapshot: unknown
  setColumnsDrawerOpen: (open: boolean) => void
  setColumnSearchTerm: (term: string) => void
  startDrawerColumnDrag: (key: Key) => void
  toggleColumnVisibility: (key: Key) => void
  renameColumnLabel: (key: Key, label: string) => void
  resetColumnPreferences: () => void
  undoResetColumnPreferences: () => void
}

type AssetDashboardColumnsDrawerProps<Key extends string> = {
  activeUserScope: string
  tableLabel: string
  columnMap: Record<Key, ColumnDefinition<Key>>
  grid: ColumnStateLike<Key>
}

export function AssetDashboardColumnsDrawer<Key extends string>({
  activeUserScope,
  tableLabel,
  columnMap,
  grid,
}: AssetDashboardColumnsDrawerProps<Key>) {
  const handleRenameColumn = (column: ColumnDefinition<Key>) => {
    const nextLabel = window.prompt("Column label", column.label)
    if (nextLabel === null) {
      return
    }
    grid.renameColumnLabel(column.key, nextLabel)
  }

  return (
    <Drawer
      open={grid.isColumnsDrawerOpen}
      onClose={() => grid.setColumnsDrawerOpen(false)}
      title="Column Preferences"
      widthClass="w-[420px]"
    >
      <div className="space-y-4 text-slate-300">
        <p className="text-sm text-slate-400">
          Toggle visible columns and drag rows to reorder them. Changes are saved automatically for this asset dashboard profile.
        </p>

        <div className="space-y-1 rounded-[8px] border border-[#283140] bg-[#111722] px-3 py-2 text-xs text-slate-400">
          <div>
            Active profile: <span className="font-semibold text-slate-100">{activeUserScope}</span>
          </div>
          <div>
            Current table: <span className="font-semibold text-slate-100">{tableLabel}</span>
          </div>
        </div>

        <input
          className="h-10 w-full rounded-[8px] border border-[#283140] bg-[#111722] px-3 text-sm text-slate-100 outline-none transition focus:border-[#00d68f]"
          placeholder="Search columns..."
          value={grid.columnSearchTerm}
          onChange={(event) => grid.setColumnSearchTerm(event.target.value)}
        />

        <div className="text-xs text-slate-400">
          Visible columns: {grid.orderedColumns.length}/{grid.orderedColumnKeys.length}
        </div>

        <div className="space-y-2">
          {grid.filteredColumnKeys.map((key) => {
            const column = { ...columnMap[key], label: grid.columnLabels[key] ?? columnMap[key].label }
            const visible = !grid.hiddenColumns.includes(key)
            const isDragging = grid.drawerDraggingColumnKey === key
            const dropBefore =
              grid.drawerDropTarget?.key === key && grid.drawerDropTarget.position === "before"
            const dropAfter =
              grid.drawerDropTarget?.key === key && grid.drawerDropTarget.position === "after"

            return (
              <div
                key={key}
                data-asset-dashboard-column-key={key}
                className={`relative flex items-center gap-3 rounded-[8px] border px-3 py-2 ${
                  isDragging
                    ? "border-[#00d68f]/50 bg-[#00d68f]/10 opacity-75"
                    : "border-[#283140] bg-[#111722]"
                }`}
              >
                {dropBefore ? (
                  <span className="absolute inset-x-3 top-0 h-px bg-[#00d68f]" />
                ) : null}
                {dropAfter ? (
                  <span className="absolute inset-x-3 bottom-0 h-px bg-[#00d68f]" />
                ) : null}

                <button
                  className="inline-flex h-8 w-8 items-center justify-center rounded-[8px] border border-[#283140] bg-[#151b26] text-slate-400 transition hover:bg-[#1b2230] hover:text-slate-200"
                  onPointerDown={() => grid.startDrawerColumnDrag(key)}
                  type="button"
                  aria-label={`Drag to reorder ${column.label}`}
                  title={`Drag to reorder ${column.label}`}
                >
                  <GripVertical size={14} />
                </button>

                <input
                  type="checkbox"
                  checked={visible}
                  onChange={() => grid.toggleColumnVisibility(key)}
                  aria-label={`Toggle ${column.label} column visibility`}
                />

                <div className="min-w-0 flex-1">
                  <div className="text-sm font-medium text-slate-100">{column.label}</div>
                  <div className="text-[11px] uppercase tracking-[0.06em] text-slate-500">
                    {column.key}
                  </div>
                </div>

                <button
                  className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-[8px] border border-[#283140] bg-[#151b26] text-slate-400 transition hover:bg-[#1b2230] hover:text-slate-100"
                  onClick={() => handleRenameColumn(column)}
                  type="button"
                  aria-label={`Rename ${column.label} column`}
                  title={`Rename ${column.label}`}
                >
                  <PencilLine size={15} />
                </button>
              </div>
            )
          })}

          {grid.filteredColumnKeys.length === 0 ? (
            <div className="rounded-[8px] border border-[#283140] bg-[#111722] px-3 py-2 text-sm text-slate-400">
              No columns match your search.
            </div>
          ) : null}
        </div>

        <div className="flex flex-wrap items-center gap-2 border-t border-[#283140] pt-3">
          <button
            className="rounded-[8px] border border-[#283140] px-3 py-2 text-sm font-medium text-slate-100 transition hover:bg-[#1b2230]"
            onClick={grid.resetColumnPreferences}
            type="button"
          >
            Reset Default
          </button>
          {grid.undoResetSnapshot ? (
            <button
              className="rounded-[8px] border border-amber-400/50 px-3 py-2 text-sm font-medium text-amber-300 transition hover:bg-amber-400/10"
              onClick={grid.undoResetColumnPreferences}
              type="button"
            >
              Undo Reset
            </button>
          ) : null}
          <div className="ml-auto text-xs text-slate-500">Saved automatically for this table</div>
        </div>
      </div>
    </Drawer>
  )
}
