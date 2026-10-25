import { useEffect, useMemo, useState } from "react"
import { AlertTriangle, CheckCircle2, ShieldCheck, X } from "lucide-react"
import type { AssetIntegrityCheckResult, AssetIntegrityFinding } from "../../types/staff"
import { formatEmployeeIdForDisplay } from "./employeeIdDisplay"

export type AssetIntegrityModalProps = {
    open: boolean
    result: AssetIntegrityCheckResult | null
    computerDuplicatesCount: number
    onClose: () => void
    onClearHighlights?: () => void
}

function formatCategoryLabel(category: string): string {
    switch (category) {
        case "duplicate_holder":
            return "Duplicate Active Holder"
        case "multiple_active_loans":
            return "Multiple Active Loans"
        case "duplicate_asset_code":
            return "Duplicate Asset Code"
        case "orphan_loan":
            return "Orphan Loan"
        case "status_mismatch":
            return "Status Mismatch"
        case "duplicate_computer_name":
            return "Duplicate Computer Name"
        default:
            return category
    }
}

export function AssetIntegrityModal({
    open,
    result,
    computerDuplicatesCount,
    onClose,
    onClearHighlights,
}: AssetIntegrityModalProps) {
    const [filterCategory, setFilterCategory] = useState<string>("all")

    // Keyboard navigation (Esc to close)
    useEffect(() => {
        if (!open) return

        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === "Escape") {
                onClose()
            }
        }

        window.addEventListener("keydown", handleKeyDown)
        return () => window.removeEventListener("keydown", handleKeyDown)
    }, [open, onClose])

    const findings = useMemo(() => result?.findings ?? [], [result?.findings])
    const isClean = (result?.isClean ?? true) && computerDuplicatesCount === 0 && findings.length === 0

    const filteredFindings = useMemo(() => {
        if (filterCategory === "all") return findings
        return findings.filter((f) => f.category === filterCategory)
    }, [findings, filterCategory])

    if (!open) return null

    return (
        <div
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs"
            role="dialog"
            aria-modal="true"
            aria-labelledby="asset-integrity-title"
            onClick={(e) => {
                if (e.target === e.currentTarget) onClose()
            }}
        >
            <div className="flex max-h-[88vh] w-full max-w-4xl flex-col rounded-xl border border-[var(--border)] bg-[var(--surface-elevated)] shadow-2xl">
                {/* Header */}
                <div className="flex items-center justify-between border-b border-[var(--border)] px-6 py-4">
                    <div className="flex items-center gap-3">
                        <div className={`flex h-10 w-10 items-center justify-center rounded-lg ${isClean ? "bg-emerald-500/15 text-emerald-500" : "bg-amber-500/15 text-amber-500"}`}>
                            {isClean ? <ShieldCheck size={22} /> : <AlertTriangle size={22} />}
                        </div>
                        <div>
                            <div className="flex items-center gap-2.5">
                                <h2 id="asset-integrity-title" className="text-lg font-semibold text-[var(--text-primary)]">
                                    Asset & Assignment Integrity Check
                                </h2>
                                <span
                                    className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wider ${
                                        isClean
                                            ? "border border-emerald-500/30 bg-emerald-500/10 text-emerald-500"
                                            : "border border-amber-500/30 bg-amber-500/10 text-amber-500"
                                    }`}
                                >
                                    {isClean ? "PASS" : `${findings.length} Finding${findings.length > 1 ? "s" : ""}`}
                                </span>
                            </div>
                            <p className="text-xs text-[var(--text-secondary)]">
                                Database-wide validation of canonical assets, active loans, and custody consistency
                            </p>
                        </div>
                    </div>
                    <button
                        onClick={onClose}
                        className="rounded-lg p-1.5 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                        title="Close (Esc)"
                        aria-label="Close modal"
                    >
                        <X size={18} />
                    </button>
                </div>

                {/* Metrics Bar */}
                {result && (
                    <div className="grid grid-cols-3 divide-x divide-[var(--border)] border-b border-[var(--border)] bg-[var(--surface)] text-center text-xs">
                        <div className="py-2.5">
                            <span className="text-[var(--text-secondary)]">Assets Checked: </span>
                            <span className="font-semibold text-[var(--text-primary)]">{result.totalAssetsChecked}</span>
                        </div>
                        <div className="py-2.5">
                            <span className="text-[var(--text-secondary)]">Active Loans Checked: </span>
                            <span className="font-semibold text-[var(--text-primary)]">{result.totalActiveLoansChecked}</span>
                        </div>
                        <div className="py-2.5">
                            <span className="text-[var(--text-secondary)]">Employees Checked: </span>
                            <span className="font-semibold text-[var(--text-primary)]">{result.totalEmployeesChecked}</span>
                        </div>
                    </div>
                )}

                {/* Body */}
                <div className="flex-1 overflow-y-auto p-6">
                    {isClean ? (
                        <div className="flex flex-col items-center justify-center py-10 text-center">
                            <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-emerald-500/15 text-emerald-500">
                                <CheckCircle2 size={36} />
                            </div>
                            <h3 className="text-xl font-semibold text-[var(--text-primary)]">All Integrity Checks Passed</h3>
                            <p className="mt-2 max-w-md text-sm text-[var(--text-secondary)]">
                                No duplicate holders, multiple active loans, orphan records, or status mismatches were found across the full database.
                            </p>
                            <div className="mt-6 rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-4 py-3 text-left text-xs text-[var(--text-secondary)]">
                                <span className="font-medium text-emerald-500">Validation Verified:</span>
                                <ul className="mt-1 list-disc pl-4 space-y-0.5">
                                    <li>Canonical Asset Codes & Monitor assignments are unique per active holder</li>
                                    <li>Multiple Monitors per employee across Home/Office locations are safely preserved</li>
                                    <li>Serialized status (Assigned vs In Stock) matches active loan custody</li>
                                </ul>
                            </div>
                        </div>
                    ) : (
                        <div className="space-y-4">
                            {/* Filter Bar */}
                            <div className="flex items-center justify-between">
                                <span className="text-xs font-medium text-[var(--text-secondary)]">
                                    Displaying {filteredFindings.length} of {findings.length} issue{findings.length > 1 ? "s" : ""}:
                                </span>
                                <select
                                    className="rounded-lg border border-[var(--border)] bg-[var(--surface)] px-2.5 py-1 text-xs text-[var(--text-primary)] focus:outline-none"
                                    value={filterCategory}
                                    onChange={(e) => setFilterCategory(e.target.value)}
                                >
                                    <option value="all">All Categories ({findings.length})</option>
                                    <option value="duplicate_holder">Duplicate Active Holder</option>
                                    <option value="multiple_active_loans">Multiple Active Loans</option>
                                    <option value="duplicate_asset_code">Duplicate Asset Code</option>
                                    <option value="orphan_loan">Orphan Loan</option>
                                    <option value="status_mismatch">Status Mismatch</option>
                                </select>
                            </div>

                            {/* Findings Table */}
                            <div className="overflow-x-auto rounded-lg border border-[var(--border)]">
                                <table className="w-full text-left text-xs">
                                    <thead className="border-b border-[var(--border)] bg-[var(--surface)] text-[var(--text-secondary)]">
                                        <tr>
                                            <th className="py-2.5 px-3 font-medium">Severity</th>
                                            <th className="py-2.5 px-3 font-medium">Category</th>
                                            <th className="py-2.5 px-3 font-medium">Asset</th>
                                            <th className="py-2.5 px-3 font-medium">Affected Staff</th>
                                            <th className="py-2.5 px-3 font-medium">Reason & Details</th>
                                            <th className="py-2.5 px-3 font-medium">Recommendation</th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-[var(--border)]">
                                        {filteredFindings.map((finding: AssetIntegrityFinding) => (
                                            <tr key={finding.id} className="hover:bg-[var(--surface-hover)]">
                                                <td className="py-3 px-3 align-top">
                                                    <span
                                                        className={`inline-flex rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider ${
                                                            finding.severity === "error"
                                                                ? "bg-red-500/15 text-red-500 border border-red-500/30"
                                                                : "bg-amber-500/15 text-amber-500 border border-amber-500/30"
                                                        }`}
                                                    >
                                                        {finding.severity}
                                                    </span>
                                                </td>
                                                <td className="py-3 px-3 align-top font-medium text-[var(--text-primary)]">
                                                    {formatCategoryLabel(finding.category)}
                                                </td>
                                                <td className="py-3 px-3 align-top font-mono text-[var(--text-primary)]">
                                                    <div>{finding.assetCode || "—"}</div>
                                                    {finding.assetName && (
                                                        <div className="text-[11px] font-sans text-[var(--text-secondary)]">
                                                            {finding.assetName}
                                                        </div>
                                                    )}
                                                </td>
                                                <td className="py-3 px-3 align-top">
                                                    {finding.affectedEmployees.length > 0 ? (
                                                        <div className="space-y-1">
                                                            {finding.affectedEmployees.map((emp) => (
                                                                <div key={emp.id} className="text-[var(--text-primary)]">
                                                                    <span className="font-mono font-medium">
                                                                        {formatEmployeeIdForDisplay(emp.employeeId)}
                                                                    </span>
                                                                    <span className="text-[var(--text-secondary)] ml-1">
                                                                        ({emp.fullName})
                                                                    </span>
                                                                </div>
                                                            ))}
                                                        </div>
                                                    ) : (
                                                        <span className="text-[var(--text-secondary)]">—</span>
                                                    )}
                                                </td>
                                                <td className="py-3 px-3 align-top text-[var(--text-primary)]">
                                                    {finding.reason}
                                                </td>
                                                <td className="py-3 px-3 align-top text-[var(--text-secondary)]">
                                                    {finding.recommendation}
                                                </td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    )}
                </div>

                {/* Footer */}
                <div className="flex items-center justify-between border-t border-[var(--border)] px-6 py-3.5 bg-[var(--surface)] text-xs">
                    <span className="text-[var(--text-secondary)]">
                        Read-only validation. Zero database mutations occurred.
                    </span>
                    <div className="flex items-center gap-2.5">
                        {onClearHighlights && (
                            <button
                                type="button"
                                onClick={onClearHighlights}
                                className="rounded-lg border border-[var(--border)] px-3 py-1.5 font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                            >
                                Clear Table Highlights
                            </button>
                        )}
                        <button
                            type="button"
                            onClick={onClose}
                            className="rounded-lg bg-[var(--primary)] px-4 py-1.5 font-medium text-[var(--primary-foreground)] transition hover:opacity-90"
                        >
                            Close
                        </button>
                    </div>
                </div>
            </div>
        </div>
    )
}
