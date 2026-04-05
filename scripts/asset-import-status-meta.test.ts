import assert from "node:assert/strict"

import {
  getAssetImportSummaryLabel,
  getAssetImportReviewFilterLabel,
  getAssetImportStatusMeta,
} from "../src/features/assets/assetImportStatusMeta.ts"

assert.equal(getAssetImportStatusMeta("valid").label, "Ready")
assert.equal(
  getAssetImportStatusMeta("valid").emptyValidationMessage,
  "This row is ready to import.",
)

assert.equal(getAssetImportStatusMeta("error").label, "Needs Fix")
assert.equal(
  getAssetImportStatusMeta("error").emptyValidationMessage,
  "Fix the validation issues on this row before importing it.",
)

assert.equal(getAssetImportStatusMeta("imported").label, "Imported")
assert.equal(
  getAssetImportStatusMeta("imported").emptyValidationMessage,
  "This row has already been imported and is now read-only.",
)

assert.equal(getAssetImportStatusMeta("skipped").label, "Skipped")
assert.equal(
  getAssetImportStatusMeta("skipped").emptyValidationMessage,
  "This row is skipped and will stay out of the next import.",
)

assert.equal(getAssetImportReviewFilterLabel("all"), "All Rows")
assert.equal(getAssetImportReviewFilterLabel("errors"), "Needs Fix")
assert.equal(getAssetImportReviewFilterLabel("pending"), "Needs Review")

assert.equal(getAssetImportSummaryLabel("valid"), "Ready")
assert.equal(getAssetImportSummaryLabel("errors"), "Needs Fix")
assert.equal(getAssetImportSummaryLabel("imported"), "Imported")
assert.equal(getAssetImportSummaryLabel("skipped"), "Skipped")

console.log("asset-import-status-meta tests passed")