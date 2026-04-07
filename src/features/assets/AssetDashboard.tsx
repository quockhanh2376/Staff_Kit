import { useMemo, useState } from "react"
import {
  Boxes,
  FileSpreadsheet,
  LoaderCircle,
  PlusCircle,
  RefreshCw,
} from "lucide-react"
import type { AuthState } from "../auth/useAuthState"
import type { AssetImportState } from "./useAssetImportState"
import type { AssetDashboardState } from "./useAssetDashboardState"
import {
  buildAssetDashboardSummaryCards,
  formatAssetDashboardDisplayNameLines,
  formatAssetDashboardHolderLabel,
  formatAssetDashboardStatusLabel,
  formatAssetDashboardUsageLocationLabel,
  getAssetDashboardDescription,
  getAssetDashboardEmptyStateLabel,
  getAssetDashboardTabLabel,
  parseAssetDashboardQuantityDraft,
  type AssetDashboardTabKey,
} from "./assetDashboardCopy"
import { getAssetImportModeLabel } from "./assetImportModeConfig"
import { getAssetImportSummaryLabel } from "./assetImportStatusMeta"

type AssetDashboardProps = {
  auth: Pick<AuthState, "canImportData" | "isAdminAccount">
  assetDashboard: AssetDashboardState
  assetImport: AssetImportState
}

type QuantityDraftMap = Record<
  number,
  {
    quantityOnHand: string
    assignedQuantity: string
  }
>

