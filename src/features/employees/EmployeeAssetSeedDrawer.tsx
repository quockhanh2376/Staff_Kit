import { Drawer } from "../../components/Drawer"
import { SharedImportShell } from "../import/sharedImportShell"
import type { EmployeeAssetSeedState } from "./useEmployeeAssetSeedState"

type EmployeeAssetSeedDrawerProps = {
    seedState: EmployeeAssetSeedState
}

export function EmployeeAssetSeedDrawer({ seedState }: EmployeeAssetSeedDrawerProps) {
    const seed = seedState

    const preview = seed.preview
        ? {
              title: "Preview Employee Asset Import",
              summaryItems: [
                  { label: "Matched Employees", value: seed.preview.totalRows },
                  { label: "Valid Assets", value: seed.preview.validRows },
                  { label: "Error Rows", value: seed.preview.errorRows },
              ],
              rows: seed.preview.rows,
              errors: seed.preview.errors,
              renderRow: (row: (typeof seed.preview.rows)[number], index: number) => (
                  <div
                      key={`${row.employeeId}-${index}`}
                      className="rounded border border-[var(--border)] bg-[var(--surface)] p-3"
                  >
                      <div className="flex items-center gap-2">
                          <span className="font-medium">{row.fullName}</span>
                          <span className="text-sm text-[var(--text-secondary)]">{row.employeeId}</span>
                          <span
                              className={`rounded px-2 py-1 text-xs ${
                                  row.status === "valid"
                                      ? "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
                                      : "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
                              }`}
                          >
                              {row.status}
                          </span>
                      </div>
                      <div className="mt-2 grid gap-2 text-sm text-[var(--text-secondary)] md:grid-cols-2">
                          <div>
                              <span className="font-medium text-[var(--text-primary)]">Source:</span>{" "}
                              {row.sourceComputerName}
                          </div>
                          <div>
                              <span className="font-medium text-[var(--text-primary)]">Asset Code:</span>{" "}
                              {row.assetCode ?? "-"}
                          </div>
                          <div>
                              <span className="font-medium text-[var(--text-primary)]">Computer Name:</span>{" "}
                              {row.computerName ?? "-"}
                          </div>
                          <div>
                              <span className="font-medium text-[var(--text-primary)]">Category:</span>{" "}
                              {row.categoryName ?? row.categoryCode ?? "-"}
                          </div>
                      </div>
                  </div>
              ),
              onCancel: seed.handleCancelPreview,
              onApprove: () => void seed.handleApprove(),
              approveDisabled: seed.previewApproveDisabled,
              isApproving: seed.isImporting,
          }
        : null

    const report = seed.report
        ? {
              title: "Employee asset import completed",
              summaryItems: [
                  { label: "Imported", value: seed.report.imported },
                  { label: "Skipped", value: seed.report.skipped },
                  { label: "Failed", value: seed.report.failed },
                  { label: "Asset Codes", value: seed.report.importedAssetCodes.length },
              ],
              errors: seed.report.errors,
          }
        : null

    return (
        <Drawer
            open={seed.isDrawerOpen}
            onClose={seed.closeDrawer}
            title="Create Assets from Employee List"
            widthClass="w-[640px]"
        >
            <SharedImportShell
                fileSectionTitle="Current Employee Filters"
                fileSectionDescription="Serialized assets will be created from stored Computer Name values in the current Employee List result set."
                selectedFiles={seed.sourceSummaryLines}
                emptyFilesLabel="Current Employee List filters will be used."
                primaryActionLabel="Preview Import"
                primaryActionBusyLabel="Preparing Preview..."
                onPrimaryAction={() => void seed.handlePreview()}
                isPrimaryActionDisabled={!seed.canOpenDrawer}
                isPrimaryActionBusy={seed.isPreviewing}
                onClose={seed.closeDrawer}
                preview={preview}
                report={report}
            >
                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-4 text-sm text-[var(--text-secondary)]">
                    <div className="font-semibold text-[var(--text-primary)]">How this works</div>
                    <ul className="mt-2 list-disc space-y-1 pl-5 text-xs">
                        <li>Only current Employee List filters are used.</li>
                        <li>Preview imports valid rows and skips duplicate or invalid rows.</li>
                        <li>Approve writes valid serialized assets directly into the database.</li>
                    </ul>
                    {seed.statusMessage && (
                        <div className="mt-3 rounded-[6px] border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-300">
                            {seed.statusMessage}
                        </div>
                    )}
                </div>
            </SharedImportShell>
        </Drawer>
    )
}
