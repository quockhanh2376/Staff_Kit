import assert from "node:assert/strict"

import {
  ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  filterSerializedAssetRows,
} from "../src/features/assets/serializedAssetFilters.ts"
import type { AssetDashboardSerializedRecord } from "../src/types/staff.ts"

const rows: AssetDashboardSerializedRecord[] = [
  {
    assetId: 1,
    assetCode: "VNLAP235",
    categoryCode: "laptop",
    categoryName: "Laptop",
    computerName: "ASWVNLAP235",
    displayName: "Dell Latitude 3520",
    displayNameShort: null,
    model: "Dell Latitude 3520",
    serialNumber: "SN-235",
    adapterNumber: null,
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
  filterSerializedAssetRows(rows, {
    searchTerm: "vnlap235",
    categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  }).map((row) => row.assetCode),
  ["VNLAP235"],
)

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "  aswvnlap235  ",
    categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  }).map((row) => row.assetCode),
  ["VNLAP235"],
)

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "hung",
    categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  }).map((row) => row.assetCode),
  ["VNLAP235"],
)

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "lg 27",
    categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  }).map((row) => row.assetCode),
  ["VNMON709"],
)

console.log("serialized-asset-filters tests passed")
