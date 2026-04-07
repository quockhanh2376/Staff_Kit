type AssetImportPanelMode = "import" | "manual"

export function getAssetImportPanelTitle(
  panelMode: AssetImportPanelMode,
): string {
  return panelMode === "manual" ? "Add Serialized Asset" : "Asset Import Wizard"
}

export function getAssetImportSettingsEntryActionLabel(): string {
  return "Open Import Wizard"
}

export function getAssetImportSettingsEntryDescription(): string {
  return "Start the staged desktop flow for CSV/Excel imports, then confirm quantity rows into stock records or serialized rows into borrow-ready assets. Use manual add when IT just needs one serialized asset fast."
}

export function getAssetImportSettingsManualActionLabel(): string {
  return "Add Serialized Asset"
}

export function getAssetImportBatchSummaryEmptyStateLabel(): string {
  return "No staged import batches yet."
}

export function buildAssetImportBatchSummaryCountLabel(count: number): string {
  return `${count} staged import batch(es) available for review.`
}

export function getAssetImportBatchListTitle(): string {
  return "Staged Import Batches"
}

export function getAssetImportActiveBatchTitle(): string {
  return "Active Import Batch"
}

export function getAssetImportSerializedModeDescription(): string {
  return "One serialized asset per row. Category, asset name, serial number, warehouse, and note."
}

export function getAssetImportManualPanelTitle(): string {
  return "Quick Serialized Add"
}

export function getAssetImportManualPanelDescription(): string {
  return "Serialized-only fallback for one-off serialized assets or rows that are not worth pushing through batch review."
}

export function getAssetImportManualPanelPrimaryActionLabel(
  isCreatingManualAsset: boolean,
): string {
  return isCreatingManualAsset ? "Saving..." : "Create Serialized Asset"
}

export function buildManualSerializedAssetRequiredMessage(): string {
  return "Asset code, category, and asset name are required for serialized assets."
}

export function buildManualSerializedAssetCreatedMessage(
  assetCode: string,
): string {
  return `Created serialized asset ${assetCode} as a borrow-ready asset.`
}

export function shouldCloseAssetImportWizardAfterImport(
  importedCount: number,
  remainingErrorRows: number,
): boolean {
  return importedCount > 0 && remainingErrorRows === 0
}