export function AssetDashboard({
  auth,
  assetDashboard,
  assetImport,
}: AssetDashboardProps) {
  const [activeTab, setActiveTab] = useState<AssetDashboardTabKey>("serialized")
  const [quantityDraftOverrides, setQuantityDraftOverrides] = useState<QuantityDraftMap>({})

  const summaryCards = useMemo(
    () =>
      assetDashboard.summary
        ? buildAssetDashboardSummaryCards(assetDashboard.summary)
        : [],
    [assetDashboard.summary],
  )

  const quantityRowsByStockItemId = useMemo(
    () =>
      new Map(
        assetDashboard.quantityRows.map((row) => [row.stockItemId, row] as const),
      ),
    [assetDashboard.quantityRows],
  )

  const quantityDrafts = useMemo(
    () =>
      Object.fromEntries(
        assetDashboard.quantityRows.map((row) => [
          row.stockItemId,
          quantityDraftOverrides[row.stockItemId] ?? {
            quantityOnHand: String(row.quantityOnHand),
            assignedQuantity: String(row.assignedQuantity),
          },
        ]),
      ) as QuantityDraftMap,
    [assetDashboard.quantityRows, quantityDraftOverrides],
  )

  const updateQuantityDraft = (
    stockItemId: number,
    fieldKey: "quantityOnHand" | "assignedQuantity",
    value: string,
  ) => {
    const row = quantityRowsByStockItemId.get(stockItemId)
    setQuantityDraftOverrides((current) => ({
      ...current,
      [stockItemId]: {
        quantityOnHand:
          current[stockItemId]?.quantityOnHand ??
          String(row?.quantityOnHand ?? 0),
        assignedQuantity:
          current[stockItemId]?.assignedQuantity ??
          String(row?.assignedQuantity ?? 0),
        [fieldKey]: value,
      },
    }))
  }

  const clearQuantityDraft = (stockItemId: number) => {
    setQuantityDraftOverrides((current) => {
      const next = { ...current }
      delete next[stockItemId]
      return next
    })
  }

  return (
    <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="max-w-3xl">
          <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
            <Boxes size={16} />
            Asset Dashboard
          </div>
          <p className="mt-1 text-sm text-[var(--text-secondary)]">
            {getAssetDashboardDescription()}
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
          <button
            className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
            onClick={() => void assetDashboard.refreshDashboard()}
            type="button"
            disabled={assetDashboard.isRefreshingDashboard || assetDashboard.isLoadingDashboard}
          >
            <RefreshCw
              className={
                assetDashboard.isRefreshingDashboard || assetDashboard.isLoadingDashboard
                  ? "animate-spin"
                  : undefined
              }
              size={14}
            />
            {assetDashboard.isRefreshingDashboard || assetDashboard.isLoadingDashboard
              ? "Refreshing..."
              : "Refresh"}
          </button>
          <button
            className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-xs font-semibold text-[#00131c] disabled:opacity-50"
            onClick={assetImport.openImportWizard}
            type="button"
            disabled={!auth.canImportData}
          >
            <FileSpreadsheet size={14} />
            Open Import Wizard
          </button>
          <button
            className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
            onClick={assetImport.openManualAssetPanel}
            type="button"
            disabled={!auth.isAdminAccount}
          >
            <PlusCircle size={14} />
            Add Serialized Asset
          </button>
        </div>
      </div>

      {assetDashboard.statusMessage && (
        <div className="mt-4 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
          {assetDashboard.statusMessage}
        </div>
      )}

      {assetImport.activeBatchSummary && (
        <div className="mt-4 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
          <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div>
              <div className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                Active Import Batch
              </div>
              <div className="mt-1 text-sm text-[var(--text-primary)]">
                {assetImport.activeBatchSummary.batchKey} · {assetImport.activeBatchSummary.sourceFileName}
              </div>
              <div className="mt-1 text-[11px] text-[var(--text-secondary)]">
                {getAssetImportModeLabel(assetImport.activeBatchSummary.importType)} ·{" "}
                {getAssetImportSummaryLabel("valid")} {assetImport.activeBatchSummary.validRows} ·{" "}
                {getAssetImportSummaryLabel("errors")} {assetImport.activeBatchSummary.errorRows} ·{" "}
                {getAssetImportSummaryLabel("imported")} {assetImport.activeBatchSummary.importedRows}
              </div>
            </div>
            <button
              className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)]"
              onClick={() => void assetImport.openBatchDetail(assetImport.activeBatchSummary!.id)}
              type="button"
            >
              Resume Review
            </button>
          </div>
        </div>
      )}

      <div className="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-5">
        {assetDashboard.isLoadingDashboard && !assetDashboard.summary
          ? Array.from({ length: 5 }, (_, index) => (
              <div
                key={index}
                className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-4"
              >
                <div className="h-3 w-24 animate-pulse rounded bg-[var(--surface-hover)]" />
                <div className="mt-3 h-8 w-16 animate-pulse rounded bg-[var(--surface-hover)]" />
              </div>
            ))
          : summaryCards.map((card) => (
              <div
                key={card.key}
                className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-4"
              >
                <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                  {card.label}
                </div>
                <div className="mt-2 text-[26px] font-bold text-[var(--text-primary)]">
                  {card.value}
                </div>
              </div>
            ))}
      </div>

      <div className="mt-5 flex flex-wrap gap-2 border-b border-[var(--border)] pb-3">
        {(["serialized", "quantity"] as const).map((tab) => (
          <button
            key={tab}
            className={`rounded-[999px] border px-3 py-1.5 text-xs font-semibold transition ${
              activeTab === tab
                ? "border-[var(--primary)]/45 bg-[var(--primary)]/10 text-[var(--primary)]"
                : "border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
            }`}
            onClick={() => setActiveTab(tab)}
            type="button"
          >
            {getAssetDashboardTabLabel(tab)}
          </button>
        ))}
      </div>

      <div className="mt-4">
        {activeTab === "serialized" ? (
          <SerializedDashboardTable assetDashboard={assetDashboard} />
        ) : (
          <QuantityDashboardTable
            auth={auth}
            assetDashboard={assetDashboard}
            quantityDrafts={quantityDrafts}
            updateQuantityDraft={updateQuantityDraft}
            clearQuantityDraft={clearQuantityDraft}
          />
        )}
      </div>
    </div>
  )
}

