import assert from "node:assert/strict"

import { syncAssetImportWizardOnOpen } from "../src/features/assets/assetImportOpenBehavior.ts"

const freshCalls: string[] = []
await syncAssetImportWizardOnOpen({
  hasActiveBatchDetail: false,
  openFreshWizard: () => {
    freshCalls.push("openFreshWizard")
  },
  refreshActiveBatch: async () => {
    freshCalls.push("refreshActiveBatch")
  },
  loadBatchSummaries: async () => {
    freshCalls.push("loadBatchSummaries")
  },
})

assert.deepEqual(freshCalls, ["openFreshWizard", "loadBatchSummaries"])

const reopenCalls: string[] = []
await syncAssetImportWizardOnOpen({
  hasActiveBatchDetail: true,
  openFreshWizard: () => {
    reopenCalls.push("openFreshWizard")
  },
  refreshActiveBatch: async () => {
    reopenCalls.push("refreshActiveBatch")
  },
  loadBatchSummaries: async () => {
    reopenCalls.push("loadBatchSummaries")
  },
})

assert.deepEqual(reopenCalls, ["refreshActiveBatch", "loadBatchSummaries"])

console.log("asset-import-open-behavior tests passed")
