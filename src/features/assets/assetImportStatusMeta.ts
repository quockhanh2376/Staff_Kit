import type { AssetImportCommitResult, AssetImportMode } from "../../types/staff"

// ── Row Status Metadata ──────────────────────────────────────────────────────

type AssetImportRowStatus = "valid" | "error" | "imported" | "skipped"
type AssetImportReviewFilter = "all" | "errors" | "pending"
type AssetImportSummaryKey = "valid" | "errors" | "imported" | "skipped"

type AssetImportStatusMeta = {
  label: string
  emptyValidationMessage: string
}

const ASSET_IMPORT_STATUS_META: Record<AssetImportRowStatus, AssetImportStatusMeta> = {
  valid: {
    label: "New",
    emptyValidationMessage: "This row is new and ready to import.",
  },
  error: {
    label: "Error / Conflict",
    emptyValidationMessage: "Fix the validation or identity conflict on this row before importing it.",
  },
  imported: {
    label: "Imported",
    emptyValidationMessage: "This row has already been imported and is now read-only.",
  },
  skipped: {
    label: "Existing / Skipped",
    emptyValidationMessage: "This existing row is skipped and will stay out of the next import.",
  },
}

const ASSET_IMPORT_REVIEW_FILTER_LABELS: Record<AssetImportReviewFilter, string> = {
  all: "All Rows",
  errors: "Needs Fix",
  pending: "Needs Review",
}

const ASSET_IMPORT_SUMMARY_LABELS: Record<AssetImportSummaryKey, string> = {
  valid: "New Rows",
  errors: "Error Rows",
  imported: "Imported",
  skipped: "Existing / Skipped",
}

export function getAssetImportStatusMeta(status: string): AssetImportStatusMeta {
  return ASSET_IMPORT_STATUS_META[status as AssetImportRowStatus] ?? {
    label: status,
    emptyValidationMessage: "No validation errors.",
  }
}

export function getAssetImportReviewFilterLabel(filter: AssetImportReviewFilter): string {
  return ASSET_IMPORT_REVIEW_FILTER_LABELS[filter]
}

export function getAssetImportSummaryLabel(summaryKey: AssetImportSummaryKey): string {
  return ASSET_IMPORT_SUMMARY_LABELS[summaryKey]
}

// ── Batch Action Messages ────────────────────────────────────────────────────

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
