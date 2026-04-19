import assert from "node:assert/strict"

import {
  buildAssetImportActionLabel,
  buildAssetImportDeleteMessage,
  buildAssetImportSuccessMessage,
} from "../src/features/assets/assetImportStatusMeta.ts"
import type { AssetImportCommitResult } from "../src/types/staff.ts"

const sharedResult: AssetImportCommitResult = {
  batchId: 42,
  importedRowIds: [7],
  importedAssetCodes: [],
  importedCount: 1,
  remainingErrorRows: 2,
  batchStatus: "pending_review",
}

assert.equal(
  buildAssetImportSuccessMessage("quantity", sharedResult),
  "Imported 1 valid quantity row(s) into stock records only. 2 row(s) still need review.",
)

assert.equal(
  buildAssetImportSuccessMessage("serialized", sharedResult),
  "Imported 1 valid serialized row(s) into borrow-ready assets. 2 row(s) still need review.",
)

assert.equal(
  buildAssetImportDeleteMessage("BATCH-001", "quantity"),
  "Delete staged batch BATCH-001? Imported rows already committed into official stock records will be kept.",
)

assert.equal(
  buildAssetImportDeleteMessage("BATCH-002", "serialized"),
  "Delete staged batch BATCH-002? Imported rows already committed into borrow-ready assets will be kept.",
)

assert.equal(
  buildAssetImportActionLabel("quantity", 3),
  "Import Stock Rows (3)",
)

assert.equal(
  buildAssetImportActionLabel("serialized", 4),
  "Import Serialized Assets (4)",
)

console.log("asset-import-messages tests passed")
