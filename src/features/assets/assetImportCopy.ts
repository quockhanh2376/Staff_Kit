type AssetImportPanelMode = "import" | "manual"

export function getAssetImportPanelTitle(
  panelMode: AssetImportPanelMode,
): string {
  return panelMode === "manual" ? "Add Serialized Asset" : "Asset Import"
}

export function getAssetImportSettingsEntryActionLabel(): string {
  return "Import Assets"
}

export function getAssetImportSettingsEntryDescription(): string {
  return "Import CSV or Excel files into serialized assets or quantity stock with automatic preview and approval. New rows import into the database; existing and conflict rows stay visible in the report."
}

export function getAssetImportSettingsManualActionLabel(): string {
  return "Add Serialized Asset"
}

export function getAssetImportBatchSummaryEmptyStateLabel(): string {
  return "No recent import report yet."
}

export function buildAssetImportBatchSummaryCountLabel(count: number): string {
  return `Last import processed ${count} row(s).`
}

export function getAssetImportBatchListTitle(): string {
  return "Latest Import Report"
}

export function getAssetImportActiveBatchTitle(): string {
  return "Preview Summary"
}

export function getAssetImportSerializedModeDescription(): string {
  return "One serialized asset per row. The app auto-detects known columns and previews new/existing/conflict rows before approval."
}

export function getAssetImportManualPanelTitle(): string {
  return "Quick Serialized Add"
}

export function getAssetImportManualPanelDescription(): string {
  return "Serialized-only fallback for one-off assets that do not need the file import flow."
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
