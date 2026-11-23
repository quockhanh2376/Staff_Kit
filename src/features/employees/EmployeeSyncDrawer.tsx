import { useState } from "react"
import { Drawer } from "../../components/Drawer"
import type { EmployeeSyncFields, EmployeeSyncStagedRow } from "../../types/staff"
import type { EmployeeState } from "./useEmployeeState"

const DUPLICATE_NOTE = "Duplicate Staff Code in HR source"

const ACTION_LABELS: Record<string, string> = {
    apply: "Apply",
    skip: "Skip",
    move_to_employee_list: "Move to EE List",
    move_to_offboarding: "Move to Offboarding",
    none: "Keep",
}

const KIND_LABELS: Record<string, string> = { new: "New", changed: "Changed", missing: "Missing", conflict: "Conflicts" }

const FIELD_COLUMNS: { key: keyof EmployeeSyncFields; label: string }[] = [
    { key: "fullName", label: "Full name" },
    { key: "nickName", label: "Nick" },
    { key: "email", label: "Email" },
    { key: "azureAdAccount", label: "Azure AD" },
]

const isDuplicate = (row: EmployeeSyncStagedRow) => row.notes.includes(DUPLICATE_NOTE)

// Mirrors allowed_actions in employee_sync_staging.rs.
function actionsFor(row: EmployeeSyncStagedRow): string[] {
    if (isDuplicate(row)) return ["skip"]
    switch (row.kind) {
        case "new":
        case "changed":
            return ["apply", "skip"]
        case "conflict":
            return ["move_to_employee_list", "skip"]
        case "missing":
            return ["move_to_offboarding", "none"]
        default:
            return [row.action]
    }
}

const shownFields = (row: EmployeeSyncStagedRow): EmployeeSyncFields | null => row.edits ?? row.source[0] ?? row.local

type EmployeeSyncDrawerProps = {
    open: boolean
    onClose: () => void
    employeeState: EmployeeState
    setGlobalNotice: (message: string | null) => void
}

