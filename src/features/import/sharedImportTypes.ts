export const SHARED_IMPORT_PREVIEW_SUMMARY_KEYS = ["totalRows", "validRows", "errorRows"] as const
export const SHARED_IMPORT_REPORT_COUNT_KEYS = ["imported", "skipped", "failed"] as const
export const SHARED_IMPORT_PREVIEW_ROW_KEYS = ["id", "title", "subtitle", "badge", "cells"] as const

export type {
    SharedImportErrorItem,
    SharedImportPreviewCell,
    SharedImportPreviewRow,
    SharedImportPreviewSummary,
    SharedImportReport,
    SharedImportSourceInfo,
    SharedImportStatItem,
} from "../../types/staff"
