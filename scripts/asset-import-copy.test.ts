import assert from "node:assert/strict"

import {
  buildManualSerializedAssetCreatedMessage,
  buildManualSerializedAssetRequiredMessage,
  buildAssetImportBatchSummaryCountLabel,
  getAssetImportActiveBatchTitle,
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

assert.equal(getAssetImportPanelTitle("import"), "Asset Import")
assert.equal(getAssetImportPanelTitle("manual"), "Add Serialized Asset")

assert.equal(getAssetImportSettingsEntryActionLabel(), "Import Assets")
assert.equal(
  getAssetImportSettingsEntryDescription(),
  "Import CSV or Excel files into serialized assets or quantity stock with automatic preview and approval. Valid rows import into the database; invalid rows stay in the report.",
)
assert.equal(
  getAssetImportSettingsManualActionLabel(),
  "Add Serialized Asset",
)
assert.equal(
  getAssetImportBatchSummaryEmptyStateLabel(),
  "No recent import report yet.",
)
assert.equal(
  buildAssetImportBatchSummaryCountLabel(3),
  "Last import processed 3 row(s).",
)
assert.equal(
  getAssetImportBatchListTitle(),
  "Latest Import Report",
)
assert.equal(
  getAssetImportActiveBatchTitle(),
  "Preview Summary",
)
assert.equal(
  getAssetImportSerializedModeDescription(),
  "One serialized asset per row. The app auto-detects known columns and previews valid/error rows before approval.",
)

assert.equal(getAssetImportManualPanelTitle(), "Quick Serialized Add")
assert.equal(
  getAssetImportManualPanelDescription(),
  "Serialized-only fallback for one-off assets that do not need the file import flow.",
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
