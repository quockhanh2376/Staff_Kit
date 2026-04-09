import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

import {
  DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER,
  SERIALIZED_ASSET_COLUMN_MAP,
  buildSerializedAssetGridStorageKeys,
  cycleSerializedAssetSort,
  sortSerializedAssetRows,
  type SerializedAssetGridSort,
} from "../src/features/assets/serializedAssetGridConfig.ts"
import type { AssetDashboardSerializedRecord } from "../src/types/staff.ts"

assert.deepEqual(DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER, [
  "id",
  "category",
  "computerName",
  "assetName",
  "model",
  "serialNumber",
  "adapterNumber",
  "usageLocation",
  "note",
  "status",
  "holder",
])

assert.equal(SERIALIZED_ASSET_COLUMN_MAP.id.label, "ID")
assert.equal(SERIALIZED_ASSET_COLUMN_MAP.serialNumber.label, "Serial Number")
assert.equal(SERIALIZED_ASSET_COLUMN_MAP.usageLocation.label, "Usage Location")

const scopedKeys = buildSerializedAssetGridStorageKeys("adman")
assert.equal(
  scopedKeys.order,
  "staffkit:asset-dashboard-serialized-grid:adman:order",
)
assert.equal(
  scopedKeys.widths,
  "staffkit:asset-dashboard-serialized-grid:adman:widths",
)

let sort: SerializedAssetGridSort = { key: null, direction: null }
sort = cycleSerializedAssetSort(sort, "id")
assert.deepEqual(sort, { key: "id", direction: "asc" })
sort = cycleSerializedAssetSort(sort, "id")
assert.deepEqual(sort, { key: "id", direction: "desc" })
sort = cycleSerializedAssetSort(sort, "id")
assert.deepEqual(sort, { key: null, direction: null })

const rows: AssetDashboardSerializedRecord[] = [
  {
    assetId: 1,
    assetCode: "VNLAP235",
    categoryCode: "laptop",
    categoryName: "Laptop",
    computerName: "ASWVNLAP235",
    displayName: "ASWVNLAP235",
    displayNameShort: null,
    model: "Dell Latitude 3520",
    serialNumber: "7900LG3",
    adapterNumber: "7900LG3",
    usageLocation: null,
    notes: null,
    status: "assigned",
    holderEmployeeId: "ASWVN1302",
    holderFullName: "Le The Hung",
  },
  {
    assetId: 2,
    assetCode: "VNMON709",
    categoryCode: "monitor",
    categoryName: "Monitor",
    computerName: null,
    displayName: "Mon709",
    displayNameShort: "Mon709",
    model: "LG 27",
    serialNumber: null,
    adapterNumber: null,
    usageLocation: "office",
    notes: "Window desk",
    status: "in_stock",
    holderEmployeeId: null,
    holderFullName: null,
  },
]

assert.deepEqual(
  sortSerializedAssetRows(rows, { key: "computerName", direction: "asc" }).map(
    (row) => row.assetCode,
  ),
  ["VNLAP235", "VNMON709"],
)
assert.deepEqual(
  sortSerializedAssetRows(rows, { key: "category", direction: "asc" }).map(
    (row) => row.assetCode,
  ),
  ["VNLAP235", "VNMON709"],
)
assert.deepEqual(
  sortSerializedAssetRows(rows, { key: "status", direction: "desc" }).map(
    (row) => row.assetCode,
  ),
  ["VNMON709", "VNLAP235"],
)

const appSource = readFileSync("src/App.tsx", "utf8")
const settingsViewSource = readFileSync("src/features/settings/SettingsView.tsx", "utf8")
const assetDashboardSource = readFileSync("src/features/assets/AssetDashboard.tsx", "utf8")

assert.match(appSource, /activeUserScope=\{activeUserScope\}/)
assert.match(settingsViewSource, /activeUserScope: string/)
assert.match(settingsViewSource, /activeUserScope=\{activeUserScope\}/)
assert.match(assetDashboardSource, /useSerializedAssetGridState/)
assert.match(assetDashboardSource, /renderSerializedCellValue/)
assert.match(assetDashboardSource, /orderedColumns\.map\(\(column\) => \(/)
assert.doesNotMatch(assetDashboardSource, />\s*Display\s*</)
assert.doesNotMatch(assetDashboardSource, />\s*Usage\s*</)

console.log("serialized-asset-grid tests passed")
