import { useState } from "react"
import {
    AlertTriangle,
    ArrowRightLeft,
    CheckCircle2,
    Clock,
    FileSpreadsheet,
    HelpCircle,
    Layers,
    MapPin,
    RotateCcw,
    ShieldAlert,
    XCircle,
} from "lucide-react"
import { Drawer } from "../../components/Drawer"
import { staffApi } from "../../services/staff-api"
import type {
    HistoricalAssignmentActionDecision,
    HistoricalAssignmentImportReport,
    HistoricalAssignmentItem,
    HistoricalAssignmentPreview,
    HistoricalAssignmentRowOverride,
} from "../../types/staff"

type FilterTab =
    | "all"
    | "valid"
    | "locationChanged"
    | "differentHolder"
    | "missingWorkbook"
    | "conflicts"
    | "errors"
    | "skipped"

export function HistoricalAssignmentImportModal({
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
    const [preview, setPreview] = useState<HistoricalAssignmentPreview | null>(null)
    const [report, setReport] = useState<HistoricalAssignmentImportReport | null>(null)
    const [errorMessage, setErrorMessage] = useState<string | null>(null)
    const [activeTab, setActiveTab] = useState<FilterTab>("all")

    // Per-asset conflict decisions: key = `${rowNumber}:${canonicalAssetCode ?? rawAssetCode}`
    const [decisions, setDecisions] = useState<Record<string, "confirm_transfer" | "quarantine" | "update_location" | "skip">>({})

    // In-memory row mapping corrections for Preview
    const [editingKey, setEditingKey] = useState<string | null>(null)
    const [editStaffId, setEditStaffId] = useState<string>("")
    const [editAssetCode, setEditAssetCode] = useState<string>("")
    const [overrides, setOverrides] = useState<HistoricalAssignmentRowOverride[]>([])

    const getItemKey = (item: HistoricalAssignmentItem) =>
        `${item.rowNumber}:${(item.canonicalAssetCode ?? item.rawAssetCode).toUpperCase()}`

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
                setDecisions({})
                setOverrides([])
            }
        } catch (err) {
            setErrorMessage(err instanceof Error ? err.message : String(err))
        }
    }

    const runPreview = async (currentOverrides?: HistoricalAssignmentRowOverride[]) => {
        if (!filePath) return
        setIsLoading(true)
        setErrorMessage(null)
        setReport(null)
        try {
            const result = await staffApi.previewHistoricalAssignmentImport({
                filePath,
                rowOverrides: currentOverrides ?? overrides,
            })
            setPreview(result)
            // Initialize default decisions for locationChanged items
            const initDecisions: Record<string, "confirm_transfer" | "quarantine" | "update_location" | "skip"> = {}
            for (const item of result.items) {
                const k = getItemKey(item)
                if (item.status === "locationChanged") {
                    initDecisions[k] = "update_location"
                } else if (item.status === "differentHolder") {
                    initDecisions[k] = "quarantine"
                }
            }
            setDecisions((prev) => ({ ...initDecisions, ...prev }))
        } catch (err) {
            setErrorMessage(err instanceof Error ? err.message : String(err))
        } finally {
            setIsLoading(false)
        }
    }

    const handlePreview = () => runPreview(overrides)

    const handleSaveMappingEdit = (item: HistoricalAssignmentItem) => {
        const newOverrides = overrides.filter(
            (o) => !(o.rowNumber === item.rowNumber && o.rawAssetCode === item.rawAssetCode)
        )
        if (editStaffId.trim() || editAssetCode.trim()) {
            newOverrides.push({
                rowNumber: item.rowNumber,
                rawAssetCode: item.rawAssetCode,
                correctedStaffId: editStaffId.trim() || undefined,
                correctedAssetCode: editAssetCode.trim() || undefined,
            })
        }
        setOverrides(newOverrides)
        setEditingKey(null)
        runPreview(newOverrides)
    }

    const handleApprove = async () => {
        if (!filePath || !preview) return
        setIsApproving(true)
        setErrorMessage(null)

        const rowActions: HistoricalAssignmentActionDecision[] = Object.entries(decisions).map(
            ([key, action]) => {
                const [rowStr, assetCode] = key.split(":")
                return {
                    rowNumber: Number(rowStr),
                    rawAssetCode: assetCode,
                    action,
                }
            }
        )

        try {
            const result = await staffApi.applyHistoricalAssignmentImport({
                filePath,
                rowOverrides: overrides.length > 0 ? overrides : undefined,
                rowActions: rowActions.length > 0 ? rowActions : undefined,
            })
            setReport(result)
            onSuccess?.()
        } catch (err) {
            setErrorMessage(err instanceof Error ? err.message : String(err))
        } finally {
            setIsApproving(false)
        }
    }

    const handleCancel = () => {
        setPreview(null)
        setReport(null)
        setErrorMessage(null)
        setDecisions({})
        setOverrides([])
        setEditingKey(null)
    }

    const summary = preview?.summary

    const filteredItems = (preview?.items ?? []).filter((item: HistoricalAssignmentItem) => {
        if (activeTab === "valid") {
            return item.status === "newAsset" || item.status === "newAssignment"
        }
        if (activeTab === "locationChanged") {
            return item.status === "locationChanged"
        }
        if (activeTab === "differentHolder") {
            return item.status === "differentHolder"
        }
        if (activeTab === "missingWorkbook") {
            return item.status === "existingLoanMissingFromWorkbook"
        }
        if (activeTab === "conflicts") {
            return item.status === "conflict" || item.status === "differentHolder"
        }
        if (activeTab === "errors") {
            return (
                item.status === "missingEmployee" ||
                item.status === "missingAsset" ||
                item.status === "error"
            )
        }
        if (activeTab === "skipped") {
            return item.status === "alreadyAssigned"
        }
        return true
    })

    return (
        <Drawer
            open={open}
            onClose={onClose}
            title="Historical Monitor Assignment Import & Reconciliation"
            widthClass="w-[1180px]"
        >
            <div className="flex h-full flex-col space-y-5 p-6">
                {/* File picker section */}
                {!preview && !report && (
                    <div className="rounded-xl border border-[var(--border)] bg-[var(--surface-subtle)] p-6 space-y-4">
                        <div className="flex items-center gap-3">
                            <div className="rounded-lg bg-[var(--primary)]/10 p-2 text-[var(--primary)]">
                                <FileSpreadsheet size={24} />
                            </div>
                            <div>
                                <h4 className="text-sm font-semibold text-[var(--text-primary)]">
                                    Select Historical Monitor Reconciliation Workbook
                                </h4>
                                <p className="text-xs text-[var(--text-secondary)]">
                                    Reconciles verified monitor assignments against existing loans with dual-location
                                    Home/Office tracking, conflict resolution, confirmed transfers, and pending signatures.
                                </p>
                            </div>
                        </div>

                        <div className="flex items-center gap-3">
                            <input
                                type="text"
                                value={filePath}
                                placeholder="Choose .xlsx file or paste file path..."
                                onChange={(e) => setFilePath(e.target.value)}
                                className="flex-1 rounded-lg border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-xs text-[var(--text-primary)]"
                            />
                            <button
                                type="button"
                                onClick={handlePickFile}
                                className="rounded-lg border border-[var(--border)] bg-[var(--surface)] px-4 py-2 text-xs font-medium text-[var(--text-primary)] hover:bg-[var(--surface-hover)]"
                            >
                                Browse...
                            </button>
                            <button
                                type="button"
                                onClick={handlePreview}
                                disabled={!filePath || isLoading}
                                className="rounded-lg bg-[var(--primary)] px-4 py-2 text-xs font-semibold text-white shadow hover:opacity-90 disabled:opacity-50"
                            >
                                {isLoading ? "Analyzing..." : "Preview Assignments"}
                            </button>
                        </div>
                    </div>
                )}

                {/* Error Banner */}
                {errorMessage && (
                    <div className="flex items-center gap-2 rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-500">
                        <XCircle size={16} className="shrink-0" />
                        <span>{errorMessage}</span>
                    </div>
                )}

                {/* Report Section */}
                {report && (
                    <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/5 p-6 space-y-4">
                        <div className="flex items-center gap-2 text-sm font-bold text-emerald-600 dark:text-emerald-400">
                            <CheckCircle2 size={20} />
                            <span>Reconciliation Completed Successfully</span>
                        </div>
                        <div className="grid grid-cols-2 gap-3 sm:grid-cols-5">
                            <div className="rounded-lg border border-[var(--border)] bg-[var(--surface)] p-3 text-center">
                                <div className="text-xs text-[var(--text-secondary)]">Assets Created</div>
                                <div className="text-lg font-bold text-[var(--text-primary)]">
                                    {report.createdAssets}
                                </div>
                            </div>
                            <div className="rounded-lg border border-[var(--border)] bg-[var(--surface)] p-3 text-center">
                                <div className="text-xs text-[var(--text-secondary)]">Assignments Created</div>
                                <div className="text-lg font-bold text-emerald-600">
                                    {report.createdAssignments}
                                </div>
                            </div>
                            <div className="rounded-lg border border-[var(--border)] bg-[var(--surface)] p-3 text-center">
                                <div className="text-xs text-[var(--text-secondary)]">Locations Updated</div>
                                <div className="text-lg font-bold text-purple-600">
                                    {report.updatedLocations ?? 0}
                                </div>
                            </div>
                            <div className="rounded-lg border border-[var(--border)] bg-[var(--surface)] p-3 text-center">
                                <div className="text-xs text-[var(--text-secondary)]">Transfers Completed</div>
                                <div className="text-lg font-bold text-blue-600">
                                    {report.transferredAssignments ?? 0}
                                </div>
                            </div>
                            <div className="rounded-lg border border-[var(--border)] bg-[var(--surface)] p-3 text-center">
                                <div className="text-xs text-[var(--text-secondary)]">Skipped / Excluded</div>
                                <div className="text-lg font-bold text-amber-500">
                                    {report.skippedAlreadyAssigned + report.excludedConflicts}
                                </div>
                            </div>
                        </div>

                        {report.errors.length > 0 && (
                            <div className="space-y-1">
                                <div className="text-xs font-semibold text-red-500">Excluded Record Notes:</div>
                                <div className="max-h-32 overflow-y-auto space-y-1 rounded border border-[var(--border)] bg-[var(--surface)] p-2">
                                    {report.errors.map((err, i) => (
                                        <div key={i} className="text-[11px] text-[var(--text-secondary)]">
                                            • {err}
                                        </div>
                                    ))}
                                </div>
                            </div>
                        )}

                        <div className="flex justify-end pt-2">
                            <button
                                type="button"
                                onClick={onClose}
                                className="rounded-lg bg-[var(--primary)] px-5 py-2 text-xs font-semibold text-white hover:opacity-90"
                            >
                                Done
                            </button>
                        </div>
                    </div>
                )}

                {/* Preview Section */}
                {preview && summary && !report && (
                    <div className="flex flex-1 flex-col space-y-4 overflow-hidden">
                        {/* Summary Badges Cards */}
                        <div className="grid grid-cols-2 gap-2 sm:grid-cols-6">
                            <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 p-2.5 text-center">
                                <div className="flex items-center justify-center gap-1 text-[11px] font-semibold text-emerald-600 dark:text-emerald-400">
                                    <Layers size={13} /> New Assets
                                </div>
                                <div className="text-lg font-bold text-emerald-600 dark:text-emerald-400">
                                    {summary.newAssets}
                                </div>
                            </div>

                            <div className="rounded-lg border border-blue-500/30 bg-blue-500/5 p-2.5 text-center">
                                <div className="flex items-center justify-center gap-1 text-[11px] font-semibold text-blue-600 dark:text-blue-400">
                                    <Clock size={13} /> New Assigns
                                </div>
                                <div className="text-lg font-bold text-blue-600 dark:text-blue-400">
                                    {summary.newAssignments}
                                </div>
                            </div>

                            <div className="rounded-lg border border-purple-500/30 bg-purple-500/5 p-2.5 text-center">
                                <div className="flex items-center justify-center gap-1 text-[11px] font-semibold text-purple-600 dark:text-purple-400">
                                    <MapPin size={13} /> Location Changed
                                </div>
                                <div className="text-lg font-bold text-purple-600 dark:text-purple-400">
                                    {summary.locationChanged ?? 0}
                                </div>
                            </div>

                            <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-2.5 text-center">
                                <div className="flex items-center justify-center gap-1 text-[11px] font-semibold text-amber-600 dark:text-amber-400">
                                    <ShieldAlert size={13} /> Conflicts / Diff Holder
                                </div>
                                <div className="text-lg font-bold text-amber-600 dark:text-amber-400">
                                    {summary.excludedConflicts}
                                </div>
                            </div>

                            <div className="rounded-lg border border-indigo-500/30 bg-indigo-500/5 p-2.5 text-center">
                                <div className="flex items-center justify-center gap-1 text-[11px] font-semibold text-indigo-600 dark:text-indigo-400">
                                    <HelpCircle size={13} /> Not in Workbook
                                </div>
                                <div className="text-lg font-bold text-indigo-600 dark:text-indigo-400">
                                    {summary.existingLoansMissing ?? 0}
                                </div>
                            </div>

                            <div className="rounded-lg border border-gray-500/30 bg-gray-500/5 p-2.5 text-center">
                                <div className="flex items-center justify-center gap-1 text-[11px] font-semibold text-gray-500">
                                    <CheckCircle2 size={13} /> Skipped
                                </div>
                                <div className="text-lg font-bold text-gray-600 dark:text-gray-300">
                                    {summary.existingSkipped}
                                </div>
                            </div>
                        </div>

                        {/* Quarantine Warning */}
                        {summary.excludedConflicts > 0 && (
                            <div className="flex items-start gap-3 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-700 dark:text-amber-300">
                                <AlertTriangle size={16} className="mt-0.5 shrink-0 text-amber-500" />
                                <div>
                                    <div className="font-semibold">
                                        Quarantine Active: {summary.excludedConflicts} conflicting assignment(s) excluded
                                    </div>
                                    <div className="text-[11px] opacity-90">
                                        Identity mismatches and unconfirmed holder transfers are quarantined and will NOT be assigned.
                                        You may safely approve the remaining {summary.newAssets + summary.newAssignments + (summary.locationChanged ?? 0)} verified assignment(s).
                                    </div>
                                </div>
                            </div>
                        )}

                        {/* Filter Tabs */}
                        <div className="flex flex-wrap items-center gap-1 border-b border-[var(--border)] pb-2 text-xs">
                            <button
                                type="button"
                                onClick={() => setActiveTab("all")}
                                className={`rounded px-3 py-1 font-medium transition ${
                                    activeTab === "all"
                                        ? "bg-[var(--primary)] text-white"
                                        : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                }`}
                            >
                                All ({preview.items.length})
                            </button>
                            <button
                                type="button"
                                onClick={() => setActiveTab("valid")}
                                className={`rounded px-3 py-1 font-medium transition ${
                                    activeTab === "valid"
                                        ? "bg-emerald-600 text-white"
                                        : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                }`}
                            >
                                Valid to Assign ({summary.newAssets + summary.newAssignments})
                            </button>
                            {(summary.locationChanged ?? 0) > 0 && (
                                <button
                                    type="button"
                                    onClick={() => setActiveTab("locationChanged")}
                                    className={`rounded px-3 py-1 font-medium transition ${
                                        activeTab === "locationChanged"
                                            ? "bg-purple-600 text-white"
                                            : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                    }`}
                                >
                                    Location Changed ({summary.locationChanged})
                                </button>
                            )}
                            {(summary.differentHolders ?? 0) > 0 && (
                                <button
                                    type="button"
                                    onClick={() => setActiveTab("differentHolder")}
                                    className={`rounded px-3 py-1 font-medium transition ${
                                        activeTab === "differentHolder"
                                            ? "bg-amber-600 text-white"
                                            : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                    }`}
                                >
                                    Different Holder ({summary.differentHolders})
                                </button>
                            )}
                            {(summary.existingLoansMissing ?? 0) > 0 && (
                                <button
                                    type="button"
                                    onClick={() => setActiveTab("missingWorkbook")}
                                    className={`rounded px-3 py-1 font-medium transition ${
                                        activeTab === "missingWorkbook"
                                            ? "bg-indigo-600 text-white"
                                            : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                    }`}
                                >
                                    Not in Workbook ({summary.existingLoansMissing})
                                </button>
                            )}
                            <button
                                type="button"
                                onClick={() => setActiveTab("conflicts")}
                                className={`rounded px-3 py-1 font-medium transition ${
                                    activeTab === "conflicts"
                                        ? "bg-amber-600 text-white"
                                        : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                }`}
                            >
                                Quarantined ({summary.excludedConflicts})
                            </button>
                            <button
                                type="button"
                                onClick={() => setActiveTab("errors")}
                                className={`rounded px-3 py-1 font-medium transition ${
                                    activeTab === "errors"
                                        ? "bg-red-600 text-white"
                                        : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                }`}
                            >
                                Errors ({summary.errors})
                            </button>
                            <button
                                type="button"
                                onClick={() => setActiveTab("skipped")}
                                className={`rounded px-3 py-1 font-medium transition ${
                                    activeTab === "skipped"
                                        ? "bg-gray-600 text-white"
                                        : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                }`}
                            >
                                Skipped ({summary.existingSkipped})
                            </button>
                        </div>

                        {/* Items Table */}
                        <div className="flex-1 overflow-auto rounded-lg border border-[var(--border)] bg-[var(--surface)]">
                            <table className="w-full text-left text-xs">
                                <thead className="sticky top-0 border-b border-[var(--border)] bg-[var(--surface-subtle)] text-[11px] font-semibold text-[var(--text-secondary)]">
                                    <tr>
                                        <th className="px-3 py-2">Row</th>
                                        <th className="px-3 py-2">Staff ID</th>
                                        <th className="px-3 py-2">Workbook Holder</th>
                                        <th className="px-3 py-2">Asset Code</th>
                                        <th className="px-3 py-2">Location</th>
                                        <th className="px-3 py-2">Status</th>
                                        <th className="px-3 py-2">Action / Resolution</th>
                                        <th className="px-3 py-2">Details</th>
                                    </tr>
                                </thead>
                                <tbody className="divide-y divide-[var(--border)]">
                                    {filteredItems.map((item: HistoricalAssignmentItem, idx: number) => {
                                        const k = getItemKey(item)
                                        const currentDecision = decisions[k]
                                        const isEditing = editingKey === k

                                        return (
                                            <tr key={idx} className="hover:bg-[var(--surface-hover)]">
                                                <td className="px-3 py-2 text-[var(--text-secondary)]">
                                                    {item.rowNumber > 0 ? item.rowNumber : "DB"}
                                                </td>
                                                <td className="px-3 py-2 font-mono font-medium text-[var(--text-primary)]">
                                                    {item.rawStaffId}
                                                </td>
                                                <td className="px-3 py-2 text-[var(--text-primary)]">
                                                    <div>{item.rawEmployeeName}</div>
                                                    {item.canonicalEmployeeName &&
                                                        item.canonicalEmployeeName.toUpperCase() !==
                                                            item.rawEmployeeName.toUpperCase() && (
                                                            <div className="text-[10px] font-medium text-amber-500">
                                                                DB: {item.canonicalEmployeeName} ({item.canonicalStaffId})
                                                            </div>
                                                        )}
                                                    {item.status === "missingEmployee" && (
                                                        <div className="text-[10px] font-medium text-red-400">
                                                            Not found in DB
                                                        </div>
                                                    )}
                                                </td>
                                                <td className="px-3 py-2 font-mono text-[var(--text-primary)]">
                                                    <span>{item.canonicalAssetCode ?? item.rawAssetCode}</span>
                                                    {item.canonicalAssetCode &&
                                                        item.canonicalAssetCode !== item.rawAssetCode && (
                                                            <span className="ml-1 text-[10px] text-[var(--text-secondary)]">
                                                                ({item.rawAssetCode})
                                                            </span>
                                                        )}
                                                </td>
                                                <td className="px-3 py-2">
                                                    <span
                                                        className={`inline-block rounded px-1.5 py-0.5 text-[10px] font-medium ${
                                                            item.location.toLowerCase().includes("nhà")
                                                                ? "bg-purple-500/10 text-purple-600 dark:text-purple-400"
                                                                : "bg-blue-500/10 text-blue-600 dark:text-blue-400"
                                                        }`}
                                                    >
                                                        {item.location}
                                                    </span>
                                                    {item.currentLocation &&
                                                        item.currentLocation.toLowerCase() !==
                                                            item.location.toLowerCase() && (
                                                            <div className="mt-0.5 text-[10px] text-[var(--text-secondary)]">
                                                                (DB: {item.currentLocation})
                                                            </div>
                                                        )}
                                                </td>
                                                <td className="px-3 py-2">
                                                    <StatusBadge status={item.status} />
                                                </td>
                                                <td className="px-3 py-2">
                                                    {item.status === "differentHolder" && (
                                                        <select
                                                            value={currentDecision ?? "quarantine"}
                                                            onChange={(e) =>
                                                                setDecisions((prev) => ({
                                                                    ...prev,
                                                                    [k]: e.target.value as "confirm_transfer" | "quarantine",
                                                                }))
                                                            }
                                                            className="rounded border border-amber-500/40 bg-[var(--surface)] px-2 py-1 text-[11px] font-medium text-[var(--text-primary)]"
                                                        >
                                                            <option value="quarantine">Quarantine (Keep DB)</option>
                                                            <option value="confirm_transfer">Confirm Transfer</option>
                                                        </select>
                                                    )}
                                                    {item.status === "locationChanged" && (
                                                        <select
                                                            value={currentDecision ?? "update_location"}
                                                            onChange={(e) =>
                                                                setDecisions((prev) => ({
                                                                    ...prev,
                                                                    [k]: e.target.value as "update_location" | "skip",
                                                                }))
                                                            }
                                                            className="rounded border border-purple-500/40 bg-[var(--surface)] px-2 py-1 text-[11px] font-medium text-[var(--text-primary)]"
                                                        >
                                                            <option value="update_location">Update Location</option>
                                                            <option value="skip">Keep Current Loc</option>
                                                        </select>
                                                    )}
                                                    {item.status === "existingLoanMissingFromWorkbook" && (
                                                        <span className="text-[11px] text-[var(--text-secondary)]">
                                                            Review only
                                                        </span>
                                                    )}
                                                    {(item.status === "newAsset" || item.status === "newAssignment") && (
                                                        <span className="text-[11px] font-medium text-emerald-600 dark:text-emerald-400">
                                                            Approve
                                                        </span>
                                                    )}
                                                    {item.status === "alreadyAssigned" && (
                                                        <span className="text-[11px] text-gray-500">
                                                            Skip (0 writes)
                                                        </span>
                                                    )}
                                                    {(item.status === "conflict" ||
                                                        item.status === "missingEmployee" ||
                                                        item.status === "error") && (
                                                        <div className="flex items-center gap-1">
                                                            {!isEditing ? (
                                                                <button
                                                                    type="button"
                                                                    onClick={() => {
                                                                        setEditingKey(k)
                                                                        setEditStaffId(item.rawStaffId)
                                                                        setEditAssetCode(item.rawAssetCode)
                                                                    }}
                                                                    className="rounded bg-[var(--surface-hover)] px-2 py-0.5 text-[10px] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                                                                >
                                                                    Correct...
                                                                </button>
                                                            ) : (
                                                                <div className="flex items-center gap-1">
                                                                    <input
                                                                        type="text"
                                                                        value={editStaffId}
                                                                        onChange={(e) => setEditStaffId(e.target.value)}
                                                                        placeholder="Staff ID"
                                                                        className="w-16 rounded border px-1 py-0.5 text-[10px]"
                                                                    />
                                                                    <button
                                                                        type="button"
                                                                        onClick={() => handleSaveMappingEdit(item)}
                                                                        className="rounded bg-emerald-600 px-1.5 py-0.5 text-[10px] text-white"
                                                                    >
                                                                        Apply
                                                                    </button>
                                                                    <button
                                                                        type="button"
                                                                        onClick={() => setEditingKey(null)}
                                                                        className="rounded bg-gray-500 px-1.5 py-0.5 text-[10px] text-white"
                                                                    >
                                                                        X
                                                                    </button>
                                                                </div>
                                                            )}
                                                        </div>
                                                    )}
                                                </td>
                                                <td
                                                    className="max-w-xs truncate px-3 py-2 text-[11px] text-[var(--text-secondary)]"
                                                    title={item.statusReason ?? ""}
                                                >
                                                    {item.statusReason ?? "—"}
                                                </td>
                                            </tr>
                                        )
                                    })}
                                </tbody>
                            </table>
                        </div>

                        {/* Action Buttons */}
                        <div className="flex items-center justify-between border-t border-[var(--border)] pt-3">
                            <button
                                type="button"
                                onClick={handleCancel}
                                disabled={isApproving}
                                className="flex items-center gap-1.5 rounded-lg border border-[var(--border)] bg-[var(--surface)] px-4 py-2 text-xs font-medium text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                            >
                                <RotateCcw size={13} />
                                <span>Cancel (Zero Writes)</span>
                            </button>

                            <button
                                type="button"
                                onClick={handleApprove}
                                disabled={!summary.canApproveRemainder || isApproving}
                                className="rounded-lg bg-emerald-600 px-5 py-2 text-xs font-semibold text-white shadow hover:bg-emerald-500 disabled:opacity-50"
                            >
                                {isApproving
                                    ? "Applying Reconciliation..."
                                    : `Approve Valid Remainder (${summary.newAssets + summary.newAssignments})`}
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </Drawer>
    )
}

function StatusBadge({ status }: { status: string }) {
    switch (status) {
        case "newAsset":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-emerald-600 dark:text-emerald-400">
                    <Layers size={10} /> New Asset & Assign
                </span>
            )
        case "newAssignment":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-blue-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-blue-600 dark:text-blue-400">
                    <Clock size={10} /> New Assign
                </span>
            )
        case "locationChanged":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-purple-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-purple-600 dark:text-purple-400">
                    <MapPin size={10} /> Location Changed
                </span>
            )
        case "differentHolder":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-amber-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-amber-600 dark:text-amber-400">
                    <ArrowRightLeft size={10} /> Different Holder
                </span>
            )
        case "existingLoanMissingFromWorkbook":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-indigo-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-indigo-600 dark:text-indigo-400">
                    <HelpCircle size={10} /> Not in Workbook
                </span>
            )
        case "alreadyAssigned":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-gray-500/10 px-1.5 py-0.5 text-[10px] font-medium text-gray-500">
                    <CheckCircle2 size={10} /> Skipped (Assigned)
                </span>
            )
        case "conflict":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-amber-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-amber-600 dark:text-amber-400">
                    <AlertTriangle size={10} /> Quarantined Conflict
                </span>
            )
        case "missingEmployee":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-red-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-red-500">
                    <XCircle size={10} /> Missing Employee
                </span>
            )
        case "missingAsset":
            return (
                <span className="inline-flex items-center gap-1 rounded bg-red-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-red-500">
                    <XCircle size={10} /> Missing Asset
                </span>
            )
        default:
            return (
                <span className="inline-flex items-center gap-1 rounded bg-red-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-red-500">
                    <XCircle size={10} /> Error
                </span>
            )
    }
}
