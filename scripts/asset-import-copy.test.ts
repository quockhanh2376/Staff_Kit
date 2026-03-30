import assert from "node:assert/strict"

import {
  buildManualSerializedAssetCreatedMessage,
  buildManualSerializedAssetRequiredMessage,
  getAssetImportManualPanelDescription,
  getAssetImportManualPanelPrimaryActionLabel,
  getAssetImportManualPanelTitle,
  getAssetImportPanelTitle,
  getAssetImportSerializedModeDescription,
  getAssetImportSettingsEntryActionLabel,
} from "../src/features/assets/assetImportCopy.ts"

assert.equal(getAssetImportPanelTitle("import"), "Asset Import Wizard")
assert.equal(getAssetImportPanelTitle("manual"), "Add Serialized Asset")

assert.equal(getAssetImportSettingsEntryActionLabel(), "Open Import Wizard")
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
