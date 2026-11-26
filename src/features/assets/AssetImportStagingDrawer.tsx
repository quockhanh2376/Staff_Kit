import { useState } from "react"
import { Drawer } from "../../components/Drawer"
import type { AssetImportRowRecord, AssetImportRowUpdateInput, AssetTrackingMode } from "../../types/staff"
import { ASSET_CONDITIONS, CONDITION_LABELS } from "./assetCondition"
import type { AssetImportStagingState } from "./useAssetImportStaging"

// Mirrors EXISTING_ASSET_NOTE in asset_import.rs.
const EXISTING_NOTE = "Already in Staff Kit"
const SERIAL_NOTE_PREFIX = "Serial will update"

type Kind = "new" | "existing" | "error" | "skipped"
const KIND_LABELS: Record<Kind, string> = { new: "New", existing: "Existing", error: "Errors", skipped: "Skipped" }

function kindOf(row: AssetImportRowRecord): Kind {
    if (row.status === "error") return "error"
    if (row.reviewNotes.includes(EXISTING_NOTE)) return "existing"
    return row.status === "skipped" ? "skipped" : "new"
}

const TYPE_LABELS: Record<AssetTrackingMode, string> = { serialized: "Serial", quantity: "Quantity" }

const TEXT_COLUMNS: { key: AssetImportRowUpdateInput["fieldKey"]; label: string }[] = [
    { key: "assetType", label: "Category" },
    { key: "assetCode", label: "Asset code" },
    { key: "displayName", label: "Name" },
    { key: "computerName", label: "Computer name" },
    { key: "brand", label: "Brand" },
    { key: "model", label: "Model" },
    { key: "serialNumber", label: "Serial" },
    { key: "quantity", label: "Qty" },
    { key: "usageLocation", label: "Location" },
    { key: "notes", label: "Note" },
]

type Props = {
    staging: AssetImportStagingState
    setGlobalNotice: (message: string | null) => void
}

