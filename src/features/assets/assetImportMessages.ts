import type { AssetImportCommitResult, AssetImportMode } from "../../types/staff"

export function buildAssetImportActionLabel(
  importType: AssetImportMode,
  readyCount: number,
): string {
  return importType === "quantity"
    ? `Import Stock Rows (${readyCount})`
    : `Import Serialized Assets (${readyCount})`
}

export function buildAssetImportSuccessMessage(
  importType: AssetImportMode,
  result: AssetImportCommitResult,
): string {
  const prefix =
    importType === "quantity"
      ? `Imported ${result.importedCount} valid quantity row(s) into stock records only.`
      : `Imported ${result.importedCount} valid serialized row(s) into borrow-ready assets.`

  return `${prefix} ${result.remainingErrorRows} row(s) still need review.`
}

export function buildAssetImportDeleteMessage(
  batchKey: string,
  importType: AssetImportMode,
): string {
  const targetLabel =
    importType === "quantity" ? "official stock records" : "borrow-ready assets"

  return `Delete staged batch ${batchKey}? Imported rows already committed into ${targetLabel} will be kept.`
}
