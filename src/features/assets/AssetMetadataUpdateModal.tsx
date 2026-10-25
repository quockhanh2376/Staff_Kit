import { useState } from "react"
import {
    AlertCircle,
    CheckCircle2,
    Database,
    FileSpreadsheet,
    Info,
    RefreshCw,
    X,
} from "lucide-react"
import { staffApi } from "../../services/staff-api"
import type {
    AssetMetadataDiff,
    AssetMetadataImportReport,
    AssetMetadataPreviewReport,
    AssetMetadataPreviewRow,
    AssetMetadataRowStatus,
} from "../../types/staff"

type FilterTab = "all" | "changed" | "unchanged" | "missing" | "outOfScope"

export function AssetMetadataUpdateModal({
    open,
    onClose,
    onSuccess,
}: {
    open: boolean
    onClose: () => void
    onSuccess?: () => void
}) {
    const [filePath, setFilePath] = useState<string>("")
    const [isLoading, setIsLoading] = useState<boolean>(false)
    const [isApproving, setIsApproving] = useState<boolean>(false)
    const [preview, setPreview] = useState<AssetMetadataPreviewReport | null>(null)
    const [report, setReport] = useState<AssetMetadataImportReport | null>(null)
    const [errorMessage, setErrorMessage] = useState<string | null>(null)
    const [activeTab, setActiveTab] = useState<FilterTab>("all")

    if (!open) return null

    const handlePickFile = async () => {
        try {
            const { open: openFileDialog } = await import("@tauri-apps/plugin-dialog")
            const selected = await openFileDialog({
                multiple: false,
                filters: [{ name: "Excel Files", extensions: ["xlsx", "xls"] }],
            })
            if (selected && typeof selected === "string") {
                setFilePath(selected)
                setPreview(null)
                setReport(null)
                setErrorMessage(null)
            }
        } catch (err) {
            setErrorMessage(err instanceof Error ? err.message : String(err))
        }
    }

    const runPreview = async (pathOverride?: string) => {
        const fileToUse = pathOverride ?? filePath
        if (!fileToUse) return
        setIsLoading(true)
        setErrorMessage(null)
        setReport(null)
        try {
            const result = await staffApi.previewAssetMetadataReimport({
                filePath: fileToUse,
            })
            setPreview(result)
        } catch (err) {
            setErrorMessage(err instanceof Error ? err.message : String(err))
        } finally {
            setIsLoading(false)
        }
    }

    const handleApprove = async () => {
        if (!filePath) return
        setIsApproving(true)
        setErrorMessage(null)
        try {
            const result = await staffApi.applyAssetMetadataReimport({
                filePath,
            })
            setReport(result)
            setPreview(null)
            onSuccess?.()
        } catch (err) {
            setErrorMessage(err instanceof Error ? err.message : String(err))
        } finally {
            setIsApproving(false)
        }
    }

    const filterRows = (rows: AssetMetadataPreviewRow[]): AssetMetadataPreviewRow[] => {
        switch (activeTab) {
            case "changed":
                return rows.filter((r) => r.status === "metadataChanged")
            case "unchanged":
                return rows.filter((r) => r.status === "unchanged")
            case "missing":
                return rows.filter((r) => r.status === "missingAsset")
            case "outOfScope":
                return rows.filter((r) => r.status === "outOfScope")
            default:
                return rows
        }
    }

    const renderStatusBadge = (status: AssetMetadataRowStatus) => {
        switch (status) {
            case "metadataChanged":
                return (
                    <span className="inline-flex items-center gap-1 rounded-full bg-emerald-500/10 px-2 py-0.5 text-xs font-medium text-emerald-400">
                        <CheckCircle2 size={12} />
                        Metadata Changed
                    </span>
                )
            case "unchanged":
                return (
                    <span className="inline-flex items-center gap-1 rounded-full bg-slate-500/10 px-2 py-0.5 text-xs font-medium text-slate-400">
                        <Info size={12} />
                        Unchanged
                    </span>
                )
            case "missingAsset":
                return (
                    <span className="inline-flex items-center gap-1 rounded-full bg-amber-500/10 px-2 py-0.5 text-xs font-medium text-amber-400">
                        <AlertCircle size={12} />
                        Missing in DB
                    </span>
                )
            case "outOfScope":
                return (
                    <span className="inline-flex items-center gap-1 rounded-full bg-purple-500/10 px-2 py-0.5 text-xs font-medium text-purple-400">
                        <Info size={12} />
                        Non-Monitor (Skipped)
                    </span>
                )
            case "invalid":
                return (
                    <span className="inline-flex items-center gap-1 rounded-full bg-rose-500/10 px-2 py-0.5 text-xs font-medium text-rose-400">
                        <AlertCircle size={12} />
                        Invalid
                    </span>
                )
        }
    }

    const renderDiffItem = (diff: AssetMetadataDiff) => {
        const fieldLabels: Record<string, string> = {
            display_name: "Asset Name",
            brand: "Brand",
            model: "Model",
            serial_number: "Serial Number",
        }
        const label = fieldLabels[diff.field] || diff.field

        return (
            <div key={diff.field} className="text-xs">
                <span className="font-medium text-slate-400">{label}: </span>
                <span className="text-rose-400 line-through">
                    {diff.before ?? "(empty)"}
                </span>
                <span className="mx-1.5 text-slate-500">→</span>
                <span className="font-medium text-emerald-300">
                    {diff.after ?? "(empty)"}
                </span>
            </div>
        )
    }

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
            <div className="flex h-[90vh] w-full max-w-6xl flex-col rounded-xl border border-[var(--border)] bg-[var(--surface)] text-[var(--text-primary)] shadow-2xl">
                {/* Header */}
                <div className="flex items-center justify-between border-b border-[var(--border)] px-6 py-4">
                    <div className="flex items-center gap-3">
                        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-400">
                            <RefreshCw size={20} />
                        </div>
                        <div>
                            <h2 className="text-lg font-bold">Update Existing Asset Metadata</h2>
                            <p className="text-xs text-[var(--text-secondary)]">
                                Enrich existing Monitor records with canonical Asset Name, Brand, and Model from workbook without changing assignments.
                            </p>
                        </div>
                    </div>
                    <button
                        onClick={onClose}
                        className="rounded-lg p-1 text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                    >
                        <X size={20} />
                    </button>
                </div>

                {/* Content */}
                <div className="flex-1 overflow-y-auto p-6 space-y-6">
                    {/* Error Banner */}
                    {errorMessage && (
                        <div className="flex items-center gap-3 rounded-lg border border-rose-500/20 bg-rose-500/10 p-4 text-sm text-rose-400">
                            <AlertCircle size={18} />
                            <span>{errorMessage}</span>
                        </div>
                    )}

                    {/* Step 1: File Selection */}
                    <div className="rounded-xl border border-[var(--border)] bg-[var(--surface-card)] p-5 space-y-4">
                        <div className="flex flex-wrap items-center justify-between gap-4">
                            <div className="flex-1 min-w-[280px]">
                                <label className="block text-xs font-semibold uppercase tracking-wider text-[var(--text-secondary)] mb-1">
                                    Workbook Source File
                                </label>
                                <div className="flex items-center gap-2">
                                    <input
                                        type="text"
                                        value={filePath}
                                        onChange={(e) => setFilePath(e.target.value)}
                                        placeholder="Pick AssetList.xlsx workbook..."
                                        className="form-input flex-1 font-mono text-xs"
                                    />
                                    <button
                                        onClick={() => void handlePickFile()}
                                        className="btn btn-secondary flex items-center gap-1.5 px-3 py-1.5 text-xs"
                                    >
                                        <FileSpreadsheet size={14} />
                                        Browse
                                    </button>
                                </div>
                            </div>
                            <div className="flex items-end gap-2">
                                <button
                                    onClick={() => void runPreview()}
                                    disabled={!filePath || isLoading || isApproving}
                                    className="btn btn-primary flex items-center gap-1.5 px-4 py-2 text-xs font-semibold"
                                >
                                    {isLoading ? (
                                        <RefreshCw size={14} className="animate-spin" />
                                    ) : (
                                        <RefreshCw size={14} />
                                    )}
                                    {isLoading ? "Analyzing..." : "Preview Metadata Update"}
                                </button>
                            </div>
                        </div>
                    </div>

                    {/* Success Report */}
                    {report && (
                        <div className="rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-6 text-center space-y-3">
                            <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-400">
                                <CheckCircle2 size={24} />
                            </div>
                            <h3 className="text-base font-bold text-emerald-400">
                                Metadata Update Complete!
                            </h3>
                            <div className="flex justify-center gap-6 text-sm">
                                <div>
                                    <span className="text-[var(--text-secondary)]">Updated: </span>
                                    <span className="font-bold text-emerald-400">{report.updatedCount}</span>
                                </div>
                                <div>
                                    <span className="text-[var(--text-secondary)]">Unchanged: </span>
                                    <span className="font-bold text-slate-300">{report.unchangedCount}</span>
                                </div>
                                <div>
                                    <span className="text-[var(--text-secondary)]">Skipped: </span>
                                    <span className="font-bold text-purple-300">{report.skippedCount}</span>
                                </div>
                            </div>
                            <p className="text-xs text-[var(--text-secondary)]">
                                All assignments, active loans, locations, and employee dynamic values remained 100% untouched.
                            </p>
                        </div>
                    )}

                    {/* Preview Section */}
                    {preview && (
                        <div className="space-y-4">
                            {/* Summary Cards */}
                            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
                                <div className="rounded-lg border border-[var(--border)] bg-[var(--surface-card)] p-3">
                                    <div className="text-[10px] uppercase tracking-wider text-[var(--text-secondary)]">Total Rows</div>
                                    <div className="text-lg font-bold">{preview.summary.totalRows}</div>
                                </div>
                                <div className="rounded-lg border border-[var(--border)] bg-[var(--surface-card)] p-3">
                                    <div className="text-[10px] uppercase tracking-wider text-[var(--text-secondary)]">Monitor Rows</div>
                                    <div className="text-lg font-bold text-blue-400">{preview.summary.monitorRows}</div>
                                </div>
                                <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 p-3">
                                    <div className="text-[10px] uppercase tracking-wider text-emerald-400">To Update</div>
                                    <div className="text-lg font-bold text-emerald-400">{preview.summary.metadataChanged}</div>
                                </div>
                                <div className="rounded-lg border border-[var(--border)] bg-[var(--surface-card)] p-3">
                                    <div className="text-[10px] uppercase tracking-wider text-[var(--text-secondary)]">Unchanged</div>
                                    <div className="text-lg font-bold text-slate-400">{preview.summary.unchanged}</div>
                                </div>
                                <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-3">
                                    <div className="text-[10px] uppercase tracking-wider text-amber-400">Missing in DB</div>
                                    <div className="text-lg font-bold text-amber-400">{preview.summary.missingAsset}</div>
                                </div>
                                <div className="rounded-lg border border-purple-500/30 bg-purple-500/5 p-3">
                                    <div className="text-[10px] uppercase tracking-wider text-purple-400">Out of Scope</div>
                                    <div className="text-lg font-bold text-purple-400">{preview.summary.outOfScope}</div>
                                </div>
                            </div>

                            {/* Guardrails Info Banner */}
                            <div className="flex items-center gap-2 rounded-lg border border-blue-500/20 bg-blue-500/5 px-4 py-2.5 text-xs text-blue-300">
                                <Database size={14} className="shrink-0" />
                                <span>
                                    <strong>Read-Only Preview Guardrail:</strong> Zero database writes have been made. Only canonical Monitor fields (display name, brand, model, serial) will be updated on Approval. Assignments, locations, and non-monitor assets are protected.
                                </span>
                            </div>

                            {/* Tabs */}
                            <div className="flex flex-wrap items-center gap-1 border-b border-[var(--border)] pb-2 text-xs font-semibold">
                                <button
                                    onClick={() => setActiveTab("all")}
                                    className={`rounded-lg px-3 py-1.5 transition-colors ${activeTab === "all" ? "bg-[var(--primary)] text-white" : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"}`}
                                >
                                    All ({preview.summary.totalRows})
                                </button>
                                <button
                                    onClick={() => setActiveTab("changed")}
                                    className={`rounded-lg px-3 py-1.5 transition-colors ${activeTab === "changed" ? "bg-emerald-600 text-white" : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"}`}
                                >
                                    Changed ({preview.summary.metadataChanged})
                                </button>
                                <button
                                    onClick={() => setActiveTab("unchanged")}
                                    className={`rounded-lg px-3 py-1.5 transition-colors ${activeTab === "unchanged" ? "bg-slate-600 text-white" : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"}`}
                                >
                                    Unchanged ({preview.summary.unchanged})
                                </button>
                                <button
                                    onClick={() => setActiveTab("missing")}
                                    className={`rounded-lg px-3 py-1.5 transition-colors ${activeTab === "missing" ? "bg-amber-600 text-white" : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"}`}
                                >
                                    Missing in DB ({preview.summary.missingAsset})
                                </button>
                                <button
                                    onClick={() => setActiveTab("outOfScope")}
                                    className={`rounded-lg px-3 py-1.5 transition-colors ${activeTab === "outOfScope" ? "bg-purple-600 text-white" : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"}`}
                                >
                                    Out of Scope ({preview.summary.outOfScope})
                                </button>
                            </div>

                            {/* Table */}
                            <div className="max-h-[420px] overflow-auto rounded-xl border border-[var(--border)]">
                                <table className="w-full text-left text-xs">
                                    <thead className="sticky top-0 bg-[var(--surface)] border-b border-[var(--border)] text-[var(--text-secondary)] font-semibold">
                                        <tr>
                                            <th className="p-3 w-12 text-center">Row</th>
                                            <th className="p-3 w-32">Asset Tag</th>
                                            <th className="p-3 w-28">Category</th>
                                            <th className="p-3 w-36">Status</th>
                                            <th className="p-3">Proposed Changes (Before → After)</th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-[var(--border)] bg-[var(--surface-card)]">
                                        {filterRows(preview.rows).map((row) => (
                                            <tr key={`${row.rowNumber}-${row.rawAssetTag}`} className="hover:bg-[var(--surface-hover)]/40">
                                                <td className="p-3 text-center text-[var(--text-secondary)] font-mono">
                                                    {row.rowNumber}
                                                </td>
                                                <td className="p-3 font-mono font-medium text-[var(--text-primary)]">
                                                    <div>{row.rawAssetTag}</div>
                                                    {row.canonicalAssetCode && row.canonicalAssetCode !== row.rawAssetTag && (
                                                        <div className="text-[10px] text-[var(--text-secondary)]">({row.canonicalAssetCode})</div>
                                                    )}
                                                </td>
                                                <td className="p-3 text-[var(--text-secondary)]">
                                                    {row.rawCategory || "-"}
                                                </td>
                                                <td className="p-3">
                                                    {renderStatusBadge(row.status)}
                                                </td>
                                                <td className="p-3">
                                                    {row.diffs.length > 0 ? (
                                                        <div className="space-y-1">
                                                            {row.diffs.map((diff) => renderDiffItem(diff))}
                                                        </div>
                                                    ) : row.statusReason ? (
                                                        <span className="text-[11px] text-[var(--text-secondary)] italic">
                                                            {row.statusReason}
                                                        </span>
                                                    ) : (
                                                        <span className="text-[11px] text-[var(--text-secondary)] italic">
                                                            No changes
                                                        </span>
                                                    )}
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
                <div className="flex items-center justify-between border-t border-[var(--border)] px-6 py-4 bg-[var(--surface)]">
                    <button
                        onClick={onClose}
                        disabled={isApproving}
                        className="btn btn-secondary px-4 py-2 text-xs font-semibold"
                    >
                        {report ? "Close" : "Cancel"}
                    </button>

                    {preview && preview.summary.metadataChanged > 0 && !report && (
                        <div className="flex items-center gap-3">
                            <span className="text-xs text-[var(--text-secondary)]">
                                Ready to update {preview.summary.metadataChanged} Monitor record(s)
                            </span>
                            <button
                                onClick={() => void handleApprove()}
                                disabled={isApproving}
                                className="btn btn-primary flex items-center gap-1.5 bg-emerald-600 hover:bg-emerald-500 px-5 py-2 text-xs font-bold shadow-lg shadow-emerald-900/20"
                            >
                                {isApproving ? (
                                    <RefreshCw size={14} className="animate-spin" />
                                ) : (
                                    <CheckCircle2 size={14} />
                                )}
                                {isApproving ? "Applying Updates..." : `Approve & Update Metadata (${preview.summary.metadataChanged})`}
                            </button>
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}
