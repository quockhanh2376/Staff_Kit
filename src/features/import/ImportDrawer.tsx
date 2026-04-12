import { Drawer } from "../../components/Drawer"
import { SharedImportShell } from "./sharedImportShell"
import type { ImportState } from "./useImportState"

type ImportDrawerProps = {
    importState: ImportState
}

export function ImportDrawer({ importState }: ImportDrawerProps) {
    const imp = importState

    const preview = imp.importPreviewResult
        ? {
              title: "Preview Import Changes",
              summaryItems: [
                  { label: "New Employees", value: imp.importPreviewResult.totalNew },
                  { label: "Updates", value: imp.importPreviewResult.totalUpdates },
                  { label: "Field Changes", value: imp.importPreviewResult.totalChanges },
                  { label: "Total Rows", value: imp.importPreviewResult.previewRows.length },
              ],
              rows: imp.importPreviewResult.previewRows,
              errors: imp.importPreviewResult.errors.map((item) => ({
                  rowNumber: item.row,
                  entityKey: item.employeeId,
                  reason: item.reason,
              })),
              renderRow: (row: (typeof imp.importPreviewResult.previewRows)[number], index: number) => (
                  <div key={`${row.employeeId}-${index}`} className="rounded border border-[var(--border)] p-3">
                      <div className="flex items-center gap-2">
                          <span className="font-medium">{row.employeeId}</span>
                          <span className="text-sm">{row.fullName}</span>
                          <span
                              className={`rounded px-2 py-1 text-xs ${
                                  row.isUpdate
                                      ? "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200"
                                      : "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
                              }`}
                          >
                              {row.isUpdate ? "Update" : "New"}
                          </span>
                      </div>
                      {row.changes.length > 0 && (
                          <div className="mt-2 text-sm text-[var(--text-secondary)]">
                              {row.changes.map((change, changeIndex) => (
                                  <div key={changeIndex} className="ml-6 py-1">
                                      <span className="font-medium">{change.fieldLabel}:</span>
                                      <span className="text-[var(--text-secondary)] line-through">
                                          {change.oldValue || "-"}
                                      </span>
                                      <span className="mx-2">-&gt;</span>
                                      <span className="font-medium text-green-600 dark:text-green-400">
                                          {change.newValue || "-"}
                                      </span>
                                  </div>
                              ))}
                          </div>
                      )}
                  </div>
              ),
              onCancel: imp.handleRejectPreviewRows,
              onApprove: () => void imp.handleApprovePreviewRows(),
              approveDisabled: imp.isImporting,
              isApproving: imp.isImporting,
          }
        : null

    const report = imp.importReport
        ? {
              title: "Import completed",
              summaryItems: [
                  { label: "Main file", value: imp.importReport.sourceFile },
                  { label: "Files", value: imp.importReport.sourceFiles.length },
                  { label: "Sheet", value: imp.importReport.sheetName },
                  {
                      label: "Processed sheets",
                      value: imp.importReport.processedSheets.join(", ") || "-",
                  },
                  { label: "Header row", value: imp.importReport.headerRow },
                  { label: "Total rows", value: imp.importReport.totalRows },
                  { label: "Inserted", value: imp.importReport.inserted },
                  { label: "Updated", value: imp.importReport.updated },
                  { label: "Skipped", value: imp.importReport.skipped },
                  { label: "Failed", value: imp.importReport.failed },
              ],
              errors: imp.importReport.errors.map((item) => ({
                  rowNumber: item.row,
                  entityKey: item.employeeId,
                  reason: item.reason,
              })),
          }
        : null

    return (
        <Drawer
            open={imp.isImportDrawerOpen}
            onClose={() => imp.setImportDrawerOpen(false)}
            title="Import from Excel"
            widthClass="w-[560px]"
        >
            <SharedImportShell
                fileSectionTitle="Step 1: Select Excel files"
                fileSectionDescription={`You can select one or multiple files. The app will detect Staff ID and merge data by employee. Import target group: ${imp.importTargetGroupLabel}`}
                selectedFiles={imp.importSelectedFiles}
                emptyFilesLabel="No files selected."
                chooseButtonLabel="Choose File(s)"
                chooseButtonBusyLabel="Preparing"
                onChooseFiles={() => void imp.handlePickImportFiles()}
                isChoosingFiles={imp.isImporting}
                primaryActionLabel="Preview Import"
                primaryActionBusyLabel="Preparing Preview..."
                onPrimaryAction={() => void imp.handleImportSelectedColumns()}
                isPrimaryActionDisabled={
                    imp.importSelectedFiles.length === 0 || imp.importColumnOptions.length === 0
                }
                isPrimaryActionBusy={imp.isImporting && !imp.showImportPreviewModal}
                onClose={() => imp.setImportDrawerOpen(false)}
                preview={preview}
                report={report}
            >
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
                                            <div className="text-sm text-[var(--text-primary)]">
                                                {column.label}
                                            </div>
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
            </SharedImportShell>
        </Drawer>
    )
}
