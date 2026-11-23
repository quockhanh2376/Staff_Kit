import { useState } from "react"
import { Drawer } from "../../components/Drawer"
import type { AssetDetailsUpdateInput } from "../../types/staff"
import type { AssetDashboardState } from "./useAssetDashboardState"
import { CONDITION_LABELS } from "./assetCondition"

type SerializedRow = AssetDashboardState["serializedRows"][number]
type DraftKey = keyof AssetDetailsUpdateInput

const EDITABLE_FIELDS: { key: DraftKey; label: string; multiline?: boolean }[] = [
  { key: "displayName", label: "Display name" },
  { key: "displayNameShort", label: "Short name" },
  { key: "brand", label: "Brand" },
  { key: "model", label: "Model" },
  { key: "serialNumber", label: "Serial number" },
  { key: "adapterNumber", label: "Adapter number" },
  { key: "usageLocation", label: "Location" },
  { key: "notes", label: "Notes", multiline: true },
]

const labelClass =
  "mb-1 block text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]"

function buildDraft(row: SerializedRow): Record<DraftKey, string> {
  return Object.fromEntries(
    EDITABLE_FIELDS.map(({ key }) => [key, row[key] ?? ""]),
  ) as Record<DraftKey, string>
}

/** Super-admin drawer for fixing the descriptive fields of one serialized asset. */
export function EditAssetDrawer({
  row,
  isSaving,
  error,
  onClose,
  onSave,
}: {
  row: SerializedRow
  isSaving: boolean
  error: string | null
  onClose: () => void
  onSave: (payload: AssetDetailsUpdateInput) => void
}) {
  const [initialDraft] = useState(() => buildDraft(row))
  const [draft, setDraft] = useState(initialDraft)
  const isDirty = EDITABLE_FIELDS.some(({ key }) => draft[key] !== initialDraft[key])
  const canSave = draft.displayName.trim() !== "" && isDirty && !isSaving

  const handleSave = () => {
    const payload = Object.fromEntries(
      EDITABLE_FIELDS.map(({ key }) => [key, draft[key].trim() || null]),
    ) as AssetDetailsUpdateInput
    onSave({ ...payload, displayName: draft.displayName.trim() })
  }

  return (
    <Drawer open onClose={onClose} title="Edit asset" widthClass="w-[560px]">
      <div className="space-y-5">
        <dl className="grid grid-cols-4 gap-3 rounded-[12px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-4 text-sm">
          {[
            ["Asset code", row.assetCode],
            ["Category", row.categoryName ?? "—"],
            ["Status", row.status],
            ["Condition", CONDITION_LABELS[row.condition] ?? row.condition],
          ].map(([term, value]) => (
            <div key={term}>
              <dt className={labelClass}>{term}</dt>
              <dd className="font-semibold text-[var(--text-primary)]">{value}</dd>
            </div>
          ))}
        </dl>
        <div className="grid gap-4 md:grid-cols-2">
          {EDITABLE_FIELDS.map(({ key, label, multiline }) => {
            const inputId = `edit-asset-${key}`
            const commonProps = {
              id: inputId,
              className: "form-input",
              value: draft[key],
              onChange: (event: { target: { value: string } }) =>
                setDraft((current) => ({ ...current, [key]: event.target.value })),
            }
            return (
              <div key={key} className={multiline || key === "displayName" ? "md:col-span-2" : ""}>
                <label className={labelClass} htmlFor={inputId}>
                  {label}
                </label>
                {multiline ? <textarea rows={3} {...commonProps} /> : <input type="text" {...commonProps} />}
              </div>
            )
          })}
        </div>
        {error && (
          <p role="alert" className="rounded-[8px] border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-300">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <button
            className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)]"
            onClick={onClose}
            type="button"
          >
            Cancel
          </button>
          <button
            className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
            disabled={!canSave}
            onClick={handleSave}
            type="button"
          >
            {isSaving ? "Saving..." : "Save changes"}
          </button>
        </div>
      </div>
    </Drawer>
  )
}
