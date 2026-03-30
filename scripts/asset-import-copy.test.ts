import assert from "node:assert/strict"

import {
  buildManualSerializedAssetCreatedMessage,
  buildManualSerializedAssetRequiredMessage,
  buildAssetImportBatchSummaryCountLabel,
  getAssetImportBatchListTitle,
  getAssetImportManualPanelDescription,
  getAssetImportManualPanelPrimaryActionLabel,
  getAssetImportManualPanelTitle,
  getAssetImportBatchSummaryEmptyStateLabel,
  getAssetImportPanelTitle,
  getAssetImportSerializedModeDescription,
  getAssetImportSettingsEntryDescription,
  getAssetImportSettingsEntryActionLabel,
  getAssetImportSettingsManualActionLabel,
} from "../src/features/assets/assetImportCopy.ts"

assert.equal(getAssetImportPanelTitle("import"), "Asset Import Wizard")
assert.equal(getAssetImportPanelTitle("manual"), "Add Serialized Asset")

assert.equal(getAssetImportSettingsEntryActionLabel(), "Open Import Wizard")
assert.equal(
  getAssetImportSettingsEntryDescription(),
  "Start the staged desktop flow for CSV/Excel imports, then confirm quantity rows into stock records or serialized rows into borrow-ready assets. Use manual add when IT just needs one serialized asset fast.",
)
assert.equal(
  getAssetImportSettingsManualActionLabel(),
  "Add Serialized Asset",
)
assert.equal(
  getAssetImportBatchSummaryEmptyStateLabel(),
  "No staged import batches yet.",
)
assert.equal(
  buildAssetImportBatchSummaryCountLabel(3),
  "3 staged import batch(es) available for review.",
)
assert.equal(
  getAssetImportBatchListTitle(),
  "Staged Import Batches",
)
assert.equal(
  getAssetImportSerializedModeDescription(),
  "One serialized asset per row. Category, asset name, serial number, warehouse, and note.",
)

assert.equal(getAssetImportManualPanelTitle(), "Quick Serialized Add")
assert.equal(
  getAssetImportManualPanelDescription(),
  "Serialized-only fallback for one-off serialized assets or rows that are not worth pushing through batch review.",
)
assert.equal(
  getAssetImportManualPanelPrimaryActionLabel(false),
  "Create Serialized Asset",
)
assert.equal(
  getAssetImportManualPanelPrimaryActionLabel(true),
  "Saving...",
)

assert.equal(
  buildManualSerializedAssetRequiredMessage(),
  "Asset code, category, and asset name are required for serialized assets.",
)
assert.equal(
  buildManualSerializedAssetCreatedMessage("ASSET-001"),
  "Created serialized asset ASSET-001 as a borrow-ready asset.",
)

console.log("asset-import-copy tests passed")
