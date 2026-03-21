import { AlertCircle, FileSpreadsheet, PlusCircle, RefreshCw, Trash2, Upload } from "lucide-react"
import { Drawer } from "../../components/Drawer"
import { formatDate } from "../../lib/utils"
import type { AssetImportFieldMapping } from "../../types/staff"
import {
    assetImportEditableRowFieldKeys,
    assetImportOptionalMappingKeys,
    assetImportRequiredMappingKeys,
    hasRequiredAssetImportMapping,
    type AssetImportState,
} from "./useAssetImportState"

type AssetImportWizardProps = {
    assetImport: AssetImportState
}

const FIELD_LABELS: Record<keyof AssetImportFieldMapping, string> = {
    assetCode: "Asset Code",
    assetType: "Asset Type",
    displayName: "Display Name",
    model: "Model",
    serialNumber: "Serial Number",
    notes: "Notes",
}

const REQUIRED_FIELD_KEY_SET = new Set<string>(assetImportRequiredMappingKeys)

export function AssetImportWizard({ assetImport }: AssetImportWizardProps) {
    return (
        <Drawer
            open={assetImport.isWizardOpen}
            onClose={assetImport.closeWizard}
            title={assetImport.panelMode === "manual" ? "Add Asset Manually" : "Asset Import Wizard"}
            widthClass="w-[1080px]"
        >
            {assetImport.panelMode === "manual" ? (
                <ManualAssetPanel assetImport={assetImport} />
            ) : (
                <ImportPanel assetImport={assetImport} />
            )}
        </Drawer>
    )
}

