import { describe, expect, it } from "vitest"
import type { AssetDashboardSerializedRecord, TeamRecord } from "../../types/staff"
import { assetExportFileName, buildAssetExportSheets, selectAssetExportRows } from "./assetExport"

function row(
  id: number,
  over: Partial<AssetDashboardSerializedRecord> = {},
): AssetDashboardSerializedRecord {
  return {
    assetId: id,
    assetCode: `AST-${id}`,
    categoryCode: "H_PHONE",
    categoryName: "H Phone",
    computerName: null,
    displayName: `Phone ${id}`,
    displayNameShort: null,
    brand: null,
    model: null,
    serialNumber: null,
    adapterNumber: null,
    usageLocation: null,
    notes: null,
    status: "in_stock",
    holderEmployeeId: null,
    holderFullName: null,
    condition: "spare",
    holderTeamName: null,
    ...over,
  }
}

const team = (id: number, name: string, parentId: number | null): TeamRecord => ({
  id,
  name,
  parentId,
  parentName: null,
  memberCount: 0,
})
const TEAMS = [team(1, "Sales", null), team(2, "Sales North", 1), team(3, "Ops", null)]

const ROWS = [
  row(1, { condition: "new" }),
  row(2, { condition: "spare" }),
  row(3, { condition: "broken" }),
  row(4, { condition: "broken", status: "assigned", holderEmployeeId: "E1", holderFullName: "A", holderTeamName: "Sales" }),
  row(5, { status: "assigned", holderEmployeeId: "E2", holderFullName: "B", holderTeamName: "Sales North" }),
  row(6, { categoryCode: "LAPTOP", categoryName: "Laptop", status: "assigned", holderEmployeeId: "E3", holderFullName: "C", holderTeamName: "Ops" }),
]
const opts = (over = {}) => ({ scope: "all" as const, categoryCode: null, teamName: null, teams: TEAMS, ...over })
const ids = (rows: AssetDashboardSerializedRecord[]) => rows.map((r) => r.assetId)

describe("assetExport", () => {
  it("assignable scope returns in-stock new and spare only", () => {
    expect(ids(selectAssetExportRows(ROWS, [], opts({ scope: "assignable" })))).toEqual([1, 2])
  })

  it("broken scope excludes assigned broken", () => {
    expect(ids(selectAssetExportRows(ROWS, [], opts({ scope: "broken" })))).toEqual([3])
  })

  it("assigned scope puts holder columns first", () => {
    const [, assets] = buildAssetExportSheets(ROWS.filter((r) => r.status === "assigned"), "assigned")
    expect(assets.headers.slice(0, 3)).toEqual(["Holder staff ID", "Holder name", "Holder team"])
    expect(assets.headers).toHaveLength(13)
    expect(assets.rows[0].slice(0, 3)).toEqual(["E1", "A", "Sales"])
  })

  it("team filter matches holder team and child team", () => {
    expect(ids(selectAssetExportRows(ROWS, [], opts({ teamName: "Sales" })))).toEqual([4, 5])
    expect(ids(selectAssetExportRows(ROWS, [], opts({ teamName: "Sales North" })))).toEqual([5])
  })

  it("category filter and currentFilter scope", () => {
    expect(ids(selectAssetExportRows(ROWS, [], opts({ categoryCode: "LAPTOP" })))).toEqual([6])
    expect(ids(selectAssetExportRows(ROWS, [ROWS[2]], opts({ scope: "currentFilter" })))).toEqual([3])
  })

  it("team filter with in-stock scope yields empty Assets and zero Summary", () => {
    const rows = selectAssetExportRows(ROWS, [], opts({ scope: "spare", teamName: "Sales" }))
    expect(rows).toEqual([])
    const [summary, assets] = buildAssetExportSheets(rows, "spare")
    expect(assets.rows).toEqual([])
    expect(summary.rows.slice(0, 5).map((r) => r[1])).toEqual([0, 0, 0, 0, 0])
  })

  it("summary counts are numbers and per-category totals add up", () => {
    const [summary] = buildAssetExportSheets(ROWS, "all")
    const top = summary.rows.slice(0, 5)
    expect(top.map((r) => r[0])).toEqual(["Total", "Assigned", "In stock — New", "In stock — Spare", "In stock — Broken"])
    expect(top.map((r) => r[1])).toEqual([6, 3, 1, 1, 1])
    const header = summary.rows.find((r) => r[0] === "Category")
    expect(header).toEqual(["Category", "Total", "Assigned", "New", "Spare", "Broken"])
    const cats = summary.rows.slice(summary.rows.indexOf(header!) + 1)
    expect(cats).toEqual([
      ["H Phone", 5, 2, 1, 1, 1],
      ["Laptop", 1, 1, 0, 0, 0],
    ])
  })

  it("file name uses scope label and date", () => {
    expect(assetExportFileName("broken", new Date(2026, 10, 23), "xlsx")).toBe("StaffKit_Assets_Broken_2026-11-23.xlsx")
    expect(assetExportFileName("currentFilter", new Date(2026, 10, 3), "csv")).toBe("StaffKit_Assets_Filtered_2026-11-03.csv")
  })
})
