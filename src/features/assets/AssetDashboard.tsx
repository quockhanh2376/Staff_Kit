import { useMemo, useState } from "react"
import {
  Boxes,
  FileSpreadsheet,
  LoaderCircle,
  PencilLine,
  PlusCircle,
  RefreshCw,
  Tags,
  Trash2,
} from "lucide-react"
import type { AssetCategoryDetailRecord } from "../../types/staff"
import type { AuthState } from "../auth/useAuthState"
import type { AssetImportState } from "./useAssetImportState"
import type { AssetDashboardState } from "./useAssetDashboardState"
import {
  buildAssetCategoryDraftFromDetail,
  buildAssetDashboardSummaryCards,
  buildEmptyAssetCategoryDraft,
  formatAssetDashboardDisplayNameLines,
  formatAssetDashboardHolderLabel,
  formatAssetDashboardStatusLabel,
  formatAssetDashboardUsageLocationLabel,
  getAssetDashboardDescription,
  getAssetDashboardEmptyStateLabel,
  getAssetDashboardTabLabel,
  parseAssetDashboardQuantityDraft,
  validateAssetCategoryDraft,
  type AssetCategoryDraft,
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
  const [categoryDraft, setCategoryDraft] = useState<AssetCategoryDraft>(() =>
    buildEmptyAssetCategoryDraft(),
  )

  const summaryCards = useMemo(
    () =>
      assetDashboard.summary
        ? buildAssetDashboardSummaryCards(assetDashboard.summary)
        : [],
    [assetDashboard.summary],
  )

  const quantityRowsById = useMemo(
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

  const selectedCategoryDetail = useMemo(
    () =>
      categoryDraft.id == null
        ? null
        : assetDashboard.categoryDetails.find((detail) => detail.id === categoryDraft.id) ?? null,
    [assetDashboard.categoryDetails, categoryDraft.id],
  )

  const categoryValidationErrors = useMemo(
    () => validateAssetCategoryDraft(categoryDraft, assetDashboard.categoryDetails),
    [assetDashboard.categoryDetails, categoryDraft],
  )

  const updateQuantityDraft = (
    stockItemId: number,
    fieldKey: "quantityOnHand" | "assignedQuantity",
    value: string,
  ) => {
    const baseRow = quantityRowsById.get(stockItemId)
    setQuantityDraftOverrides((current) => ({
      ...current,
      [stockItemId]: {
        quantityOnHand:
          current[stockItemId]?.quantityOnHand ??
          String(baseRow?.quantityOnHand ?? 0),
        assignedQuantity:
          current[stockItemId]?.assignedQuantity ??
          String(baseRow?.assignedQuantity ?? 0),
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

  const openNewCategoryDraft = () => {
    setCategoryDraft(buildEmptyAssetCategoryDraft())
    setActiveTab("categories")
  }

  const openExistingCategoryDraft = (detail: AssetCategoryDetailRecord) => {
    setCategoryDraft(buildAssetCategoryDraftFromDetail(detail))
    setActiveTab("categories")
  }

  const resetCategoryDraft = () => {
    if (selectedCategoryDetail) {
      setCategoryDraft(buildAssetCategoryDraftFromDetail(selectedCategoryDetail))
      return
    }

    setCategoryDraft(buildEmptyAssetCategoryDraft())
  }

  const updateCategoryDraft = <K extends keyof AssetCategoryDraft>(
    fieldKey: K,
    value: AssetCategoryDraft[K],
  ) => {
    setCategoryDraft((current) => ({
      ...current,
      [fieldKey]: value,
    }))
  }

  const updateCategoryPrefix = (
    index: number,
    fieldKey: "prefixValue" | "isPrimary",
    value: string | boolean,
  ) => {
    setCategoryDraft((current) => ({
      ...current,
      prefixes: current.prefixes.map((prefix, prefixIndex) =>
        prefixIndex === index
          ? { ...prefix, [fieldKey]: value }
          : fieldKey === "isPrimary"
            ? { ...prefix, isPrimary: false }
            : prefix,
      ),
    }))
  }

  const addCategoryPrefix = () => {
    setCategoryDraft((current) => ({
      ...current,
      prefixes: [
        ...current.prefixes,
        {
          prefixValue: "",
          isPrimary: current.prefixes.length === 0,
        },
      ],
    }))
  }

  const removeCategoryPrefix = (index: number) => {
    setCategoryDraft((current) => {
      const nextPrefixes = current.prefixes.filter((_, prefixIndex) => prefixIndex !== index)
      if (nextPrefixes.length === 0) {
        return {
          ...current,
          prefixes: [{ prefixValue: "", isPrimary: true }],
        }
      }

      if (!nextPrefixes.some((prefix) => prefix.isPrimary)) {
        nextPrefixes[0] = { ...nextPrefixes[0], isPrimary: true }
      }

      return {
        ...current,
        prefixes: nextPrefixes,
      }
    })
  }

  const saveCategoryDraft = async () => {
    if (!auth.isAdminAccount || categoryValidationErrors.length > 0) {
      return
    }

    const savedCategory = await assetDashboard.saveAssetCategory({
      id: categoryDraft.id,
      categoryCode: categoryDraft.categoryCode.trim(),
      categoryName: categoryDraft.categoryName.trim(),
      trackingMode: categoryDraft.trackingMode,
      qrRequired: categoryDraft.qrRequired,
      prefixes: categoryDraft.prefixes.map((prefix) => ({
        prefixValue: prefix.prefixValue.trim(),
        isPrimary: prefix.isPrimary,
      })),
    })

    if (savedCategory) {
      setCategoryDraft(buildAssetCategoryDraftFromDetail(savedCategory))
      setActiveTab("categories")
    }
  }

  const deactivateCategory = async () => {
    if (!auth.isAdminAccount || categoryDraft.id == null) {
      return
    }

    const confirmed = window.confirm(
      `Deactivate category ${categoryDraft.categoryName.trim() || categoryDraft.categoryCode.trim()}? Existing assets and stock stay in place, but new imports will stop matching these prefixes.`,
    )
    if (!confirmed) {
      return
    }

    const updatedCategory = await assetDashboard.deactivateAssetCategory(categoryDraft.id)
    if (updatedCategory) {
      setCategoryDraft(buildEmptyAssetCategoryDraft())
    }
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
            className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
            onClick={() => setActiveTab("categories")}
            type="button"
            disabled={!auth.isAdminAccount}
          >
            <Tags size={14} />
            Manage Categories
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
                {assetImport.activeBatchSummary.batchKey} - {assetImport.activeBatchSummary.sourceFileName}
              </div>
              <div className="mt-1 text-[11px] text-[var(--text-secondary)]">
                {getAssetImportModeLabel(assetImport.activeBatchSummary.importType)} -{" "}
                {getAssetImportSummaryLabel("valid")} {assetImport.activeBatchSummary.validRows} -{" "}
                {getAssetImportSummaryLabel("errors")} {assetImport.activeBatchSummary.errorRows} -{" "}
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
        {(["serialized", "quantity", "categories"] as const).map((tab) => (
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
        ) : activeTab === "quantity" ? (
          <QuantityDashboardTable
            auth={auth}
            assetDashboard={assetDashboard}
            quantityDrafts={quantityDrafts}
            updateQuantityDraft={updateQuantityDraft}
            clearQuantityDraft={clearQuantityDraft}
          />
        ) : (
          <CategoryManagementPanel
            auth={auth}
            assetDashboard={assetDashboard}
            categoryDraft={categoryDraft}
            selectedCategoryDetail={selectedCategoryDetail}
            categoryValidationErrors={categoryValidationErrors}
            openNewCategoryDraft={openNewCategoryDraft}
            openExistingCategoryDraft={openExistingCategoryDraft}
            updateCategoryDraft={updateCategoryDraft}
            updateCategoryPrefix={updateCategoryPrefix}
            addCategoryPrefix={addCategoryPrefix}
            removeCategoryPrefix={removeCategoryPrefix}
            resetCategoryDraft={resetCategoryDraft}
            saveCategoryDraft={saveCategoryDraft}
            deactivateCategory={deactivateCategory}
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
              const parsedAssigned = parseAssetDashboardQuantityDraft(draft.assignedQuantity)
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

function CategoryManagementPanel({
  auth,
  assetDashboard,
  categoryDraft,
  selectedCategoryDetail,
  categoryValidationErrors,
  openNewCategoryDraft,
  openExistingCategoryDraft,
  updateCategoryDraft,
  updateCategoryPrefix,
  addCategoryPrefix,
  removeCategoryPrefix,
  resetCategoryDraft,
  saveCategoryDraft,
  deactivateCategory,
}: {
  auth: Pick<AuthState, "isAdminAccount">
  assetDashboard: AssetDashboardState
  categoryDraft: AssetCategoryDraft
  selectedCategoryDetail: AssetCategoryDetailRecord | null
  categoryValidationErrors: string[]
  openNewCategoryDraft: () => void
  openExistingCategoryDraft: (detail: AssetCategoryDetailRecord) => void
  updateCategoryDraft: <K extends keyof AssetCategoryDraft>(
    fieldKey: K,
    value: AssetCategoryDraft[K],
  ) => void
  updateCategoryPrefix: (
    index: number,
    fieldKey: "prefixValue" | "isPrimary",
    value: string | boolean,
  ) => void
  addCategoryPrefix: () => void
  removeCategoryPrefix: (index: number) => void
  resetCategoryDraft: () => void
  saveCategoryDraft: () => Promise<void>
  deactivateCategory: () => Promise<void>
}) {
  const isTrackingModeLocked =
    selectedCategoryDetail != null &&
    (selectedCategoryDetail.assetCount > 0 || selectedCategoryDetail.stockItemCount > 0)

  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,360px)_minmax(0,1fr)]">
      <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/15 p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="text-sm font-semibold text-[var(--text-primary)]">
              Category List
            </div>
            <div className="mt-1 text-xs text-[var(--text-secondary)]">
              IT can change labels, tracking mode, and workbook prefixes here.
            </div>
          </div>
          <button
            className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
            onClick={openNewCategoryDraft}
            type="button"
            disabled={!auth.isAdminAccount}
          >
            <PlusCircle size={14} />
            New
          </button>
        </div>

        {assetDashboard.isLoadingCategories && assetDashboard.categoryDetails.length === 0 ? (
          <div className="mt-4 flex items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-4 py-4 text-sm text-[var(--text-secondary)]">
            <LoaderCircle className="animate-spin" size={16} />
            Loading categories...
          </div>
        ) : assetDashboard.categoryDetails.length === 0 ? (
          <div className="mt-4 rounded-[10px] border border-dashed border-[var(--border)] bg-[var(--surface)] px-4 py-8 text-center text-sm text-[var(--text-secondary)]">
            {getAssetDashboardEmptyStateLabel("categories")}
          </div>
        ) : (
          <div className="mt-4 space-y-3">
            {assetDashboard.categoryDetails.map((detail) => {
              const isSelected = detail.id === categoryDraft.id
              const prefixSummary =
                detail.prefixes.length > 0
                  ? detail.prefixes
                      .filter((prefix) => prefix.isActive)
                      .map((prefix) =>
                        prefix.isPrimary
                          ? `${prefix.prefixValue} (primary)`
                          : prefix.prefixValue,
                      )
                      .join(", ")
                  : "No prefixes"

              return (
                <button
                  key={detail.id}
                  className={`w-full rounded-[10px] border px-3 py-3 text-left transition ${
                    isSelected
                      ? "border-[var(--primary)]/45 bg-[var(--primary)]/10"
                      : "border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)]"
                  }`}
                  onClick={() => openExistingCategoryDraft(detail)}
                  type="button"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <div className="text-sm font-semibold text-[var(--text-primary)]">
                        {detail.categoryName}
                      </div>
                      <div className="mt-1 text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                        {detail.categoryCode}
                      </div>
                    </div>
                    <div className="flex flex-wrap justify-end gap-1">
                      <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.05em] text-[var(--text-secondary)]">
                        {detail.trackingMode}
                      </span>
                      <span
                        className={`rounded-[999px] border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.05em] ${
                          detail.isActive
                            ? "border-emerald-500/35 bg-emerald-500/10 text-emerald-300"
                            : "border-[var(--border)] text-[var(--text-secondary)]"
                        }`}
                      >
                        {detail.isActive ? "Active" : "Inactive"}
                      </span>
                    </div>
                  </div>
                  <div className="mt-3 text-xs text-[var(--text-secondary)]">
                    Prefixes: {prefixSummary}
                  </div>
                  <div className="mt-2 flex flex-wrap gap-3 text-[11px] text-[var(--text-secondary)]">
                    <span>Assets: {detail.assetCount}</span>
                    <span>Stock items: {detail.stockItemCount}</span>
                    <span>QR required: {detail.qrRequired ? "Yes" : "No"}</span>
                  </div>
                </button>
              )
            })}
          </div>
        )}
      </div>

      <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-4">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
              <PencilLine size={16} />
              {categoryDraft.id == null ? "Create Category" : "Edit Category"}
            </div>
            <div className="mt-1 text-xs text-[var(--text-secondary)]">
              Prefixes drive workbook recognition. One prefix must stay primary.
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)]"
              onClick={resetCategoryDraft}
              type="button"
            >
              Reset
            </button>
            {categoryDraft.id != null && (
              <button
                className="rounded-[8px] border border-rose-500/35 px-3 py-2 text-xs font-semibold text-rose-300 transition hover:bg-rose-500/10 disabled:opacity-50"
                onClick={() => void deactivateCategory()}
                type="button"
                disabled={
                  !auth.isAdminAccount ||
                  assetDashboard.isDeactivatingCategoryId === categoryDraft.id
                }
              >
                {assetDashboard.isDeactivatingCategoryId === categoryDraft.id
                  ? "Deactivating..."
                  : "Deactivate"}
              </button>
            )}
            <button
              className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-xs font-semibold text-[#00131c] disabled:opacity-50"
              onClick={() => void saveCategoryDraft()}
              type="button"
              disabled={
                !auth.isAdminAccount ||
                assetDashboard.isSavingCategory ||
                categoryValidationErrors.length > 0
              }
            >
              {assetDashboard.isSavingCategory ? "Saving..." : "Save Category"}
            </button>
          </div>
        </div>

        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <label className="flex flex-col gap-1 text-xs text-[var(--text-secondary)]">
            Category Code
            <input
              className="form-input"
              value={categoryDraft.categoryCode}
              onChange={(event) => updateCategoryDraft("categoryCode", event.target.value)}
              disabled={!auth.isAdminAccount}
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-[var(--text-secondary)]">
            Category Name
            <input
              className="form-input"
              value={categoryDraft.categoryName}
              onChange={(event) => updateCategoryDraft("categoryName", event.target.value)}
              disabled={!auth.isAdminAccount}
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-[var(--text-secondary)]">
            Tracking Mode
            <select
              className="form-input"
              value={categoryDraft.trackingMode}
              onChange={(event) =>
                updateCategoryDraft(
                  "trackingMode",
                  event.target.value as AssetCategoryDraft["trackingMode"],
                )
              }
              disabled={!auth.isAdminAccount || isTrackingModeLocked}
            >
              <option value="serialized">Serialized</option>
              <option value="quantity">Quantity</option>
            </select>
          </label>
          <label className="flex items-center gap-2 pt-6 text-xs text-[var(--text-secondary)]">
            <input
              checked={categoryDraft.qrRequired}
              onChange={(event) => updateCategoryDraft("qrRequired", event.target.checked)}
              type="checkbox"
              disabled={!auth.isAdminAccount}
            />
            QR required for assigned review flows
          </label>
        </div>

        {isTrackingModeLocked && (
          <div className="mt-3 rounded-[8px] border border-amber-500/25 bg-amber-500/8 px-3 py-2 text-xs text-amber-200">
            Tracking mode is locked because this category already has live assets or stock items.
          </div>
        )}

        <div className="mt-5 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/15 p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="text-sm font-semibold text-[var(--text-primary)]">Prefixes</div>
              <div className="mt-1 text-xs text-[var(--text-secondary)]">
                Add every workbook prefix that should map into this category.
              </div>
            </div>
            <button
              className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
              onClick={addCategoryPrefix}
              type="button"
              disabled={!auth.isAdminAccount}
            >
              Add Prefix
            </button>
          </div>

          <div className="mt-4 space-y-3">
            {categoryDraft.prefixes.map((prefix, index) => (
              <div
                key={`${categoryDraft.id ?? "new"}-${index}`}
                className="grid gap-3 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 py-3 md:grid-cols-[minmax(0,1fr)_120px_52px]"
              >
                <label className="flex flex-col gap-1 text-xs text-[var(--text-secondary)]">
                  Prefix Value
                  <input
                    className="form-input"
                    value={prefix.prefixValue}
                    onChange={(event) =>
                      updateCategoryPrefix(index, "prefixValue", event.target.value)
                    }
                    disabled={!auth.isAdminAccount}
                  />
                </label>
                <label className="flex items-center gap-2 pt-6 text-xs text-[var(--text-secondary)]">
                  <input
                    checked={prefix.isPrimary}
                    onChange={() => updateCategoryPrefix(index, "isPrimary", true)}
                    type="radio"
                    name={`asset-category-primary-${categoryDraft.id ?? "new"}`}
                    disabled={!auth.isAdminAccount}
                  />
                  Primary
                </label>
                <div className="flex items-end justify-end">
                  <button
                    className="inline-flex h-10 w-10 items-center justify-center rounded-[8px] border border-[var(--border)] text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                    onClick={() => removeCategoryPrefix(index)}
                    type="button"
                    disabled={!auth.isAdminAccount || categoryDraft.prefixes.length === 1}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {selectedCategoryDetail && (
          <div className="mt-4 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/15 p-4 text-xs text-[var(--text-secondary)]">
            <div className="font-semibold text-[var(--text-primary)]">Current usage</div>
            <div className="mt-2 flex flex-wrap gap-4">
              <span>Assets: {selectedCategoryDetail.assetCount}</span>
              <span>Stock items: {selectedCategoryDetail.stockItemCount}</span>
              <span>Primary prefix: {selectedCategoryDetail.prefixCode ?? "none"}</span>
              <span>Status: {selectedCategoryDetail.isActive ? "active" : "inactive"}</span>
            </div>
          </div>
        )}

        {categoryValidationErrors.length > 0 && (
          <div className="mt-4 rounded-[10px] border border-amber-500/25 bg-amber-500/8 px-4 py-3 text-xs text-amber-200">
            <div className="font-semibold text-amber-100">Review before saving</div>
            <ul className="mt-2 space-y-1">
              {categoryValidationErrors.map((error) => (
                <li key={error}>- {error}</li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  )
}
