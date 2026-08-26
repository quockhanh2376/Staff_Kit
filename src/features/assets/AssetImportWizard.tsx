import { FileSpreadsheet } from "lucide-react"
import { Drawer } from "../../components/Drawer"
import { SharedImportShell } from "../import/sharedImportShell"
import type {
    AssetDirectImportPreviewRow,
    AssetImportMode,
} from "../../types/staff"
import { AssetImportCategoryInput } from "./AssetImportCategoryInput"
import {
    getAssetImportManualPanelDescription,
    getAssetImportManualPanelPrimaryActionLabel,
    getAssetImportManualPanelTitle,
    getAssetImportPanelTitle,
    getAssetImportSerializedModeDescription,
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
            primaryActionLabel="Preview Import"
            primaryActionBusyLabel="Preparing Preview..."
            onPrimaryAction={() => void assetImport.handlePreviewImport()}
            isPrimaryActionDisabled={!assetImport.canPreviewCurrentFile}
            isPrimaryActionBusy={assetImport.isPreviewingImport}
            onClose={assetImport.closeWizard}
            preview={
                preview
                    ? {
                          title: `Approve ${getAssetImportModeLabel(preview.importType)}`,
                          summaryItems: [
                              { label: "Total Rows", value: preview.totalRows },
                              { label: "New Rows", value: preview.validRows },
                              { label: "Existing / Skipped", value: preview.skippedRows },
                              { label: "Error Rows", value: preview.errorRows },
                          ],
                          rows: preview.rows,
                          errors: preview.errors,
                          renderRow: (row) => (
                              <PreviewRowCard
                                  key={`${row.rowNumber}-${row.assetCode ?? row.displayName ?? "row"}`}
                                  importType={preview.importType}
                                  row={row}
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

                <div className="grid gap-3 md:grid-cols-2">
                    <ModeCard
                        mode="serialized"
                        active={assetImport.selectedImportMode === "serialized"}
                        description={getAssetImportSerializedModeDescription()}
                        onClick={() => assetImport.setSelectedImportMode("serialized")}
                    />
                    <ModeCard
                        mode="quantity"
                        active={assetImport.selectedImportMode === "quantity"}
                        description="Import quantity stock rows directly into stock records after preview and approval."
                        onClick={() => assetImport.setSelectedImportMode("quantity")}
                    />
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
                                value={getAssetImportModeLabel(assetImport.selectedImportMode)}
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

function ModeCard({
    active,
    description,
    mode,
    onClick,
}: {
    active: boolean
    description: string
    mode: AssetImportMode
    onClick: () => void
}) {
    return (
        <button
            className={`rounded-[12px] border p-3 text-left transition ${
                active
                    ? "border-[var(--primary)]/45 bg-[var(--primary)]/10"
                    : "border-[var(--border)] bg-[var(--surface-hover)]/20 hover:bg-[var(--surface-hover)]"
            }`}
            onClick={onClick}
            type="button"
        >
            <div className="text-sm font-semibold text-[var(--text-primary)]">
                {getAssetImportModeLabel(mode)}
            </div>
            <div className="mt-1 text-xs text-[var(--text-secondary)]">{description}</div>
        </button>
    )
}

function PreviewRowCard({
    importType,
    row,
}: {
    importType: AssetImportMode
    row: AssetDirectImportPreviewRow
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

    return (
        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-4">
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