export function AssetImportStagingDrawer({ staging, setGlobalNotice }: Props) {
    const [kindFilter, setKindFilter] = useState<"all" | Kind>("all")
    const batch = staging.batch
    if (!batch) return null

    const rows = batch.rows
    const count = (kind: Kind) => rows.filter((row) => kindOf(row) === kind).length
    const serialUpdates = rows.filter((row) => row.reviewNotes.some((note) => note.startsWith(SERIAL_NOTE_PREFIX))).length
    const newRows = count("new")
    const errorRows = count("error")
    const parts = [newRows && `${newRows} new`, serialUpdates && `${serialUpdates} serial updates`].filter(Boolean)
    const approveLabel = parts.length ? `Approve — ${parts.join(" · ")}` : "Approve"
    const visibleRows = kindFilter === "all" ? rows : rows.filter((row) => kindOf(row) === kindFilter)

    const save = (row: AssetImportRowRecord, key: AssetImportRowUpdateInput["fieldKey"], value: string) => {
        const next = value.trim() || null
        if (next === ((row[key as keyof AssetImportRowRecord] as string | null | undefined) ?? null)) return
        void staging.updateRow(row.id, key, next)
    }

    const handleDiscard = async () => {
        if (!window.confirm("Discard this staged import? Nothing will be written.")) return
        await staging.discard()
    }

    const handleApprove = async () => {
        const result = await staging.approve()
        if (result) setGlobalNotice(`Asset import approved: ${result.importedCount} new, ${serialUpdates} serial updates.`)
    }

    const chipClass = "rounded-full border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-1 text-xs font-semibold text-[var(--text-primary)]"
    const inputClass =
        "w-full min-w-[7rem] rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1 text-xs text-[var(--text-primary)] disabled:opacity-60"

    return (
        <Drawer open={staging.isOpen} onClose={staging.close} title={`Staged import — ${batch.sourceFileName}`} widthClass="w-[1200px]">
            <div aria-label="Staged asset import" className="flex h-full flex-col gap-4 text-sm text-[var(--text-primary)]">
                <div className="flex flex-wrap items-center gap-2">
                    {(Object.keys(KIND_LABELS) as Kind[]).map((kind) => (
                        <span key={kind} className={chipClass}>{`${KIND_LABELS[kind]} ${count(kind)}`}</span>
                    ))}
                    <span className={chipClass}>{`Serial updates ${serialUpdates}`}</span>
                    <select
                        aria-label="Row filter"
                        className="ml-auto rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1 text-xs text-[var(--text-primary)]"
                        value={kindFilter}
                        onChange={(e) => setKindFilter(e.target.value as "all" | Kind)}
                    >
                        <option value="all">All</option>
                        {(Object.keys(KIND_LABELS) as Kind[]).map((kind) => (
                            <option key={kind} value={kind}>{KIND_LABELS[kind]}</option>
                        ))}
                    </select>
                </div>

                {staging.error && (
                    <div role="alert" className="rounded-[6px] border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-2 text-xs text-[var(--text-primary)]">
                        {staging.error}
                    </div>
                )}

                <div className="min-h-0 flex-1 overflow-auto rounded-[8px] border border-[var(--border)]">
                    <table className="w-full min-w-max text-left text-xs">
                        <thead className="table-head sticky top-0 uppercase tracking-[0.04em] text-[var(--text-secondary)]">
                            <tr>
                                {["Row", "Kind", "Type", ...TEXT_COLUMNS.map((c) => c.label), "Condition", "Skip", "Notes"].map((h) => (
                                    <th key={h} className="px-2 py-2 font-semibold">{h}</th>
                                ))}
                            </tr>
                        </thead>
                        <tbody>
                            {visibleRows.map((row) => {
                                const kind = kindOf(row)
                                const condition = row.condition?.trim().toLowerCase() ?? ""
                                const label = row.assetCode ?? `row ${row.rowNumber}`
                                return (
                                    <tr
                                        key={row.id}
                                        className={`border-t border-[var(--border)] align-top ${kind === "error" ? "search-highlight-cell" : ""}`}
                                    >
                                        <td className="px-2 py-2 font-medium">{row.rowNumber}</td>
                                        <td className="px-2 py-2 text-[var(--text-secondary)]">{KIND_LABELS[kind]}</td>
                                        <td className="px-2 py-2">
                                            <select
                                                aria-label={`Type for ${label}`}
                                                className={inputClass}
                                                value={row.kindOverride ?? ""}
                                                disabled={staging.isBusy}
                                                onChange={(e) => save(row, "kind", e.target.value)}
                                            >
                                                <option value="">{`Auto${row.kind ? ` (${TYPE_LABELS[row.kind]})` : ""}`}</option>
                                                <option value="serialized">{TYPE_LABELS.serialized}</option>
                                                <option value="quantity">{TYPE_LABELS.quantity}</option>
                                            </select>
                                        </td>
                                        {TEXT_COLUMNS.map(({ key, label: column }) => {
                                            const value = (row[key as keyof AssetImportRowRecord] as string | null | undefined) ?? ""
                                            return (
                                                <td key={key} className="px-2 py-2">
                                                    {/* key remounts the uncontrolled input when the stored value changes. */}
                                                    <input
                                                        key={value}
                                                        aria-label={`${column} for ${label}`}
                                                        className={inputClass}
                                                        defaultValue={value}
                                                        disabled={staging.isBusy}
                                                        onBlur={(e) => save(row, key, e.target.value)}
                                                    />
                                                </td>
                                            )
                                        })}
                                        <td className="px-2 py-2">
                                            <select
                                                aria-label={`Condition for ${label}`}
                                                className={inputClass}
                                                value={ASSET_CONDITIONS.includes(condition as never) ? condition : ""}
                                                disabled={staging.isBusy}
                                                onChange={(e) => save(row, "condition", e.target.value)}
                                            >
                                                <option value="" />
                                                {ASSET_CONDITIONS.map((item) => (
                                                    <option key={item} value={item}>{CONDITION_LABELS[item]}</option>
                                                ))}
                                            </select>
                                        </td>
                                        <td className="px-2 py-2 text-center">
                                            <input
                                                type="checkbox"
                                                aria-label={`Skip ${label}`}
                                                checked={row.status === "skipped"}
                                                // Existing assets are never created; edit the serial to change what happens to them.
                                                disabled={staging.isBusy || kind === "existing"}
                                                onChange={(e) => void staging.setRowSkipped(row.id, e.target.checked)}
                                            />
                                        </td>
                                        <td className="px-2 py-2 text-[var(--text-secondary)]">
                                            {[...row.validationErrors, ...row.reviewNotes.filter((note) => note !== EXISTING_NOTE)].map((note) => (
                                                <div key={note}>{note}</div>
                                            ))}
                                        </td>
                                    </tr>
                                )
                            })}
                        </tbody>
                    </table>
                </div>

                <div className="flex items-center justify-end gap-2 border-t border-[var(--border)] pt-4">
                    {errorRows > 0 && (
                        <span className="mr-auto text-xs text-[var(--text-secondary)]">{`Fix or skip ${errorRows} error row(s) to approve.`}</span>
                    )}
                    <button
                        type="button"
                        className="rounded-[8px] border border-[var(--border)] px-4 py-2 text-sm font-semibold text-[var(--text-primary)] hover:bg-[var(--surface-hover)] disabled:opacity-60"
                        disabled={staging.isBusy}
                        onClick={() => void handleDiscard()}
                    >
                        Discard
                    </button>
                    <button
                        type="button"
                        className="rounded-[8px] border border-[var(--primary)] bg-[var(--primary)]/15 px-4 py-2 text-sm font-semibold text-[var(--primary)] hover:bg-[var(--primary)]/25 disabled:cursor-not-allowed disabled:opacity-60"
                        disabled={errorRows > 0 || parts.length === 0 || staging.isBusy}
                        onClick={() => void handleApprove()}
                    >
                        {approveLabel}
                    </button>
                </div>
            </div>
        </Drawer>
    )
}