export function EmployeeSyncDrawer({ open, onClose, employeeState: emp, setGlobalNotice }: EmployeeSyncDrawerProps) {
    const [kindFilter, setKindFilter] = useState("all")
    const batch = emp.syncBatch
    if (!batch) return null

    // Kind counts come from rows: the hook merges single rows on edit/refresh, so batch.counts can lag.
    const rows = batch.rows
    const countKind = (kind: string) => rows.filter((row) => row.kind === kind).length
    const countAction = (action: string, kind?: string) =>
        rows.filter((row) => row.action === action && (!kind || row.kind === kind)).length
    const totals = [
        ["Add", countAction("apply", "new")],
        ["Update", countAction("apply", "changed")],
        ["Move", countAction("move_to_employee_list"), "to EE List"],
        ["Offboard", countAction("move_to_offboarding")],
    ] as const
    const totalParts = totals.filter(([, n]) => n > 0).map(([verb, n, suffix]) => [verb, n, suffix].filter(Boolean).join(" "))
    const approveLabel = totalParts.length ? `Approve — ${totalParts.join(" · ")}` : "Approve"
    const visibleRows = kindFilter === "all" ? rows : rows.filter((row) => row.kind === kindFilter)

    const saveField = (row: EmployeeSyncStagedRow, key: keyof EmployeeSyncFields, value: string) => {
        const current = shownFields(row) ?? { fullName: "", nickName: null, email: null, azureAdAccount: null }
        const next = key === "fullName" ? value.trim() : value.trim() || null
        if (next === current[key]) return
        void emp.updateSyncRow(row.id, { edits: { ...current, [key]: next } })
    }

    const handleSkip = () => {
        if (!window.confirm("Discard this EE List sync? Nothing will be written.")) return
        void emp.discardEmployeeSync()
    }

    const handleApprove = async () => {
        const holders = rows.filter((row) => row.kind === "missing" && row.action === "move_to_offboarding" && row.assetsHeld > 0)
        if (holders.length > 0) {
            const assets = holders.reduce((sum, row) => sum + row.assetsHeld, 0)
            if (!window.confirm(`${holders.length} people being offboarded still hold ${assets} assets. Continue?`)) return
        }
        const result = await emp.applyEmployeeSync()
        if (!result) return // error stays in the drawer; the hook reloads stale rows
        setGlobalNotice(
            `EE List updated: added ${result.added}, updated ${result.updated}, moved ${result.movedToEmployeeList}, offboarded ${result.offboarded}.`,
        )
        onClose()
    }

    const chipClass = "rounded-full border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-1 text-xs font-semibold text-[var(--text-primary)]"
    const inputClass =
        "w-full rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1 text-xs text-[var(--text-primary)] disabled:opacity-60"

    return (
        <Drawer open={open} onClose={onClose} title="EE List sync" widthClass="w-[1200px]">
            <div aria-label="EE List sync" className="flex h-full flex-col gap-4 text-sm text-[var(--text-primary)]">
                <div className="flex flex-wrap items-center gap-2">
                    {(["new", "changed", "missing", "conflict"] as const).map((kind) => (
                        <span key={kind} className={chipClass}>{`${KIND_LABELS[kind]} ${countKind(kind)}`}</span>
                    ))}
                    <span className={chipClass}>{`Unchanged ${batch.counts.unchanged}`}</span>
                    <select
                        aria-label="Kind filter"
                        className="ml-auto rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1 text-xs text-[var(--text-primary)]"
                        value={kindFilter}
                        onChange={(e) => setKindFilter(e.target.value)}
                    >
                        <option value="all">All</option>
                        {Object.entries(KIND_LABELS).map(([kind, label]) => (
                            <option key={kind} value={kind}>{label}</option>
                        ))}
                    </select>
                </div>

                {emp.syncError && (
                    <div role="alert" className="rounded-[6px] border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-2 text-xs text-[var(--text-primary)]">
                        {emp.syncError}
                    </div>
                )}

                <div className="min-h-0 flex-1 overflow-auto rounded-[8px] border border-[var(--border)]">
                    <table className="w-full text-left text-xs">
                        <thead className="table-head sticky top-0 uppercase tracking-[0.04em] text-[var(--text-secondary)]">
                            <tr>
                                {["Staff code", "Kind", "Local name", "Source name", ...FIELD_COLUMNS.map((c) => c.label), "Action", "Notes"].map((h) => (
                                    <th key={h} className="px-2 py-2 font-semibold">{h}</th>
                                ))}
                            </tr>
                        </thead>
                        <tbody>
                            {visibleRows.map((row) => {
                                const fields = shownFields(row)
                                const readOnly = isDuplicate(row) || row.kind === "missing"
                                const actions = actionsFor(row)
                                const notes = row.assetsHeld > 0 ? [...row.notes, `Holds ${row.assetsHeld} assets`] : row.notes
                                return (
                                    <tr
                                        key={row.id}
                                        className={`border-t border-[var(--border)] align-top ${row.state === "stale" ? "search-highlight-cell" : ""}`}
                                    >
                                        <td className="px-2 py-2 font-medium">{row.staffCode}</td>
                                        <td className="px-2 py-2 text-[var(--text-secondary)]">{KIND_LABELS[row.kind] ?? row.kind}</td>
                                        <td className="px-2 py-2">{row.local?.fullName ?? "-"}</td>
                                        <td className="px-2 py-2">{row.source.map((s) => s.fullName).join(" / ") || "-"}</td>
                                        {FIELD_COLUMNS.map(({ key: field, label }) => (
                                            <td key={field} className="px-2 py-2">
                                                <input
                                                    key={fields?.[field] ?? ""}
                                                    aria-label={`${label} for ${row.staffCode}`}
                                                    className={inputClass}
                                                    defaultValue={fields?.[field] ?? ""}
                                                    disabled={readOnly || emp.isSyncBusy}
                                                    onBlur={(e) => saveField(row, field, e.target.value)}
                                                />
                                            </td>
                                        ))}
                                        <td className="px-2 py-2">
                                            <select
                                                aria-label={`Action for ${row.staffCode}`}
                                                className={inputClass}
                                                value={row.action}
                                                disabled={actions.length < 2 || emp.isSyncBusy}
                                                onChange={(e) => void emp.updateSyncRow(row.id, { action: e.target.value })}
                                            >
                                                {actions.map((action) => (
                                                    <option key={action} value={action}>{ACTION_LABELS[action] ?? action}</option>
                                                ))}
                                            </select>
                                        </td>
                                        <td className="px-2 py-2 text-[var(--text-secondary)]">
                                            {notes.map((note) => (
                                                <div key={note}>{note}</div>
                                            ))}
                                            {row.state === "stale" && (
                                                <button
                                                    type="button"
                                                    className="mt-1 rounded-[6px] border border-[var(--border)] px-2 py-1 text-xs font-semibold text-[var(--primary)] hover:bg-[var(--surface-hover)] disabled:opacity-60"
                                                    disabled={emp.isSyncBusy}
                                                    onClick={() => void emp.refreshSyncRow(row.id)}
                                                >
                                                    Reload from app
                                                </button>
                                            )}
                                        </td>
                                    </tr>
                                )
                            })}
                        </tbody>
                    </table>
                </div>

                <div className="flex items-center justify-end gap-2 border-t border-[var(--border)] pt-4">
                    <button
                        type="button"
                        className="rounded-[8px] border border-[var(--border)] px-4 py-2 text-sm font-semibold text-[var(--text-primary)] hover:bg-[var(--surface-hover)] disabled:opacity-60"
                        disabled={emp.isSyncBusy}
                        onClick={handleSkip}
                    >
                        Skip
                    </button>
                    <button
                        type="button"
                        className="rounded-[8px] border border-[var(--primary)] bg-[var(--primary)]/15 px-4 py-2 text-sm font-semibold text-[var(--primary)] hover:bg-[var(--primary)]/25 disabled:cursor-not-allowed disabled:opacity-60"
                        disabled={totalParts.length === 0 || emp.isSyncBusy}
                        onClick={() => void handleApprove()}
                    >
                        {approveLabel}
                    </button>
                </div>
            </div>
        </Drawer>
    )
}
