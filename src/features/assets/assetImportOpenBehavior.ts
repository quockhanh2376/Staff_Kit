type AssetImportOpenHandlers = {
  hasActiveBatchDetail: boolean
  openFreshWizard: () => void
  resetReviewFilterToAll: () => void
  refreshActiveBatch: () => Promise<void>
  loadBatchSummaries: () => Promise<void>
}

export async function syncAssetImportWizardOnOpen({
  hasActiveBatchDetail,
  openFreshWizard,
  resetReviewFilterToAll,
  refreshActiveBatch,
  loadBatchSummaries,
}: AssetImportOpenHandlers): Promise<void> {
  if (!hasActiveBatchDetail) {
    openFreshWizard()
  } else {
    resetReviewFilterToAll()
    await refreshActiveBatch()
  }

  await loadBatchSummaries()
}
