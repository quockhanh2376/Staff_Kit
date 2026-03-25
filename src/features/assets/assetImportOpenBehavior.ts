type AssetImportOpenHandlers = {
  hasActiveBatchDetail: boolean
  openFreshWizard: () => void
  refreshActiveBatch: () => Promise<void>
  loadBatchSummaries: () => Promise<void>
}

export async function syncAssetImportWizardOnOpen({
  hasActiveBatchDetail,
  openFreshWizard,
  refreshActiveBatch,
  loadBatchSummaries,
}: AssetImportOpenHandlers): Promise<void> {
  if (!hasActiveBatchDetail) {
    openFreshWizard()
  } else {
    await refreshActiveBatch()
  }

  await loadBatchSummaries()
}