function ImportPanel({ assetImport }: AssetImportWizardProps) {
    const detail = assetImport.activeBatchDetail

    return (
        <div className="space-y-5">
            <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-4">
                <div className="flex flex-wrap gap-2 text-xs">
                    <StepPill label="1. Choose File" active={assetImport.currentStep === "choose_file"} />
                    <StepPill label="2. Map Columns" active={assetImport.currentStep === "map_columns"} />
                    <StepPill label="3. Review Batch" active={assetImport.currentStep === "review_batch"} />
                </div>
                <p className="mt-3 text-sm text-[var(--text-secondary)]">
                    Desktop-first flow: inspect file, stage batch in SQLite, then review before importing valid rows.
                </p>
                {assetImport.statusMessage && (
                    <div className="mt-3 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                        {assetImport.statusMessage}
                    </div>
                )}
            </div>

            <div className="grid gap-5 xl:grid-cols-[1.45fr_0.95fr]">
                <div className="space-y-5">
                    {assetImport.currentStep === "choose_file" && <ChooseFileStep assetImport={assetImport} />}
                    {assetImport.currentStep === "map_columns" && <MapColumnsStep assetImport={assetImport} />}
                    {assetImport.currentStep === "review_batch" && detail && <ReviewBatchStep assetImport={assetImport} />}
                </div>

                <div className="space-y-5">
                    <ExistingBatchPanel assetImport={assetImport} />
                    {detail && (
                        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
                            <div className="text-sm font-semibold text-[var(--text-primary)]">Active Batch</div>
                            <div className="mt-2 text-xs text-[var(--text-secondary)]">
                                {detail.summary.batchKey} | {detail.summary.sourceFileName}
                            </div>
                            <div className="mt-3 grid gap-3 sm:grid-cols-2">
                                <Stat label="Valid" value={String(detail.summary.validRows)} />
                                <Stat label="Errors" value={String(detail.summary.errorRows)} />
                                <Stat label="Imported" value={String(detail.summary.importedRows)} />
                                <Stat label="Skipped" value={String(detail.summary.skippedRows)} />
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}

function ChooseFileStep({ assetImport }: AssetImportWizardProps) {
    const inspection = assetImport.inspection

    return (
        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                <FileSpreadsheet size={16} />
                Choose File
            </div>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">
                Pick a `csv`, `xlsx`, or `xls` file, then inspect headers before staging.
            </p>
            <div className="mt-4 flex flex-wrap gap-3">
                <button
                    className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                    onClick={() => void assetImport.handlePickImportFile()}
                    type="button"
                    disabled={assetImport.isInspectingFile || assetImport.isCreatingBatch}
                >
                    {assetImport.isInspectingFile ? "Inspecting..." : "Choose File"}
                </button>
                {inspection && (
                    <button
                        className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                        onClick={() => {
                            if (inspection.requiresManualMapping) {
                                assetImport.setCurrentStep("map_columns")
                                return
                            }
                            void assetImport.handleStageBatch()
                        }}
                        type="button"
                        disabled={assetImport.isCreatingBatch}
                    >
                        {inspection.requiresManualMapping
                            ? "Continue to Mapping"
                            : assetImport.isCreatingBatch
                              ? "Staging..."
                              : "Stage Batch"}
                    </button>
                )}
            </div>

            {!inspection ? (
                <div className="mt-5 rounded-[8px] border border-dashed border-[var(--border)] bg-[var(--surface-hover)]/25 p-4 text-sm text-[var(--text-secondary)]">
                    No file selected yet.
                </div>
            ) : (
                <div className="mt-5 space-y-4">
                    <div className="grid gap-3 md:grid-cols-2">
                        <InfoCard label="File" value={inspection.fileName} />
                        <InfoCard label="Type" value={inspection.fileType.toUpperCase()} />
                        <InfoCard label="Header Row" value={String(inspection.headerRow)} />
                        <InfoCard
                            label="Mapping Status"
                            value={inspection.requiresManualMapping ? "Needs manual mapping" : "Ready to stage"}
                        />
                    </div>
                    {inspection.availableSheets.length > 1 && (
                        <div>
                            <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                Excel Sheet
                            </label>
                            <select
                                className="form-input"
                                value={assetImport.selectedSheetName ?? ""}
                                onChange={(event) => void assetImport.handleChangeSelectedSheet(event.target.value)}
                                disabled={assetImport.isInspectingFile}
                            >
                                {inspection.availableSheets.map((sheet) => (
                                    <option key={sheet} value={sheet}>
                                        {sheet}
                                    </option>
                                ))}
                            </select>
                        </div>
                    )}
                    <div className="flex flex-wrap gap-2">
                        {inspection.headers.map((header) => (
                            <span
                                key={header}
                                className="rounded-[999px] border border-[var(--border)] bg-[var(--surface-hover)] px-2.5 py-1 text-xs text-[var(--text-primary)]"
                            >
                                {header}
                            </span>
                        ))}
                    </div>
                </div>
            )}
        </div>
    )
}

function MapColumnsStep({ assetImport }: AssetImportWizardProps) {
    const inspection = assetImport.inspection

    if (!inspection) {
        return null
    }

    const mappingKeys = [...assetImportRequiredMappingKeys, ...assetImportOptionalMappingKeys]

    return (
        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
            <div className="text-sm font-semibold text-[var(--text-primary)]">Map Columns</div>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">
                Required fields must be mapped before staging a batch.
            </p>
            <div className="mt-4 grid gap-4 md:grid-cols-2">
                {mappingKeys.map((fieldKey) => (
                    <div key={fieldKey}>
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            {FIELD_LABELS[fieldKey]}
                            {REQUIRED_FIELD_KEY_SET.has(fieldKey) ? " *" : ""}
                        </label>
                        <select
                            className="form-input"
                            value={assetImport.mappingDraft[fieldKey] ?? ""}
                            onChange={(event) =>
                                assetImport.updateMappingField(fieldKey, event.target.value || null)
                            }
                        >
                            <option value="">Not mapped</option>
                            {inspection.headers.map((header) => (
                                <option key={`${fieldKey}-${header}`} value={header}>
                                    {header}
                                </option>
                            ))}
                        </select>
                    </div>
                ))}
            </div>
            <div className="mt-4 flex gap-3 border-t border-[var(--border)] pt-4">
                <button
                    className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)]"
                    onClick={() => assetImport.setCurrentStep("choose_file")}
                    type="button"
                >
                    Back
                </button>
                <button
                    className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                    onClick={() => void assetImport.handleStageBatch()}
                    type="button"
                    disabled={!hasRequiredAssetImportMapping(assetImport.mappingDraft) || assetImport.isCreatingBatch}
                >
                    {assetImport.isCreatingBatch ? "Staging..." : "Stage Batch"}
                </button>
            </div>
        </div>
    )
}

function ReviewBatchStep({ assetImport }: AssetImportWizardProps) {
    const detail = assetImport.activeBatchDetail

    if (!detail) {
        return null
    }

    return (
        <div className="space-y-5">
            <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
                <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                        <div className="text-sm font-semibold text-[var(--text-primary)]">Review Batch</div>
                        <div className="mt-1 text-xs text-[var(--text-secondary)]">
                            {detail.summary.batchKey} | {formatDate(detail.summary.updatedAt)}
                        </div>
                    </div>
                    <div className="flex flex-wrap gap-2">
                        <button
                            className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-medium text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                            onClick={() => void assetImport.refreshActiveBatch()}
                            type="button"
                            disabled={assetImport.isRefreshingBatch}
                        >
                            <span className="inline-flex items-center gap-2">
                                <RefreshCw size={14} />
                                {assetImport.isRefreshingBatch ? "Refreshing..." : "Refresh"}
                            </span>
                        </button>
                        <button
                            className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-xs font-semibold text-[#00131c] disabled:opacity-50"
                            onClick={() => void assetImport.handleImportValidRows()}
                            type="button"
                            disabled={assetImport.isImportingRows || detail.summary.validRows === 0}
                        >
                            {assetImport.isImportingRows ? "Importing..." : `Import Valid (${detail.summary.validRows})`}
                        </button>
                        <button
                            className="rounded-[8px] border border-red-500/50 px-3 py-2 text-xs font-medium text-red-300 transition hover:bg-red-500/10 disabled:opacity-50"
                            onClick={() => void assetImport.handleDeleteActiveBatch()}
                            type="button"
                            disabled={assetImport.isDeletingBatch}
                        >
                            <span className="inline-flex items-center gap-2">
                                <Trash2 size={14} />
                                {assetImport.isDeletingBatch ? "Deleting..." : "Delete"}
                            </span>
                        </button>
                    </div>
                </div>
                <div className="mt-4 flex flex-wrap gap-2">
                    {[
                        { key: "all", label: "All Rows" },
                        { key: "errors", label: "Errors Only" },
                        { key: "pending", label: "Pending Only" },
                    ].map((filter) => (
                        <button
                            key={filter.key}
                            className={`rounded-[999px] border px-3 py-1.5 text-xs font-medium transition ${
                                assetImport.reviewFilter === filter.key
                                    ? "border-[var(--primary)]/50 bg-[var(--primary)]/12 text-[var(--primary)]"
                                    : "border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                            }`}
                            onClick={() => assetImport.setReviewFilter(filter.key as "all" | "errors" | "pending")}
                            type="button"
                        >
                            {filter.label}
                        </button>
                    ))}
                </div>
            </div>

            <div className="grid gap-5 xl:grid-cols-[1.55fr_0.85fr]">
                <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
                    <div className="mb-3 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                        Review Grid
                    </div>
                    <div className="max-h-[520px] overflow-auto rounded-[10px] border border-[var(--border)]">
                        <table className="min-w-full divide-y divide-[var(--border)] text-sm">
                            <thead className="bg-[var(--surface-hover)]/40 text-left text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                <tr>
                                    <th className="px-3 py-2">Row</th>
                                    <th className="px-3 py-2">Status</th>
                                    {assetImportEditableRowFieldKeys.map((fieldKey) => (
                                        <th key={fieldKey} className="px-3 py-2">{FIELD_LABELS[fieldKey]}</th>
                                    ))}
                                    <th className="px-3 py-2">Action</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[var(--border)]">
                                {assetImport.filteredRows.map((row) => (
                                    <tr
                                        key={row.id}
                                        className={assetImport.selectedRowId === row.id ? "bg-[var(--primary)]/6" : ""}
                                        onClick={() => assetImport.setSelectedRowId(row.id)}
                                    >
                                        <td className="px-3 py-2 align-top text-xs text-[var(--text-secondary)]">{row.rowNumber}</td>
                                        <td className="px-3 py-2 align-top">
                                            <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--text-primary)]">
                                                {row.status}
                                            </span>
                                        </td>
                                        {assetImportEditableRowFieldKeys.map((fieldKey) => (
                                            <td key={`${row.id}-${fieldKey}`} className="px-2 py-2 align-top">
                                                <input
                                                    className="form-input min-w-[130px] text-xs"
                                                    defaultValue={row[fieldKey] ?? ""}
                                                    disabled={row.status === "imported" || assetImport.isUpdatingRow === row.id}
                                                    onBlur={(event) => {
                                                        const nextValue = event.target.value
                                                        if ((row[fieldKey] ?? "") === nextValue) return
                                                        void assetImport.handleUpdateRowField(row.id, fieldKey, nextValue)
                                                    }}
                                                />
                                            </td>
                                        ))}
                                        <td className="px-3 py-2 align-top">
                                            <button
                                                className="rounded-[8px] border border-[var(--border)] px-2.5 py-1.5 text-xs font-medium text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                                                onClick={(event) => {
                                                    event.stopPropagation()
                                                    void assetImport.handleToggleRowSkipped(row)
                                                }}
                                                type="button"
                                                disabled={row.status === "imported" || assetImport.isUpdatingRow === row.id}
                                            >
                                                {row.status === "skipped" ? "Unskip" : "Skip"}
                                            </button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>

                <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
                    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                        <AlertCircle size={14} />
                        Focused Error Panel
                    </div>
                    {!assetImport.selectedRow ? (
                        <div className="mt-4 rounded-[8px] border border-dashed border-[var(--border)] bg-[var(--surface-hover)]/25 p-4 text-sm text-[var(--text-secondary)]">
                            Select a row to inspect raw values and validation errors.
                        </div>
                    ) : (
                        <div className="mt-4 space-y-4">
                            <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                                <div className="text-sm font-semibold text-[var(--text-primary)]">
                                    Row {assetImport.selectedRow.rowNumber}
                                </div>
                                <div className="mt-1 text-xs text-[var(--text-secondary)]">
                                    Status: {assetImport.selectedRow.status}
                                </div>
                            </div>
                            <div>
                                <div className="mb-2 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                    Validation Errors
                                </div>
                                {assetImport.selectedRow.validationErrors.length === 0 ? (
                                    <div className="rounded-[8px] border border-emerald-500/35 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-300">
                                        No validation errors.
                                    </div>
                                ) : (
                                    <div className="space-y-2">
                                        {assetImport.selectedRow.validationErrors.map((error) => (
                                            <div
                                                key={error}
                                                className="rounded-[8px] border border-amber-400/35 bg-amber-400/10 px-3 py-2 text-xs text-amber-200"
                                            >
                                                {error}
                                            </div>
                                        ))}
                                    </div>
                                )}
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}

function ExistingBatchPanel({ assetImport }: AssetImportWizardProps) {
    return (
        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                <Upload size={16} />
                Existing Staged Batches
            </div>
            <div className="mt-4 space-y-3">
                {assetImport.batchSummaries.length === 0 ? (
                    <div className="rounded-[8px] border border-dashed border-[var(--border)] bg-[var(--surface-hover)]/25 px-3 py-3 text-sm text-[var(--text-secondary)]">
                        No staged batches yet.
                    </div>
                ) : (
                    assetImport.batchSummaries.slice(0, 6).map((summary) => (
                        <button
                            key={summary.id}
                            className="w-full rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-3 text-left transition hover:bg-[var(--surface-hover)]"
                            onClick={() => void assetImport.openBatchDetail(summary.id)}
                            type="button"
                        >
                            <div className="text-sm font-semibold text-[var(--text-primary)]">{summary.batchKey}</div>
                            <div className="mt-1 text-xs text-[var(--text-secondary)]">{summary.sourceFileName}</div>
                            <div className="mt-2 text-[11px] text-[var(--text-secondary)]">
                                Valid {summary.validRows} | Errors {summary.errorRows} | Imported {summary.importedRows}
                            </div>
                        </button>
                    ))
                )}
            </div>
        </div>
    )
}

function ManualAssetPanel({ assetImport }: AssetImportWizardProps) {
    return (
        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                <PlusCircle size={16} />
                Quick Manual Add
            </div>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">
                Use this path for one-off assets or rows that are not worth pushing through batch review.
            </p>
            <div className="mt-4 grid gap-4 md:grid-cols-2">
                {assetImportEditableRowFieldKeys.map((fieldKey) => (
                    <div key={fieldKey}>
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            {FIELD_LABELS[fieldKey]}
                            {REQUIRED_FIELD_KEY_SET.has(fieldKey) ? " *" : ""}
                        </label>
                        <input
                            className="form-input"
                            value={assetImport.manualAssetForm[fieldKey] ?? ""}
                            onChange={(event) =>
                                assetImport.handleManualAssetFieldChange(fieldKey, event.target.value)
                            }
                            placeholder={FIELD_LABELS[fieldKey]}
                        />
                    </div>
                ))}
            </div>
            <div className="mt-5 flex gap-3 border-t border-[var(--border)] pt-4">
                <button
                    className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)]"
                    onClick={assetImport.closeWizard}
                    type="button"
                >
                    Close
                </button>
                <button
                    className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                    onClick={() => void assetImport.handleCreateManualAsset()}
                    type="button"
                    disabled={assetImport.isCreatingManualAsset}
                >
                    {assetImport.isCreatingManualAsset ? "Saving..." : "Create Asset"}
                </button>
            </div>
            {assetImport.manualAssetMessage && (
                <div className="mt-4 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                    {assetImport.manualAssetMessage}
                </div>
            )}
        </div>
    )
}

function StepPill({ label, active }: { label: string; active: boolean }) {
    return (
        <div
            className={`rounded-[999px] border px-3 py-1.5 text-xs font-medium ${
                active
                    ? "border-[var(--primary)]/45 bg-[var(--primary)]/12 text-[var(--primary)]"
                    : "border-[var(--border)] text-[var(--text-secondary)]"
            }`}
        >
            {label}
        </div>
    )
}

function InfoCard({ label, value }: { label: string; value: string }) {
    return (
        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-3">
            <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">{label}</div>
            <div className="mt-1 text-sm font-semibold text-[var(--text-primary)]">{value}</div>
        </div>
    )
}

function Stat({ label, value }: { label: string; value: string }) {
    return (
        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-3">
            <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">{label}</div>
            <div className="mt-1 text-lg font-semibold text-[var(--text-primary)]">{value}</div>
        </div>
    )
}
