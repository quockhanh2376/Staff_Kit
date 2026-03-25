type AssetImportRowStatus = "valid" | "error" | "imported" | "skipped"
type AssetImportReviewFilter = "all" | "errors" | "pending"
type AssetImportSummaryKey = "valid" | "errors" | "imported" | "skipped"

type AssetImportStatusMeta = {
  label: string
  emptyValidationMessage: string
}

const ASSET_IMPORT_STATUS_META: Record<AssetImportRowStatus, AssetImportStatusMeta> = {
  valid: {
    label: "Ready",
    emptyValidationMessage: "This row is ready to import.",
  },
  error: {
    label: "Needs Fix",
    emptyValidationMessage: "Fix the validation issues on this row before importing it.",
  },
  imported: {
    label: "Imported",
    emptyValidationMessage: "This row has already been imported and is now read-only.",
  },
  skipped: {
    label: "Skipped",
    emptyValidationMessage: "This row is skipped and will stay out of the next import.",
  },
}

const ASSET_IMPORT_REVIEW_FILTER_LABELS: Record<AssetImportReviewFilter, string> = {
  all: "All Rows",
  errors: "Needs Fix",
  pending: "Needs Review",
}

const ASSET_IMPORT_SUMMARY_LABELS: Record<AssetImportSummaryKey, string> = {
  valid: "Ready",
  errors: "Needs Fix",
  imported: "Imported",
  skipped: "Skipped",
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
