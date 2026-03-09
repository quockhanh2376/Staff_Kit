import { LoaderCircle, Upload } from "lucide-react"
import { Drawer } from "../../components/Drawer"
import type { ImportState } from "./useImportState"

type ImportDrawerProps = {
    importState: ImportState
}

export function ImportDrawer({ importState }: ImportDrawerProps) {
    const imp = importState

    return (
        <>
            <Drawer
                open={imp.isImportDrawerOpen}
                onClose={() => imp.setImportDrawerOpen(false)}
                title="Import from Excel"
                widthClass="w-[560px]"
            >
                <div className="space-y-5">
                    {/* Step 1: File selection */}
                    <div className="rounded-[8px] border border-dashed border-[var(--border)] bg-[var(--surface-hover)]/35 p-4">
                        <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                            <Upload size={16} className="text-[var(--primary)]" />
                            Step 1: Select Excel files
                        </div>
                        <div className="mt-2 text-xs text-[var(--text-secondary)]">
                            You can select one or multiple files. The app will detect Staff ID and merge data by employee.
                        </div>
                        <div className="mt-2 text-xs text-[var(--text-secondary)]">
                            Import target group:{" "}
                            <span className="font-semibold text-[var(--text-primary)]">{imp.importTargetGroupLabel}</span>
                        </div>
                        <div className="mt-3 space-y-2 text-xs text-[var(--text-secondary)]">
                            {imp.importSelectedFiles.length === 0 ? (
                                <div className="rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-2">
                                    No files selected.
                                </div>
                            ) : (
                                imp.importSelectedFiles.map((file) => (
                                    <div
                                        key={file}
                                        className="break-all rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-2"
                                    >
                                        {file}
                                    </div>
                                ))
                            )}
                        </div>
                    </div>

                    {/* Step 2: Column selection */}
                    {imp.importColumnOptions.length > 0 && (
                        <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-4">
                            <div className="text-sm font-semibold text-[var(--text-primary)]">
                                Step 2: Choose columns to import
                            </div>
                            <div className="mt-2 flex items-center gap-2">
                                <button
                                    className="rounded-[8px] border border-[var(--border)] px-2.5 py-1.5 text-xs font-medium"
                                    onClick={imp.selectAllOptionalImportColumns}
                                    type="button"
                                >
                                    Select All Optional
                                </button>
                                <button
                                    className="rounded-[8px] border border-[var(--border)] px-2.5 py-1.5 text-xs font-medium"
                                    onClick={imp.clearOptionalImportColumns}
                                    type="button"
                                >
                                    Clear Optional
                                </button>
                                <div className="ml-auto text-xs text-[var(--text-secondary)]">
                                    Selected: {imp.effectiveImportColumnKeySet.size}/{imp.importColumnOptions.length}
                                </div>
                            </div>

                            <div className="mt-3 max-h-64 space-y-2 overflow-auto pr-1">
                                {imp.importColumnOptions.map((column) => {
                                    const selected = imp.effectiveImportColumnKeySet.has(column.key)
                                    const disabled = column.required
                                    return (
                                        <label
                                            key={column.key}
                                            className="flex items-center gap-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2"
                                        >
                                            <input
                                                type="checkbox"
                                                checked={selected}
                                                disabled={disabled}
                                                onChange={() => imp.toggleImportColumn(column)}
                                            />
                                            <div className="flex-1">
                                                <div className="text-sm text-[var(--text-primary)]">{column.label}</div>
                                                <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                                    {column.source}
                                                </div>
                                            </div>
                                            {column.required && (
                                                <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                                    required
                                                </span>
                                            )}
                                        </label>
                                    )
                                })}
                            </div>
                        </div>
                    )}

                    {/* Action buttons */}
                    <div className="flex flex-wrap items-center gap-2 border-t border-[var(--border)] pt-4">
                        <button
                            className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium"
                            onClick={() => void imp.handlePickImportFiles()}
                            type="button"
                            disabled={imp.isImporting}
                        >
                            {imp.isImporting ? (
                                <span className="inline-flex items-center gap-2">
                                    <LoaderCircle className="animate-spin" size={14} />
                                    Preparing
                                </span>
                            ) : (
                                "Choose File(s)"
                            )}
                        </button>
                        <button
                            className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                            onClick={() => void imp.handleImportSelectedColumns()}
                            type="button"
                            disabled={
                                imp.isImporting ||
                                imp.importSelectedFiles.length === 0 ||
                                imp.importColumnOptions.length === 0
                            }
                        >
                            {imp.isImporting ? "Importing..." : "Import to App"}
                        </button>
                        <button
                            className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium"
                            onClick={() => imp.setImportDrawerOpen(false)}
                            type="button"
                        >
                            Close
                        </button>
                    </div>

                    {/* Import report */}
                    {imp.importReport && (
                        <div className="space-y-3 rounded-[8px] border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
                            <div className="font-semibold">Import completed</div>
                            <div className="grid grid-cols-2 gap-2 text-xs">
                                <div>Main file: {imp.importReport.sourceFile}</div>
                                <div>Files: {imp.importReport.sourceFiles.length}</div>
                                <div>Sheet: {imp.importReport.sheetName}</div>
                                <div>Processed sheets: {imp.importReport.processedSheets.join(", ") || "-"}</div>
                                <div>Header row: {imp.importReport.headerRow}</div>
                                <div>Total rows: {imp.importReport.totalRows}</div>
                                <div>Inserted: {imp.importReport.inserted}</div>
                                <div>Updated: {imp.importReport.updated}</div>
                                <div>Skipped: {imp.importReport.skipped}</div>
                                <div>Failed: {imp.importReport.failed}</div>
                            </div>
                            {imp.importReport.errors.length > 0 && (
                                <div className="max-h-44 overflow-auto rounded-[6px] border border-emerald-500/30 bg-black/20 p-2 text-xs">
                                    {imp.importReport.errors.slice(0, 12).map((item, index) => (
                                        <div key={`${item.row}-${index}`}>
                                            Row {item.row}
                                            {item.employeeId ? ` (${item.employeeId})` : ""}: {item.reason}
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    )}
                </div>
            </Drawer>

            {/* Import Preview Modal */}
            {imp.showImportPreviewModal && imp.importPreviewResult && (
                <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div className="max-h-[90vh] w-full max-w-4xl overflow-auto rounded-lg border border-[var(--border)] bg-[var(--bg)] p-6 shadow-xl">
                        <h2 className="mb-4 text-lg font-semibold">Review Import Changes</h2>

                        <div className="mb-6 flex gap-6 rounded-md border border-[var(--border)] bg-[var(--bg-secondary)] p-4">
                            <div>
                                <div className="text-sm text-[var(--text-secondary)]">New Employees</div>
                                <div className="text-xl font-bold">{imp.importPreviewResult.totalNew}</div>
                            </div>
                            <div>
                                <div className="text-sm text-[var(--text-secondary)]">Updates</div>
                                <div className="text-xl font-bold">{imp.importPreviewResult.totalUpdates}</div>
                            </div>
                            <div>
                                <div className="text-sm text-[var(--text-secondary)]">Field Changes</div>
                                <div className="text-xl font-bold">{imp.importPreviewResult.totalChanges}</div>
                            </div>
                            <div>
                                <div className="text-sm text-[var(--text-secondary)]">Total Rows</div>
                                <div className="text-xl font-bold">{imp.importPreviewResult.previewRows.length}</div>
                            </div>
                        </div>

                        <div className="mb-6 max-h-[50vh] space-y-3 overflow-y-auto">
                            {imp.importPreviewResult.previewRows.map((row, idx) => (
                                <div key={idx} className="flex gap-3 rounded border border-[var(--border)] p-3">
                                    <input
                                        type="checkbox"
                                        checked={imp.selectedImportRowIndices.has(idx)}
                                        onChange={() => imp.togglePreviewRowSelection(idx)}
                                        className="mt-1"
                                    />
                                    <div className="flex-1">
                                        <div className="flex items-center gap-2">
                                            <span className="font-medium">{row.employeeId}</span>
                                            <span className="text-sm">{row.fullName}</span>
                                            <span
                                                className={`rounded px-2 py-1 text-xs ${row.isUpdate
                                                        ? "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200"
                                                        : "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
                                                    }`}
                                            >
                                                {row.isUpdate ? "Update" : "New"}
                                            </span>
                                        </div>
                                        {row.changes.length > 0 && (
                                            <div className="mt-2 text-sm text-[var(--text-secondary)]">
                                                {row.changes.map((change, chIdx) => (
                                                    <div key={chIdx} className="ml-6 py-1">
                                                        <span className="font-medium">{change.fieldLabel}:</span>
                                                        <span className="text-[var(--text-secondary)] line-through">
                                                            {change.oldValue || "-"}
                                                        </span>
                                                        <span className="mx-2">→</span>
                                                        <span className="font-medium text-green-600 dark:text-green-400">
                                                            {change.newValue || "-"}
                                                        </span>
                                                    </div>
                                                ))}
                                            </div>
                                        )}
                                    </div>
                                </div>
                            ))}
                        </div>

                        {imp.importPreviewResult.errors.length > 0 && (
                            <div className="mb-6 rounded border border-red-200 bg-red-50 p-3 dark:border-red-800 dark:bg-red-950">
                                <div className="text-sm font-medium text-red-800 dark:text-red-200">
                                    Errors ({imp.importPreviewResult.errors.length})
                                </div>
                                {imp.importPreviewResult.errors.slice(0, 5).map((err, idx) => (
                                    <div key={idx} className="text-xs text-red-700 dark:text-red-300">
                                        Row {err.row}: {err.reason}
                                    </div>
                                ))}
                            </div>
                        )}

                        <div className="flex gap-3 border-t border-[var(--border)] pt-4">
                            <button
                                className="flex-1 rounded-md border border-[var(--border)] px-4 py-2 text-sm font-medium hover:bg-[var(--bg-secondary)]"
                                onClick={imp.handleRejectPreviewRows}
                                disabled={imp.isImporting}
                            >
                                Cancel
                            </button>
                            <button
                                className="rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-50"
                                onClick={() => imp.setSelectedImportRowIndices(new Set())}
                                disabled={imp.selectedImportRowIndices.size === 0 || imp.isImporting}
                            >
                                Deselect All
                            </button>
                            <button
                                className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
                                onClick={() =>
                                    imp.setSelectedImportRowIndices(
                                        new Set(imp.importPreviewResult!.previewRows.map((_, idx) => idx)),
                                    )
                                }
                                disabled={
                                    imp.selectedImportRowIndices.size === imp.importPreviewResult.previewRows.length ||
                                    imp.isImporting
                                }
                            >
                                Select All
                            </button>
                            <button
                                className="rounded-md bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:opacity-50"
                                onClick={() => void imp.handleApprovePreviewRows()}
                                disabled={imp.selectedImportRowIndices.size === 0 || imp.isImporting}
                            >
                                {imp.isImporting ? "Importing..." : `Approve (${imp.selectedImportRowIndices.size})`}
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </>
    )
}
