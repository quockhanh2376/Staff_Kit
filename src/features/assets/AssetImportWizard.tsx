import { AlertCircle, FileSpreadsheet, PlusCircle, RefreshCw, Trash2, Upload } from "lucide-react"
import { Drawer } from "../../components/Drawer"
import { formatDate } from "../../lib/utils"
import {
    hasRequiredAssetImportMapping,
    type AssetImportState,
} from "./useAssetImportState"
import {
    getAssetImportFieldLabel,
    getAssetImportMappingKeys,
    getAssetImportModeLabel,
    getAssetImportReviewFieldKeys,
    getAssetImportRowFieldValue,
    getRequiredAssetImportMappingKeys,
    isAssetImportFieldEditable,
} from "./assetImportModeConfig"
import {
    getAssetImportReviewFilterLabel,
    getAssetImportSummaryLabel,
    getAssetImportStatusMeta,
} from "./assetImportStatusMeta"

type AssetImportWizardProps = {
    assetImport: AssetImportState
}

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
                            <div className="mt-2 text-[11px] text-[var(--text-secondary)]">
                                {getAssetImportModeLabel(detail.summary.importType)}
                            </div>
                            <div className="mt-3 grid gap-3 sm:grid-cols-2">
                                <Stat label={getAssetImportSummaryLabel("valid")} value={String(detail.summary.validRows)} />
                                <Stat label={getAssetImportSummaryLabel("errors")} value={String(detail.summary.errorRows)} />
                                <Stat label={getAssetImportSummaryLabel("imported")} value={String(detail.summary.importedRows)} />
                                <Stat label={getAssetImportSummaryLabel("skipped")} value={String(detail.summary.skippedRows)} />
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
    const stageBlockReason = inspection
        ? !assetImport.canStageCurrentMode
            ? "Map all required columns before staging a batch."
            : null
        : null

    return (
        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                <FileSpreadsheet size={16} />
                Choose File
            </div>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">
                Pick a `csv`, `xlsx`, or `xls` file, choose the import mode, then inspect headers before mapping.
            </p>
            <div className="mt-4 grid gap-3 md:grid-cols-2">
                {[
                    {
                        mode: "serialized" as const,
                        description:
                            "One asset per row. Category, asset name, serial number, warehouse, and note.",
                    },
                    {
                        mode: "quantity" as const,
                        description:
                            "Stock-style import. Item name, category, quantity, warehouse, and note.",
                    },
                ].map((option) => (
                    <button
                        key={option.mode}
                        className={`rounded-[12px] border p-3 text-left transition ${
                            assetImport.currentImportMode === option.mode
                                ? "border-[var(--primary)]/45 bg-[var(--primary)]/10"
                                : "border-[var(--border)] bg-[var(--surface-hover)]/20 hover:bg-[var(--surface-hover)]"
                        }`}
                        onClick={() => assetImport.setSelectedImportMode(option.mode)}
                        type="button"
                        disabled={Boolean(assetImport.activeBatchDetail)}
                    >
                        <div className="text-sm font-semibold text-[var(--text-primary)]">
                            {getAssetImportModeLabel(option.mode)}
                        </div>
                        <div className="mt-1 text-xs text-[var(--text-secondary)]">
                            {option.description}
                        </div>
                    </button>
                ))}
            </div>
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
                        onClick={() => assetImport.setCurrentStep("map_columns")}
                        type="button"
                        disabled={assetImport.isCreatingBatch || assetImport.isInspectingFile}
                    >
                        Continue to Mapping
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
                        <InfoCard label="Import Mode" value={getAssetImportModeLabel(assetImport.currentImportMode)} />
                    </div>
                    {stageBlockReason && (
                        <div className="rounded-[8px] border border-amber-400/35 bg-amber-400/10 px-3 py-2 text-xs text-amber-100">
                            {stageBlockReason}
                        </div>
                    )}
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

    const mappingKeys = getAssetImportMappingKeys(assetImport.currentImportMode)
    const requiredFieldKeySet = new Set<string>(
        getRequiredAssetImportMappingKeys(assetImport.currentImportMode),
    )
    const stageBlockReason = !assetImport.canStageCurrentMode
        ? "Map all required columns before staging a batch."
        : null

    return (
        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
            <div className="text-sm font-semibold text-[var(--text-primary)]">Map Columns</div>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">
                Required fields change with the selected mode. Once mapped, the batch stages into SQLite for review before any official import happens.
            </p>
            <div className="mt-4 grid gap-4 md:grid-cols-2">
                {mappingKeys.map((fieldKey) => (
                    <div key={fieldKey}>
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            {getAssetImportFieldLabel(fieldKey)}
                            {requiredFieldKeySet.has(fieldKey) ? " *" : ""}
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
            {stageBlockReason && (
                <div className="mt-4 rounded-[8px] border border-amber-400/35 bg-amber-400/10 px-3 py-2 text-xs text-amber-100">
                    {stageBlockReason}
                </div>
            )}
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
                    disabled={
                        !hasRequiredAssetImportMapping(
                            assetImport.mappingDraft,
                            assetImport.currentImportMode,
                        ) ||
                        assetImport.isCreatingBatch ||
                        !assetImport.canStageCurrentMode
                    }
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

    const reviewFieldKeys = getAssetImportReviewFieldKeys(assetImport.currentImportMode)
    const selectedRowStatusMeta = assetImport.selectedRow
        ? getAssetImportStatusMeta(assetImport.selectedRow.status)
        : null
    const reviewFilters = [
        { key: "all" as const },
        { key: "errors" as const },
        { key: "pending" as const },
    ]

    return (
        <div className="space-y-5">
            <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
                <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                        <div className="text-sm font-semibold text-[var(--text-primary)]">Review Batch</div>
                        <div className="mt-1 text-xs text-[var(--text-secondary)]">
                            {detail.summary.batchKey} | {formatDate(detail.summary.updatedAt)}
                        </div>
                        <div className="mt-2 inline-flex rounded-[999px] border border-[var(--border)] px-2.5 py-1 text-[10px] uppercase tracking-[0.08em] text-[var(--text-secondary)]">
                            {getAssetImportModeLabel(assetImport.currentImportMode)}
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
                            disabled={assetImport.isImportingRows || !assetImport.canImportCurrentBatch}
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
                    {reviewFilters.map((filter) => (
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
                            {getAssetImportReviewFilterLabel(filter.key)}
                        </button>
                    ))}
                </div>
                {assetImport.importBlockReason && (
                    <div className="mt-4 rounded-[8px] border border-amber-400/35 bg-amber-400/10 px-3 py-2 text-xs text-amber-100">
                        {assetImport.importBlockReason}
                    </div>
                )}
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
                                    {reviewFieldKeys.map((fieldKey) => (
                                        <th key={fieldKey} className="px-3 py-2">
                                            {getAssetImportFieldLabel(fieldKey)}
                                        </th>
                                    ))}
                                    <th className="px-3 py-2">Action</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[var(--border)]">
                                {assetImport.filteredRows.map((row) => {
                                    const statusMeta = getAssetImportStatusMeta(row.status)

                                    return (
                                        <tr
                                            key={row.id}
                                            className={assetImport.selectedRowId === row.id ? "bg-[var(--primary)]/6" : ""}
                                            onClick={() => assetImport.setSelectedRowId(row.id)}
                                        >
                                            <td className="px-3 py-2 align-top text-xs text-[var(--text-secondary)]">{row.rowNumber}</td>
                                            <td className="px-3 py-2 align-top">
                                                <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--text-primary)]">
                                                    {statusMeta.label}
                                                </span>
                                            </td>
                                            {reviewFieldKeys.map((fieldKey) => (
                                                <td key={`${row.id}-${fieldKey}`} className="px-2 py-2 align-top">
                                                    <input
                                                        className="form-input min-w-[130px] text-xs"
                                                        value={getAssetImportRowFieldValue(
                                                            row,
                                                            fieldKey,
                                                            assetImport.mappingDraft,
                                                        )}
                                                        disabled={
                                                            row.status === "imported" ||
                                                            assetImport.isUpdatingRow === row.id ||
                                                            !isAssetImportFieldEditable(fieldKey)
                                                        }
                                                        onChange={(event) => {
                                                            const nextValue = event.target.value
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
                                    )
                                })}
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
                                    Status: {selectedRowStatusMeta?.label}
                                </div>
                            </div>
                            <div>
                                <div className="mb-2 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                    Validation Errors
                                </div>
                                {assetImport.selectedRow.validationErrors.length === 0 ? (
                                    <div className="rounded-[8px] border border-emerald-500/35 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-300">
                                        {selectedRowStatusMeta?.emptyValidationMessage}
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
                                {getAssetImportModeLabel(summary.importType)}
                            </div>
                            <div className="mt-2 text-[11px] text-[var(--text-secondary)]">
                                {getAssetImportSummaryLabel("valid")} {summary.validRows} | {getAssetImportSummaryLabel("errors")} {summary.errorRows} | {getAssetImportSummaryLabel("imported")} {summary.importedRows}
                            </div>
                        </button>
                    ))
                )}
            </div>
        </div>
    )
}

function ManualAssetPanel({ assetImport }: AssetImportWizardProps) {
    const manualFields = [
        {
            key: "assetCode",
            label: "Asset Code",
            value: assetImport.manualAssetForm.assetCode,
            targetKey: "assetCode" as const,
            required: true,
        },
        {
            key: "category",
            label: getAssetImportFieldLabel("category"),
            value: assetImport.manualAssetForm.assetType,
            targetKey: "assetType" as const,
            required: true,
        },
        {
            key: "assetName",
            label: getAssetImportFieldLabel("assetName"),
            value: assetImport.manualAssetForm.displayName,
            targetKey: "displayName" as const,
            required: true,
        },
        {
            key: "model",
            label: getAssetImportFieldLabel("model"),
            value: assetImport.manualAssetForm.model,
            targetKey: "model" as const,
            required: false,
        },
        {
            key: "serialNumber",
            label: getAssetImportFieldLabel("serialNumber"),
            value: assetImport.manualAssetForm.serialNumber,
            targetKey: "serialNumber" as const,
            required: false,
        },
        {
            key: "note",
            label: getAssetImportFieldLabel("note"),
            value: assetImport.manualAssetForm.notes,
            targetKey: "notes" as const,
            required: false,
        },
    ] as const

    return (
        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                <PlusCircle size={16} />
                Quick Manual Add
            </div>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">
                Serialized-only fallback for one-off assets or rows that are not worth pushing through batch review.
            </p>
            <div className="mt-4 grid gap-4 md:grid-cols-2">
                {manualFields.map((field) => (
                    <div key={field.key}>
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            {field.label}
                            {field.required ? " *" : ""}
                        </label>
                        <input
                            className="form-input"
                            value={field.value}
                            onChange={(event) =>
                                assetImport.handleManualAssetFieldChange(field.targetKey, event.target.value)
                            }
                            placeholder={field.label}
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
