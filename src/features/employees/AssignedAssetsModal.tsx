import { useEffect, useMemo, useState } from "react"
import { AlertCircle, Laptop, LoaderCircle, MapPin, Package, X } from "lucide-react"
import type { EmployeeAssignedAssetRecord, EmployeeRecord } from "../../types/staff"
import { staffApi } from "../../services/staff-api"
import { formatDate } from "../../lib/utils"
import { formatEmployeeIdForDisplay } from "./employeeIdDisplay"

export type AssignedAssetsModalProps = {
    open: boolean
    employee: EmployeeRecord | null
    onClose: () => void
}

export function AssignedAssetsModal({
    open,
    employee,
    onClose,
}: AssignedAssetsModalProps) {
    const [assets, setAssets] = useState<EmployeeAssignedAssetRecord[]>([])
    const [isLoading, setIsLoading] = useState(false)
    const [errorMessage, setErrorMessage] = useState<string | null>(null)

    useEffect(() => {
        if (!open || !employee) {
            setAssets([])
            setErrorMessage(null)
            setIsLoading(false)
            return
        }

        let disposed = false
        setIsLoading(true)
        setErrorMessage(null)

        void (async () => {
            try {
                const list = await staffApi.getEmployeeAssignedAssets(employee.id)
                if (!disposed) {
                    setAssets(list)
                }
            } catch (err) {
                if (!disposed) {
                    setErrorMessage(err instanceof Error ? err.message : String(err))
                }
            } finally {
                if (!disposed) {
                    setIsLoading(false)
                }
            }
        })()

        return () => {
            disposed = true
        }
    }, [open, employee])

    const groupedAssets = useMemo(() => {
        const groups: { categoryName: string; items: EmployeeAssignedAssetRecord[] }[] = []
        const map = new Map<string, EmployeeAssignedAssetRecord[]>()
        for (const item of assets) {
            const cat = item.categoryName?.trim() || "Uncategorized"
            let list = map.get(cat)
            if (!list) {
                list = []
                map.set(cat, list)
                groups.push({ categoryName: cat, items: list })
            }
            list.push(item)
        }
        return groups
    }, [assets])

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

    if (!open || !employee) return null

    return (
        <div
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs"
            role="dialog"
            aria-modal="true"
            aria-labelledby="assigned-assets-title"
            onClick={(e) => {
                if (e.target === e.currentTarget) onClose()
            }}
        >
            <div className="flex max-h-[85vh] w-full max-w-3xl flex-col rounded-xl border border-[var(--border)] bg-[var(--surface-elevated)] shadow-2xl">
                {/* Header */}
                <div className="flex items-center justify-between border-b border-[var(--border)] px-5 py-4">
                    <div className="flex items-center gap-3">
                        <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-[var(--primary)]/10 text-[var(--primary)]">
                            <Laptop size={18} />
                        </div>
                        <div>
                            <h2 id="assigned-assets-title" className="text-base font-semibold text-[var(--foreground)]">
                                Assigned Assets
                            </h2>
                            <p className="text-xs text-[var(--muted)]">
                                <span className="font-mono font-medium text-[var(--primary)]">
                                    {formatEmployeeIdForDisplay(employee.employeeId)}
                                </span>{" "}
                                — {employee.fullName}
                                {employee.teamName ? ` (${employee.teamName})` : ""}
                            </p>
                        </div>
                    </div>
                    <button
                        type="button"
                        onClick={onClose}
                        className="rounded-lg p-1.5 text-[var(--muted)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--foreground)]"
                        aria-label="Close dialog"
                    >
                        <X size={18} />
                    </button>
                </div>

                {/* Body */}
                <div className="flex-1 overflow-y-auto p-5">
                    {isLoading && (
                        <div className="flex flex-col items-center justify-center py-12 text-sm text-[var(--muted)]">
                            <LoaderCircle size={28} className="animate-spin text-[var(--primary)] mb-2" />
                            Loading assigned assets...
                        </div>
                    )}

                    {errorMessage && (
                        <div className="flex items-center gap-2.5 rounded-lg border border-rose-500/30 bg-rose-500/10 p-3.5 text-xs text-rose-400">
                            <AlertCircle size={16} className="shrink-0" />
                            <span>{errorMessage}</span>
                        </div>
                    )}

                    {!isLoading && !errorMessage && assets.length === 0 && (
                        <div className="flex flex-col items-center justify-center py-12 text-center">
                            <div className="mb-2 flex h-10 w-10 items-center justify-center rounded-full bg-[var(--surface-hover)] text-[var(--muted)]">
                                <Package size={20} />
                            </div>
                            <p className="text-sm font-medium text-[var(--foreground)]">No active assigned assets</p>
                            <p className="mt-1 text-xs text-[var(--muted)]">
                                This employee currently has no active asset loans.
                            </p>
                        </div>
                    )}

                    {!isLoading && !errorMessage && assets.length > 0 && (
                        <div className="space-y-4">
                            {groupedAssets.map((group) => (
                                <div
                                    key={group.categoryName}
                                    className="overflow-hidden rounded-lg border border-[var(--border)]"
                                >
                                    <div className="flex items-center justify-between border-b border-[var(--border)] bg-[var(--surface-hover)]/60 px-3.5 py-2">
                                        <h3 className="text-xs font-semibold text-[var(--foreground)]">
                                            {group.categoryName} ({group.items.length})
                                        </h3>
                                    </div>
                                    <table className="w-full text-left text-xs">
                                        <thead className="border-b border-[var(--border)] bg-[var(--surface)] text-[var(--muted)]">
                                            <tr>
                                                <th className="px-3 py-2 font-medium">#</th>
                                                <th className="px-3 py-2 font-medium">Asset Code</th>
                                                <th className="px-3 py-2 font-medium">Asset Name</th>
                                                <th className="px-3 py-2 font-medium">Serial Number</th>
                                                <th className="px-3 py-2 font-medium">Location</th>
                                                <th className="px-3 py-2 font-medium">Status</th>
                                                <th className="px-3 py-2 font-medium">Assigned Date</th>
                                            </tr>
                                        </thead>
                                        <tbody className="divide-y divide-[var(--border)]">
                                            {group.items.map((item, idx) => (
                                                <tr
                                                    key={`${item.assetId}-${item.assetCode}`}
                                                    className="transition hover:bg-[var(--surface-hover)]"
                                                >
                                                    <td className="px-3 py-2.5 text-[var(--muted)]">{idx + 1}</td>
                                                    <td className="px-3 py-2.5 font-mono font-semibold text-[var(--primary)]">
                                                        {item.assetCode}
                                                    </td>
                                                    <td className="px-3 py-2.5 font-medium text-[var(--foreground)]">
                                                        {item.assetName}
                                                    </td>
                                                    <td className="px-3 py-2.5 font-mono text-[var(--muted)]">
                                                        {item.serialNumber || "-"}
                                                    </td>
                                                    <td className="px-3 py-2.5 text-[var(--foreground)]">
                                                        {item.usageLocation ? (
                                                            <span className="inline-flex items-center gap-1">
                                                                <MapPin size={12} className="text-[var(--muted)]" />
                                                                {item.usageLocation}
                                                            </span>
                                                        ) : (
                                                            <span className="text-[var(--muted)]">-</span>
                                                        )}
                                                    </td>
                                                    <td className="px-3 py-2.5">
                                                        {item.signatureStatus === "pending_signature" ? (
                                                            <span className="inline-flex items-center rounded-full border border-amber-500/40 bg-amber-500/10 px-2 py-0.5 text-[11px] font-medium text-amber-400">
                                                                Pending Signature
                                                            </span>
                                                        ) : (
                                                            <span className="inline-flex items-center rounded-full border border-emerald-500/40 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-400">
                                                                Signed
                                                            </span>
                                                        )}
                                                    </td>
                                                    <td className="px-3 py-2.5 text-[var(--muted)]">
                                                        {formatDate(item.borrowedAt)}
                                                    </td>
                                                </tr>
                                            ))}
                                        </tbody>
                                    </table>
                                </div>
                            ))}
                        </div>
                    )}
                </div>

                {/* Footer */}
                <div className="flex items-center justify-between border-t border-[var(--border)] px-5 py-3 bg-[var(--surface)]/50 rounded-b-xl">
                    <div className="text-xs text-[var(--muted)]">
                        {!isLoading && !errorMessage ? (
                            <span>
                                Total assigned assets:{" "}
                                <strong className="font-semibold text-[var(--foreground)]">{assets.length}</strong>
                            </span>
                        ) : null}
                    </div>
                    <button
                        type="button"
                        onClick={onClose}
                        className="rounded-lg border border-[var(--border)] bg-[var(--surface)] px-4 py-1.5 text-xs font-medium text-[var(--foreground)] transition hover:bg-[var(--surface-hover)]"
                    >
                        Close
                    </button>
                </div>
            </div>
        </div>
    )
}