function SerializedDashboardTable({
  assetDashboard,
}: {
  assetDashboard: AssetDashboardState
}) {
  if (assetDashboard.isLoadingDashboard && assetDashboard.serializedRows.length === 0) {
    return (
      <div className="flex items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 px-4 py-4 text-sm text-[var(--text-secondary)]">
        <LoaderCircle className="animate-spin" size={16} />
        Loading serialized assets...
      </div>
    )
  }

  if (assetDashboard.serializedRows.length === 0) {
    return (
      <div className="rounded-[10px] border border-dashed border-[var(--border)] bg-[var(--surface-hover)]/20 px-4 py-8 text-center text-sm text-[var(--text-secondary)]">
        {getAssetDashboardEmptyStateLabel("serialized")}
      </div>
    )
  }

  return (
    <div className="overflow-hidden rounded-[10px] border border-[var(--border)]">
      <div className="overflow-x-auto">
        <table className="min-w-full text-left text-sm">
          <thead className="bg-[var(--surface-hover)]/25 text-xs uppercase tracking-[0.04em] text-[var(--text-secondary)]">
            <tr>
              <th className="px-3 py-3 font-semibold">Asset Code</th>
              <th className="px-3 py-3 font-semibold">Category</th>
              <th className="px-3 py-3 font-semibold">Display</th>
              <th className="px-3 py-3 font-semibold">Model</th>
              <th className="px-3 py-3 font-semibold">Serial</th>
              <th className="px-3 py-3 font-semibold">Usage</th>
              <th className="px-3 py-3 font-semibold">Status</th>
              <th className="px-3 py-3 font-semibold">Holder</th>
            </tr>
          </thead>
          <tbody>
            {assetDashboard.serializedRows.map((row) => (
              <tr key={row.assetId} className="border-t border-[var(--border)] align-top">
                <td className="whitespace-nowrap px-3 py-3 font-semibold text-[var(--text-primary)]">
                  {row.assetCode}
                </td>
                <td className="px-3 py-3 text-[var(--text-secondary)]">
                  {row.categoryName ?? row.categoryCode ?? "—"}
                </td>
                <td className="px-3 py-3 text-[var(--text-primary)]">
                  <div className="whitespace-pre-line">
                    {formatAssetDashboardDisplayNameLines(
                      row.assetCode,
                      row.displayNameShort,
                      row.displayName,
                    )}
                  </div>
                </td>
                <td className="px-3 py-3 text-[var(--text-secondary)]">{row.model ?? "—"}</td>
                <td className="px-3 py-3 text-[var(--text-secondary)]">{row.serialNumber ?? "—"}</td>
                <td className="px-3 py-3 text-[var(--text-secondary)]">
                  {formatAssetDashboardUsageLocationLabel(row.usageLocation)}
                </td>
                <td className="px-3 py-3">
                  <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[11px] font-semibold capitalize text-[var(--text-primary)]">
                    {formatAssetDashboardStatusLabel(row.status)}
                  </span>
                </td>
                <td className="px-3 py-3 text-[var(--text-secondary)]">
                  <div className="whitespace-pre-line">
                    {formatAssetDashboardHolderLabel(
                      row.holderFullName,
                      row.holderEmployeeId,
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

function QuantityDashboardTable({
  auth,
  assetDashboard,
  quantityDrafts,
  updateQuantityDraft,
  clearQuantityDraft,
}: {
  auth: Pick<AuthState, "isAdminAccount">
  assetDashboard: AssetDashboardState
  quantityDrafts: QuantityDraftMap
  updateQuantityDraft: (
    stockItemId: number,
    fieldKey: "quantityOnHand" | "assignedQuantity",
    value: string,
  ) => void
  clearQuantityDraft: (stockItemId: number) => void
}) {
  if (assetDashboard.isLoadingDashboard && assetDashboard.quantityRows.length === 0) {
    return (
      <div className="flex items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 px-4 py-4 text-sm text-[var(--text-secondary)]">
        <LoaderCircle className="animate-spin" size={16} />
        Loading quantity stock...
      </div>
    )
  }

  if (assetDashboard.quantityRows.length === 0) {
    return (
      <div className="rounded-[10px] border border-dashed border-[var(--border)] bg-[var(--surface-hover)]/20 px-4 py-8 text-center text-sm text-[var(--text-secondary)]">
        {getAssetDashboardEmptyStateLabel("quantity")}
      </div>
    )
  }

  return (
    <div className="overflow-hidden rounded-[10px] border border-[var(--border)]">
      <div className="overflow-x-auto">
        <table className="min-w-full text-left text-sm">
          <thead className="bg-[var(--surface-hover)]/25 text-xs uppercase tracking-[0.04em] text-[var(--text-secondary)]">
            <tr>
              <th className="px-3 py-3 font-semibold">Category</th>
              <th className="px-3 py-3 font-semibold">Item Name</th>
              <th className="px-3 py-3 font-semibold">Brand</th>
              <th className="px-3 py-3 font-semibold">Model</th>
              <th className="px-3 py-3 font-semibold">Warehouse</th>
              <th className="px-3 py-3 font-semibold">On Hand</th>
              <th className="px-3 py-3 font-semibold">Assigned</th>
              <th className="px-3 py-3 font-semibold">Note</th>
              <th className="px-3 py-3 font-semibold">Action</th>
            </tr>
          </thead>
          <tbody>
            {assetDashboard.quantityRows.map((row) => {
              const draft = quantityDrafts[row.stockItemId] ?? {
                quantityOnHand: String(row.quantityOnHand),
                assignedQuantity: String(row.assignedQuantity),
              }
              const parsedOnHand = parseAssetDashboardQuantityDraft(draft.quantityOnHand)
              const parsedAssigned = parseAssetDashboardQuantityDraft(
                draft.assignedQuantity,
              )
              const hasValidDraft = parsedOnHand != null && parsedAssigned != null
              const isDirty =
                draft.quantityOnHand !== String(row.quantityOnHand) ||
                draft.assignedQuantity !== String(row.assignedQuantity)
              const isSaving = assetDashboard.isUpdatingStockItemId === row.stockItemId

              return (
                <tr key={row.stockItemId} className="border-t border-[var(--border)] align-top">
                  <td className="px-3 py-3 text-[var(--text-secondary)]">{row.categoryName}</td>
                  <td className="px-3 py-3 font-semibold text-[var(--text-primary)]">{row.itemName}</td>
                  <td className="px-3 py-3 text-[var(--text-secondary)]">{row.brand ?? "—"}</td>
                  <td className="px-3 py-3 text-[var(--text-secondary)]">{row.model ?? "—"}</td>
                  <td className="px-3 py-3 text-[var(--text-secondary)]">{row.warehouse ?? "—"}</td>
                  <td className="px-3 py-3">
                    <input
                      className="form-input min-w-[110px] text-xs"
                      inputMode="numeric"
                      min={0}
                      step={1}
                      type="number"
                      value={draft.quantityOnHand}
                      onChange={(event) =>
                        updateQuantityDraft(
                          row.stockItemId,
                          "quantityOnHand",
                          event.target.value,
                        )
                      }
                      disabled={!auth.isAdminAccount || isSaving}
                    />
                  </td>
                  <td className="px-3 py-3">
                    <input
                      className="form-input min-w-[110px] text-xs"
                      inputMode="numeric"
                      min={0}
                      step={1}
                      type="number"
                      value={draft.assignedQuantity}
                      onChange={(event) =>
                        updateQuantityDraft(
                          row.stockItemId,
                          "assignedQuantity",
                          event.target.value,
                        )
                      }
                      disabled={!auth.isAdminAccount || isSaving}
                    />
                  </td>
                  <td className="px-3 py-3 text-[var(--text-secondary)]">{row.note ?? "—"}</td>
                  <td className="px-3 py-3">
                    <button
                      className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                      onClick={async () => {
                        if (parsedOnHand == null || parsedAssigned == null) {
                          return
                        }
                        const updated = await assetDashboard.updateStockItemQuantity({
                          stockItemId: row.stockItemId,
                          quantityOnHand: parsedOnHand,
                          assignedQuantity: parsedAssigned,
                        })
                        if (updated) {
                          clearQuantityDraft(row.stockItemId)
                        }
                      }}
                      type="button"
                      disabled={!auth.isAdminAccount || !hasValidDraft || !isDirty || isSaving}
                    >
                      {isSaving ? "Saving..." : "Save"}
                    </button>
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </div>
  )
}
