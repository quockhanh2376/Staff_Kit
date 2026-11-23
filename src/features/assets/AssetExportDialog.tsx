import { useEffect, useState } from "react"
import { Drawer } from "../../components/Drawer"
import { buildCsv, buildXlsxBlob, saveExport } from "../../lib/tableExport"
import { staffApi } from "../../services/staff-api"
import type { AssetCategoryDetailRecord, AssetDashboardSerializedRecord, TeamRecord } from "../../types/staff"
import {
  ASSET_EXPORT_SCOPES,
  assetExportFileName,
  buildAssetExportSheets,
  selectAssetExportRows,
  type AssetExportScope,
} from "./assetExport"

const labelClass = "mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]"
const selectClass =
  "h-10 w-full rounded-md border border-[var(--border)] bg-[var(--bg)] px-3 text-sm text-[var(--text-primary)]"

/** Admin dialog: exports from the full serialized list, never the current page. */
export function AssetExportDialog({
  open,
  onClose,
  rows,
  filteredRows,
  currentFilterAvailable,
  categoryDetails,
}: {
  open: boolean
  onClose: () => void
  rows: AssetDashboardSerializedRecord[]
  filteredRows: AssetDashboardSerializedRecord[]
  /** The table filter only exists on the Serialized tab. */
  currentFilterAvailable: boolean
  categoryDetails: AssetCategoryDetailRecord[]
}) {
  const [pickedScope, setScope] = useState<AssetExportScope>("all")
  const scope = pickedScope === "currentFilter" && !currentFilterAvailable ? "all" : pickedScope
  const [categoryCode, setCategoryCode] = useState("")
  const [teamName, setTeamName] = useState("")
  const [format, setFormat] = useState<"xlsx" | "csv">("xlsx")
  const [teams, setTeams] = useState<TeamRecord[]>([])
  const [isSaving, setIsSaving] = useState(false)
  const [savedPath, setSavedPath] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    setSavedPath(null)
    setError(null)
    let active = true
    staffApi
      .listTeams()
      .then((list) => active && setTeams(list))
      .catch(() => active && setTeams([]))
    return () => {
      active = false
    }
  }, [open])

  const handleExport = async () => {
    setIsSaving(true)
    setSavedPath(null)
    setError(null)
    try {
      const selected = selectAssetExportRows(rows, filteredRows, {
        scope,
        categoryCode: categoryCode || null,
        teamName: teamName || null,
        teams,
      })
      const sheets = buildAssetExportSheets(selected, scope)
      const blob =
        format === "csv"
          ? new Blob([buildCsv(sheets[1])], { type: "text/csv;charset=utf-8" })
          : await buildXlsxBlob(sheets)
      const path = await saveExport(blob, assetExportFileName(scope, new Date(), format), format)
      // null: the save dialog was cancelled; stay open so IT can try again.
      if (path !== null) setSavedPath(path)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsSaving(false)
    }
  }

  const categories = categoryDetails.filter((detail) => detail.trackingMode === "serialized")

  return (
    <Drawer open={open} onClose={onClose} title="Export assets" widthClass="w-[460px]">
      <div className="space-y-5">
        <fieldset>
          <legend className={labelClass}>Scope</legend>
          <div className="space-y-1.5">
            {ASSET_EXPORT_SCOPES.map((item) => (
              <label key={item.value} className="flex items-center gap-2 text-sm text-[var(--text-primary)]">
                <input
                  type="radio"
                  name="asset-export-scope"
                  checked={scope === item.value}
                  disabled={item.value === "currentFilter" && !currentFilterAvailable}
                  onChange={() => setScope(item.value)}
                />
                {item.label}
              </label>
            ))}
          </div>
        </fieldset>
        <label className="block">
          <span className={labelClass}>Category</span>
          <select className={selectClass} value={categoryCode} onChange={(e) => setCategoryCode(e.target.value)}>
            <option value="">All categories</option>
            {categories.map((detail) => (
              <option key={detail.categoryCode} value={detail.categoryCode}>
                {detail.categoryName}
              </option>
            ))}
          </select>
        </label>
        <label className="block">
          <span className={labelClass}>Team</span>
          <select className={selectClass} value={teamName} onChange={(e) => setTeamName(e.target.value)}>
            <option value="">All teams</option>
            {[...new Set(teams.map((team) => team.name))].map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
          <span className="mt-1 block text-xs text-[var(--text-secondary)]">
            Matches the holder team. In-stock machines have no team and are left out.
          </span>
        </label>
        <fieldset>
          <legend className={labelClass}>Format</legend>
          <div className="flex gap-4 text-sm text-[var(--text-primary)]">
            <label className="flex items-center gap-2">
              <input type="radio" name="asset-export-format" checked={format === "xlsx"} onChange={() => setFormat("xlsx")} />
              Excel
            </label>
            <label className="flex items-center gap-2">
              <input type="radio" name="asset-export-format" checked={format === "csv"} onChange={() => setFormat("csv")} />
              CSV
            </label>
          </div>
        </fieldset>
        {savedPath && (
          <div className="rounded-[10px] border border-emerald-500/25 bg-emerald-500/8 px-3 py-2 text-xs text-[var(--text-primary)]">
            Saved to {savedPath}
          </div>
        )}
        {error && (
          <div
            role="alert"
            className="rounded-[10px] border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-[var(--text-primary)]"
          >
            {error}
          </div>
        )}
        <button
          type="button"
          onClick={() => void handleExport()}
          disabled={isSaving}
          className="inline-flex h-10 items-center rounded-md border border-emerald-500/70 bg-emerald-500 px-4 text-sm font-semibold text-white transition hover:bg-emerald-400 disabled:opacity-50"
        >
          {isSaving ? "Exporting..." : "Export file"}
        </button>
      </div>
    </Drawer>
  )
}
