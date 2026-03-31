import assert from "node:assert/strict"

import type { AssetCategoryRecord } from "../src/types/staff.ts"
import {
  getAssetImportCategoryOptions,
  resolveAssetImportCategoryValue,
} from "../src/features/assets/assetImportCategoryOptions.ts"

const categories: AssetCategoryRecord[] = [
  {
    id: 1,
    categoryCode: "laptop",
    categoryName: "Laptop",
    trackingMode: "serialized",
    prefixCode: "ASWVNLAP",
    qrRequired: true,
    isActive: true,
  },
  {
    id: 2,
    categoryCode: "monitor",
    categoryName: "Monitor",
    trackingMode: "serialized",
    prefixCode: "ASWVNMON",
    qrRequired: true,
    isActive: false,
  },
  {
    id: 3,
    categoryCode: "mouse",
    categoryName: "Mouse",
    trackingMode: "quantity",
    prefixCode: null,
    qrRequired: false,
    isActive: true,
  },
]

assert.deepEqual(getAssetImportCategoryOptions(categories, "serialized"), [
  { value: "Laptop", label: "Laptop" },
])

assert.deepEqual(getAssetImportCategoryOptions(categories, "quantity"), [
  { value: "Mouse", label: "Mouse" },
])

assert.equal(
  resolveAssetImportCategoryValue(categories, "serialized", "laptop"),
  "Laptop",
)
assert.equal(
  resolveAssetImportCategoryValue(categories, "serialized", "lApToP"),
  "Laptop",
)

assert.equal(
  resolveAssetImportCategoryValue(categories, "serialized", "LegacyDevice"),
  "LegacyDevice",
)
assert.deepEqual(
  getAssetImportCategoryOptions(categories, "serialized", "LegacyDevice"),
  [
    { value: "LegacyDevice", label: "LegacyDevice (current value)" },
    { value: "Laptop", label: "Laptop" },
  ],
)

console.log("asset-import-category-options tests passed")
