import type { AssetImportCommitResult, AssetImportMode } from "../../types/staff"

export function buildAssetImportSuccessMessage(
  importType: AssetImportMode,
  result: AssetImportCommitResult,
): string {
  const prefix =
    importType === "quantity"
      ? `Imported ${result.importedCount} valid quantity row(s) into stock.`
      : `Imported ${result.importedCount} valid row(s).`

  return `${prefix} ${result.remainingErrorRows} row(s) still need review.`
}

export function buildAssetImportDeleteMessage(
  batchKey: string,
  importType: AssetImportMode,
): string {
  const targetLabel =
    importType === "quantity" ? "official stock records" : "assets"

  return `Delete staged batch ${batchKey}? Imported rows already committed into ${targetLabel} will be kept.`
}
