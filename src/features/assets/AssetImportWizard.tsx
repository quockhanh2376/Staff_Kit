import { FileSpreadsheet, RefreshCw } from "lucide-react"
import { Drawer } from "../../components/Drawer"
import { SharedImportShell } from "../import/sharedImportShell"
import type {
    AssetCondition,
    AssetDirectImportPreviewRow,
    AssetImportMode,
} from "../../types/staff"
import { ASSET_CONDITIONS, CONDITION_LABELS } from "./assetCondition"
import { AssetImportCategoryInput } from "./AssetImportCategoryInput"
import {
    getAssetImportManualPanelDescription,
    getAssetImportManualPanelPrimaryActionLabel,
    getAssetImportManualPanelTitle,
    getAssetImportPanelTitle,
} from "./assetImportCopy"
import { getAssetImportModeLabel } from "./assetImportModeConfig"
import { getAssetImportStatusMeta } from "./assetImportStatusMeta"
import type { AssetImportState } from "./useAssetDirectImportState"

type AssetImportWizardProps = {
    assetImport: AssetImportState
}

export function AssetImportWizard({ assetImport }: AssetImportWizardProps) {
    return (
        <Drawer
            open={assetImport.isWizardOpen}
            onClose={assetImport.closeWizard}
            title={getAssetImportPanelTitle(assetImport.panelMode)}
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
    const inspection = assetImport.inspection
    const preview = assetImport.preview
    const report = assetImport.report
    const isSerialized = assetImport.selectedImportMode === "serialized"
    const pending = assetImport.staging.pendingBatch

    return (
        <SharedImportShell
            fileSectionTitle="Choose File"
            fileSectionDescription="Pick a CSV or Excel file. Staff Kit auto-detects known columns, previews valid/error rows, then imports valid rows after approval."
            selectedFiles={assetImport.selectedFiles}
            emptyFilesLabel="No file selected yet."
            chooseButtonLabel="Choose File"
            chooseButtonBusyLabel="Inspecting..."
            onChooseFiles={() => void assetImport.handlePickImportFile()}
            isChoosingFiles={assetImport.isInspectingFile}
            primaryActionLabel={isSerialized ? "Load to staging" : "Preview Import"}
            primaryActionBusyLabel={isSerialized ? "Loading..." : "Preparing Preview..."}
            onPrimaryAction={() =>
                void (isSerialized ? assetImport.handleLoadStaging() : assetImport.handlePreviewImport())
            }
            isPrimaryActionDisabled={!assetImport.canPreviewCurrentFile}
            isPrimaryActionBusy={isSerialized ? assetImport.staging.isBusy : assetImport.isPreviewingImport}
            onClose={assetImport.closeWizard}
            preview={
                preview
                    ? {
                          title: `Approve ${getAssetImportModeLabel(preview.importType)}`,
                          summaryItems: [
                              { label: "Total Rows", value: preview.totalRows },
                              { label: "New Rows", value: preview.validRows },
                              { label: "Existing / Skipped", value: preview.skippedRows },
                              { label: "Condition Changes", value: preview.conditionUpdateRows },
                              { label: "Error Rows", value: preview.errorRows },
                              ...(preview.newCategories?.map((cat) => ({
                                  label: `New Category: ${cat.categoryName}`,
                                  value: `${cat.assetCount} assets`,
                              })) ?? []),
                          ],
                          rows: preview.rows,
                          errors: preview.errors,
                          renderRow: (row) => (
                              <PreviewRowCard
                                  key={`${row.rowNumber}-${row.assetCode ?? row.displayName ?? "row"}`}
                                  importType={preview.importType}
                                  row={row}
                                  onConditionChange={(condition) =>
                                      void assetImport.handlePreviewConditionChange(row.rowNumber, condition)
                                  }
                                  conditionDisabled={assetImport.isPreviewingImport}
                              />
                          ),
                          onCancel: assetImport.handleCancelPreview,
                          onApprove: () => void assetImport.handleApproveImport(),
                          approveDisabled: assetImport.previewApproveDisabled,
                          isApproving: assetImport.isApprovingImport,
                      }
                    : null
            }
            report={
                report
                    ? {
                          title: "Import Complete",
                          summaryItems: [
                              { label: "Imported", value: report.imported },
                              {
                                  label:
                                      report.importType === "serialized"
                                          ? "Existing / Skipped"
                                          : "Skipped",
                                  value: report.skipped,
                              },
                              { label: "Failed", value: report.failed },
                              ...(report.importType === "serialized"
                                  ? [{ label: "Condition Updated", value: report.conditionUpdatedRows }]
                                  : []),
                          ],
                          errors: report.errors,
                      }
                    : null
            }
        >
            <div className="space-y-5">
                {assetImport.statusMessage && (
                    <div className="rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                        {assetImport.statusMessage}
                    </div>
                )}

                {pending && (
                    <div className="flex flex-wrap items-center gap-2 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-2 text-xs text-[var(--text-primary)]">
                        <span className="mr-auto">{`A staged import is waiting: ${pending.sourceFileName} (${pending.totalRows} rows).`}</span>
                        <button
                            type="button"
                            className="rounded-[6px] border border-[var(--border)] px-2 py-1 font-semibold hover:bg-[var(--surface)] disabled:opacity-60"
                            disabled={assetImport.staging.isBusy}
                            onClick={() => {
                                if (window.confirm("Discard the staged import? Its edits are lost and nothing is written.")) {
                                    void assetImport.staging.discard()
                                }
                            }}
                        >
                            Discard
                        </button>
                        <button
                            type="button"
                            className="rounded-[6px] border border-[var(--primary)] px-2 py-1 font-semibold text-[var(--primary)] hover:bg-[var(--primary)]/15 disabled:opacity-60"
                            disabled={assetImport.staging.isBusy}
                            onClick={() => void assetImport.handleContinueStaging()}
                        >
                            Continue
                        </button>
                    </div>
                )}

                <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-3.5 text-xs text-[var(--text-secondary)]">
                    One file for every asset. Each row is Serial or Quantity by its Category in Staff Kit; a new
                    Category follows the file (Serial Number → Serial, Quantity → Quantity). You can change the Type
                    of any row in staging before Approve.
                </div>

                <div className="rounded-[12px] border border-emerald-500/25 bg-emerald-500/5 p-3.5 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
                    <div className="space-y-0.5">
                        <div className="flex items-center gap-2 text-xs font-bold text-emerald-400">
                            <RefreshCw size={14} />
                            Update Existing Asset Metadata Mode
                        </div>
                        <p className="text-[11px] text-[var(--text-secondary)]">
                            Enrich existing Monitor records with canonical Brand, Model, and Asset Name from Excel without skipping or altering assignments.
                        </p>
                    </div>
                    <button
                        type="button"
                        className="btn btn-secondary shrink-0 text-xs text-emerald-400 hover:text-emerald-300 hover:border-emerald-500/50"
                        onClick={assetImport.openMetadataUpdate}
                    >
                        Launch Metadata Reimport →
                    </button>
                </div>

                {inspection && (
                    <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
                        <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                            <FileSpreadsheet size={16} />
                            File Inspection
                        </div>
                        <div className="mt-4 grid gap-3 md:grid-cols-2">
                            <InfoCard label="File" value={inspection.fileName} />
                            <InfoCard label="Type" value={inspection.fileType.toUpperCase()} />
                            <InfoCard
                                label="Header Row"
                                value={String(inspection.headerRow)}
                            />
                            <InfoCard
                                label="Import Mode"
                                value={getAssetImportModeLabel("auto")}
                            />
                        </div>

                        {inspection.availableSheets.length > 1 && (
                            <div className="mt-4">
                                <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                    Excel Sheet
                                </label>
                                <select
                                    className="form-input"
                                    value={inspection.selectedSheetName ?? ""}
                                    onChange={(event) =>
                                        void assetImport.handleChangeSelectedSheet(
                                            event.target.value,
                                        )
                                    }
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

                        <div className="mt-4 flex flex-wrap gap-2">
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
        </SharedImportShell>
    )
}

function PreviewRowCard({
    importType,
    row,
    onConditionChange,
    conditionDisabled,
}: {
    importType: AssetImportMode
    row: AssetDirectImportPreviewRow
    onConditionChange: (condition: AssetCondition) => void
    conditionDisabled: boolean
}) {
    const cells =
        importType === "quantity"
            ? [
                  { label: "Category", value: row.assetType ?? "—" },
                  { label: "Item Name", value: row.displayName ?? "—" },
                  { label: "Model", value: row.model ?? "—" },
                  { label: "Quantity", value: row.quantity ?? "—" },
                  { label: "Note", value: row.notes ?? "—" },
              ]
            : [
                  { label: "Category", value: row.assetType ?? "—" },
                  { label: "Computer Name", value: row.computerName ?? "—" },
                  { label: "Asset Name", value: row.displayName ?? "—" },
                  { label: "Model", value: row.model ?? "—" },
                  { label: "Serial Number", value: row.serialNumber ?? "—" },
                  { label: "Adapter Number", value: row.adapterNumber ?? "—" },
                  { label: "Usage Location", value: row.usageLocation ?? "—" },
                  { label: "Holder", value: row.holderLabel ?? "—" },
                  { label: "Note", value: row.notes ?? "—" },
              ]

    const statusMeta = getAssetImportStatusMeta(row.status)
    const reviewNotes = row.reviewNotes

    return (
        <div
            className={`rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-4 ${
                reviewNotes.length > 0 ? "search-highlight-cell" : ""
            }`}
            data-testid={`asset-preview-row-${row.rowNumber}`}
        >
            <div className="flex items-start justify-between gap-3">
                <div>
                    <div className="text-sm font-semibold text-[var(--text-primary)]">
                        Row {row.rowNumber}
                        {row.assetCode ? ` · ${row.assetCode}` : ""}
                    </div>
                    <div className="mt-1 text-xs text-[var(--text-secondary)]">
                        {row.displayName || row.computerName || "No title"}
                    </div>
                </div>
                <div className="flex items-center gap-2">
                    {importType === "serialized" && (
                        <select
                            aria-label={`Condition row ${row.rowNumber}`}
                            className="rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1 text-xs font-semibold text-[var(--text-primary)] disabled:opacity-50"
                            disabled={conditionDisabled}
                            onChange={(event) => onConditionChange(event.target.value as AssetCondition)}
                            value={row.condition ?? "spare"}
                        >
                            {ASSET_CONDITIONS.map((condition) => (
                                <option key={condition} value={condition}>
                                    {CONDITION_LABELS[condition]}
                                </option>
                            ))}
                        </select>
                    )}
                    <span
                        className={`rounded-[999px] border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.06em] ${
                            row.status === "error"
                                ? "border-red-500/40 bg-red-500/10 text-red-200"
                                : row.status === "skipped"
                                    ? "border-slate-500/40 bg-slate-500/10 text-slate-200"
                                    : "border-emerald-500/40 bg-emerald-500/10 text-emerald-200"
                        }`}
                    >
                        {statusMeta.label}
                    </span>
                </div>
            </div>

            <div className="mt-3 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {cells.map((cell) => (
                    <div key={cell.label}>
                        <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            {cell.label}
                        </div>
                        <div className="mt-1 text-sm text-[var(--text-primary)]">
                            {cell.value}
                        </div>
                    </div>
                ))}
            </div>

            {reviewNotes.length > 0 && (
                <ul className="mt-3 list-disc space-y-0.5 pl-5 text-xs font-medium text-[var(--text-primary)]">
                    {reviewNotes.map((note) => (
                        <li key={note}>{note}</li>
                    ))}
                </ul>
            )}

            {row.validationErrors.length > 0 && (
                <div className="mt-3 rounded-[8px] border border-red-500/35 bg-red-500/10 px-3 py-2 text-xs text-red-100">
                    {row.validationErrors.join(" | ")}
                </div>
            )}
        </div>
    )
}

function ManualAssetPanel({ assetImport }: AssetImportWizardProps) {
    return (
        <div className="space-y-5">
            <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-5">
                <div className="text-sm font-semibold text-[var(--text-primary)]">
                    {getAssetImportManualPanelTitle()}
                </div>
                <p className="mt-2 text-sm text-[var(--text-secondary)]">
                    {getAssetImportManualPanelDescription()}
                </p>

                <div className="mt-4 grid gap-4 md:grid-cols-2">
                    <div>
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            Asset Code *
                        </label>
                        <input
                            className="form-input"
                            value={assetImport.manualAssetForm.assetCode}
                            onChange={(event) =>
                                assetImport.handleManualAssetFieldChange(
                                    "assetCode",
                                    event.target.value,
                                )
                            }
                            placeholder="VNLAP235"
                        />
                    </div>

                    <div>
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            Category *
                        </label>
                        <AssetImportCategoryInput
                            assetCategories={assetImport.assetCategories}
                            disabled={assetImport.isLoadingCategories}
                            mode="serialized"
                            onChange={(nextValue) =>
                                assetImport.handleManualAssetFieldChange("assetType", nextValue)
                            }
                            value={assetImport.manualAssetForm.assetType}
                        />
                    </div>

                    <div className="md:col-span-2">
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            Asset Name *
                        </label>
                        <input
                            className="form-input"
                            value={assetImport.manualAssetForm.displayName}
                            onChange={(event) =>
                                assetImport.handleManualAssetFieldChange(
                                    "displayName",
                                    event.target.value,
                                )
                            }
                            placeholder="Dell Latitude 7440"
                        />
                    </div>

                    <div>
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            Model
                        </label>
                        <input
                            className="form-input"
                            value={assetImport.manualAssetForm.model}
                            onChange={(event) =>
                                assetImport.handleManualAssetFieldChange("model", event.target.value)
                            }
                            placeholder="7440"
                        />
                    </div>

                    <div>
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            Serial Number
                        </label>
                        <input
                            className="form-input"
                            value={assetImport.manualAssetForm.serialNumber}
                            onChange={(event) =>
                                assetImport.handleManualAssetFieldChange(
                                    "serialNumber",
                                    event.target.value,
                                )
                            }
                            placeholder="SN-001"
                        />
                    </div>

                    <div>
                        <label
                            className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]"
                            htmlFor="manual-asset-condition"
                        >
                            Condition
                        </label>
                        <select
                            className="form-input"
                            id="manual-asset-condition"
                            value={assetImport.manualAssetForm.condition}
                            onChange={(event) =>
                                assetImport.handleManualAssetFieldChange("condition", event.target.value)
                            }
                        >
                            <option value="new">{CONDITION_LABELS.new}</option>
                            <option value="spare">{CONDITION_LABELS.spare}</option>
                        </select>
                    </div>

                    <div className="md:col-span-2">
                        <label className="mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            Note
                        </label>
                        <textarea
                            className="form-input min-h-[96px]"
                            value={assetImport.manualAssetForm.notes}
                            onChange={(event) =>
                                assetImport.handleManualAssetFieldChange("notes", event.target.value)
                            }
                            placeholder="Optional note"
                        />
                    </div>
                </div>

                {assetImport.manualAssetMessage && (
                    <div className="mt-4 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                        {assetImport.manualAssetMessage}
                    </div>
                )}

                {assetImport.manualAssetResult && (
                    <div className="mt-4 rounded-[8px] border border-emerald-500/35 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-100">
                        Saved {assetImport.manualAssetResult.assetCode} successfully.
                    </div>
                )}

                <div className="mt-4 flex gap-3 border-t border-[var(--border)] pt-4">
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
                        {getAssetImportManualPanelPrimaryActionLabel(
                            assetImport.isCreatingManualAsset,
                        )}
                    </button>
                </div>
            </div>
        </div>
    )
}

function InfoCard({ label, value }: { label: string; value: string }) {
    return (
        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 px-3 py-3">
            <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                {label}
            </div>
            <div className="mt-1 text-sm font-medium text-[var(--text-primary)]">{value}</div>
        </div>
    )
}
