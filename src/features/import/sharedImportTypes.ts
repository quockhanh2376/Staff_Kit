export const SHARED_IMPORT_PREVIEW_SUMMARY_KEYS = ["totalRows", "validRows", "errorRows"] as const
export const SHARED_IMPORT_REPORT_COUNT_KEYS = ["imported", "skipped", "failed"] as const
export const SHARED_IMPORT_PREVIEW_ROW_KEYS = ["id", "title", "subtitle", "badge", "cells"] as const

export type SharedImportStatItem = {
    label: string
    value: string | number
}

export type SharedImportPreviewSummary = {
    totalRows: number
    validRows: number
    errorRows: number
}

export type SharedImportPreviewCell = {
    key: string
    label: string
    value: string
}

export type SharedImportPreviewRow = {
    id: string
    title: string
    subtitle: string | null
    badge: string | null
    cells: SharedImportPreviewCell[]
}

export type SharedImportErrorItem = {
    rowNumber: number
    entityKey: string | null
    reason: string
}

export type SharedImportSourceInfo = {
    sourceFiles: string[]
    sheetName: string | null
}

export type SharedImportReport = {
    imported: number
    skipped: number
    failed: number
    errors: SharedImportErrorItem[]
}
