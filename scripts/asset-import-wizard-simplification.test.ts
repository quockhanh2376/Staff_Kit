import assert from "node:assert/strict"
import fs from "node:fs"
import path from "node:path"

const repoRoot = path.resolve(import.meta.dirname, "..")

const wizardPath = path.join(repoRoot, "src/features/assets/AssetImportWizard.tsx")
const directStatePath = path.join(
  repoRoot,
  "src/features/assets/useAssetDirectImportState.ts",
)
const dashboardPath = path.join(repoRoot, "src/features/assets/AssetDashboard.tsx")

const wizardSource = fs.readFileSync(wizardPath, "utf8")
const dashboardSource = fs.readFileSync(dashboardPath, "utf8")

assert.equal(fs.existsSync(directStatePath), true, "direct asset import state hook should exist")

const directStateSource = fs.readFileSync(directStatePath, "utf8")

assert.match(wizardSource, /SharedImportShell/, "asset import should reuse the shared import shell")
assert.doesNotMatch(wizardSource, /Map Columns/, "legacy mapping step should be removed")
assert.doesNotMatch(wizardSource, /Review Batch/, "legacy batch review step should be removed")
assert.doesNotMatch(wizardSource, /Stage Batch/, "legacy stage batch action should be removed")
assert.doesNotMatch(wizardSource, /Continue to Mapping/, "legacy continue-to-mapping action should be removed")
assert.match(
  wizardSource,
  /handleApproveImport/,
  "asset import should wire approve handling through the shared shell preview",
)

assert.match(
  directStateSource,
  /previewAssetImportFile/,
  "direct import state should call previewAssetImportFile",
)
assert.match(
  directStateSource,
  /importAssetImportFile/,
  "direct import state should call importAssetImportFile",
)
assert.doesNotMatch(
  directStateSource,
  /createAssetImportBatch|listAssetImportBatches|getAssetImportBatchDetail|importAssetImportBatchValidRows/,
  "direct import state should not depend on staged batch APIs",
)

assert.doesNotMatch(
  dashboardSource,
  /Active Import Batch|Resume Review|Staged Import Batches/,
  "asset dashboard should not surface staged batch review copy",
)

console.log("asset-import-wizard-simplification tests passed")
