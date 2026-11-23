import type { ExportCell, ExportSheet } from "../../lib/tableExport"
import type { AssetDashboardSerializedRecord, TeamRecord } from "../../types/staff"
import { CONDITION_LABELS, isInStock } from "./assetCondition"
import { formatAssetDashboardStatusLabel } from "./assetDashboardCopy"

type Row = AssetDashboardSerializedRecord

export type AssetExportScope =
  | "all"
  | "assignable"
  | "spare"
  | "new"
  | "broken"
  | "assigned"
  | "currentFilter"

export type AssetExportOptions = {
  scope: AssetExportScope
  categoryCode: string | null
  teamName: string | null
  teams: TeamRecord[]
}

export const ASSET_EXPORT_SCOPES: { value: AssetExportScope; label: string; fileLabel: string }[] = [
  { value: "assignable", label: "Assignable (New + Spare in stock)", fileLabel: "Assignable" },
  { value: "spare", label: "Spare (in stock)", fileLabel: "Spare" },
  { value: "new", label: "New (in stock)", fileLabel: "New" },
  { value: "broken", label: "Broken (all)", fileLabel: "Broken" },
  { value: "assigned", label: "Assigned to staff", fileLabel: "Assigned" },
  { value: "all", label: "All assets", fileLabel: "All" },
  { value: "currentFilter", label: "Current table filter", fileLabel: "Filtered" },
]

const isAssigned = (row: Row) => row.status === "assigned"

const SCOPE_TESTS: Record<Exclude<AssetExportScope, "currentFilter">, (row: Row) => boolean> = {
  all: () => true,
  assignable: (row) => isInStock(row) && (row.condition === "new" || row.condition === "spare"),
  spare: (row) => isInStock(row) && row.condition === "spare",
  new: (row) => isInStock(row) && row.condition === "new",
  // A repair list: broken machines wherever they are.
  broken: (row) => row.condition === "broken",
  assigned: isAssigned,
}

export function selectAssetExportRows(rows: Row[], filteredRows: Row[], options: AssetExportOptions): Row[] {
  const base = options.scope === "currentFilter" ? filteredRows : rows.filter(SCOPE_TESTS[options.scope])
  const { teamName, teams, categoryCode } = options
  // Teams are matched by name (rows carry only the name); duplicate names all count.
  const chosenIds = new Set(teams.filter((t) => t.name === teamName).map((t) => t.id))
  const inTeam = (row: Row) =>
    row.holderTeamName !== null &&
    (row.holderTeamName === teamName ||
      teams.some((t) => t.name === row.holderTeamName && t.parentId !== null && chosenIds.has(t.parentId)))
  return base.filter((row) => (!categoryCode || row.categoryCode === categoryCode) && (!teamName || inTeam(row)))
}

export function buildAssetExportSheets(rows: Row[], scope: AssetExportScope): ExportSheet[] {
  const holderHeaders = ["Holder staff ID", "Holder name", "Holder team"]
  const assetHeaders = [
    "Asset code", "Category", "Name", "Brand", "Model", "Serial", "Computer name", "Location", "Loan status", "Condition",
  ]
  const holderFirst = scope === "assigned"
  const assetRows = rows.map((row): ExportCell[] => {
    const holder = [row.holderEmployeeId, row.holderFullName, row.holderTeamName]
    const main = [
      row.assetCode, row.categoryName ?? row.categoryCode, row.displayName, row.brand, row.model, row.serialNumber,
      row.computerName, row.usageLocation,
      isAssigned(row) ? "Assigned" : isInStock(row) ? "In stock" : formatAssetDashboardStatusLabel(row.status),
      CONDITION_LABELS[row.condition],
    ]
    return holderFirst ? [...holder, ...main] : [...main, ...holder]
  })

  const count = (test: (row: Row) => boolean) => rows.filter(test).length
  const inStock = (condition: Row["condition"]) => (row: Row) => isInStock(row) && row.condition === condition
  const summaryRows: ExportCell[][] = [
    ["Total", rows.length],
    ["Assigned", count(isAssigned)],
    ["In stock — New", count(inStock("new"))],
    ["In stock — Spare", count(inStock("spare"))],
    ["In stock — Broken", count(inStock("broken"))],
    ["Broken — with staff", count((row) => isAssigned(row) && row.condition === "broken")],
    [],
    ["Category", "Total", "Assigned", "New", "Spare", "Broken"],
  ]
  const categoryHeaderRow = summaryRows.length - 1
  const byCategory = new Map<string, Row[]>()
  for (const row of rows) {
    const key = row.categoryName ?? row.categoryCode ?? "Uncategorized"
    byCategory.set(key, [...(byCategory.get(key) ?? []), row])
  }
  for (const [name, group] of [...byCategory].sort(([a], [b]) => a.localeCompare(b))) {
    const n = (test: (row: Row) => boolean) => group.filter(test).length
    summaryRows.push([name, group.length, n(isAssigned), n(inStock("new")), n(inStock("spare")), n(inStock("broken"))])
  }

  return [
    { name: "Summary", headers: ["Measure", "Count"], rows: summaryRows, boldRows: [categoryHeaderRow] },
    { name: "Assets", headers: holderFirst ? [...holderHeaders, ...assetHeaders] : [...assetHeaders, ...holderHeaders], rows: assetRows },
  ]
}

export function assetExportFileName(scope: AssetExportScope, date: Date, ext: "xlsx" | "csv"): string {
  const label = ASSET_EXPORT_SCOPES.find((s) => s.value === scope)?.fileLabel ?? "All"
  const pad = (n: number) => String(n).padStart(2, "0")
  return `StaffKit_Assets_${label}_${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}.${ext}`
}
