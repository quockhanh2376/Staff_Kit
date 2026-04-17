import { useMemo, useState } from "react"
import {
  ArrowUpDown,
  Boxes,
  FileSpreadsheet,
  GripVertical,
  LoaderCircle,
  PencilLine,
  PlusCircle,
  RefreshCw,
  Search,
  Trash2,
} from "lucide-react"
import type { AssetCategoryDetailRecord } from "../../types/staff"
import type { AuthState } from "../auth/useAuthState"
import type { AssetImportState } from "./useAssetDirectImportState"
import type { AssetDashboardState } from "./useAssetDashboardState"
import {
  buildAssetCategoryDraftFromDetail,
  buildAssetDashboardSummaryCards,
  buildEmptyAssetCategoryDraft,
  formatAssetDashboardHolderLabel,
  formatAssetDashboardStatusLabel,
  formatAssetDashboardUsageLocationLabel,
  getAssetDashboardEmptyStateLabel,
  getAssetDashboardTabLabel,
  parseAssetDashboardQuantityDraft,
  validateAssetCategoryDraft,
  type AssetCategoryDraft,
  type AssetDashboardTabKey,
} from "./assetDashboardCopy"
import {
  resolveSerializedAssetComputerName,
  resolveSerializedAssetName,
  type SerializedAssetColumnKey,
} from "./serializedAssetGridConfig"
import {
  ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  filterSerializedAssetRows,
  normalizeSerializedAssetFilterText,
} from "./serializedAssetFilters"
import { useSerializedAssetGridState } from "./useSerializedAssetGridState"

type AssetDashboardProps = {
  activeUserScope: string
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

const dashboardShellClass =
  "mt-4 rounded-[18px] border border-[#222938] bg-[#151921] px-5 py-5 text-slate-300 shadow-[0_16px_38px_rgba(0,0,0,0.24)]"
const dashboardShellPrimaryButtonClass =
  "inline-flex h-10 items-center gap-2 rounded-md border border-transparent bg-[#00d68f] px-5 text-sm font-semibold text-[#08130f] transition hover:bg-[#17e29a] disabled:cursor-not-allowed disabled:opacity-50"
const dashboardShellSecondaryButtonClass =
  "inline-flex h-10 items-center gap-2 rounded-md border border-[#283140] bg-[#1a202b] px-4 text-sm font-medium text-slate-100 transition hover:bg-[#202737] disabled:cursor-not-allowed disabled:opacity-50"
const dashboardShellIconButtonClass =
  "flex h-10 w-10 items-center justify-center rounded-md border border-[#283140] bg-[#1a202b] text-[#c7cfdb] transition hover:bg-[#202737] disabled:cursor-not-allowed disabled:opacity-50"
const dashboardShellLabelClass =
  "text-[10px] font-semibold uppercase tracking-[0.14em] text-[#8a93a4]"
export function AssetDashboard({
  activeUserScope,
  auth,
  assetDashboard,
  assetImport,
}: AssetDashboardProps) {
  const [activeTab, setActiveTab] = useState<AssetDashboardTabKey>("serialized")
  const [serializedSearchTerm, setSerializedSearchTerm] = useState("")
  const [serializedCategoryFilter, setSerializedCategoryFilter] = useState(
    ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  )
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

  const serializedCategoryOptions = useMemo(() => {
    if (activeTab !== "serialized") {
      return [{ value: ALL_SERIALIZED_ASSET_CATEGORY_FILTER, label: "All Categories" }]
    }

    const options = new Map<string, string>([
      [ALL_SERIALIZED_ASSET_CATEGORY_FILTER, "All Categories"],
    ])

    for (const detail of assetDashboard.categoryDetails) {
      if (detail.trackingMode !== "serialized") {
        continue
      }

      const label = detail.categoryName.trim() || detail.categoryCode.trim()
      const value = normalizeSerializedAssetFilterText(detail.categoryCode)
      if (value && !options.has(value)) {
        options.set(value, label)
      }
    }

    for (const row of assetDashboard.serializedRows) {
      const label = (row.categoryName ?? row.categoryCode ?? "").trim()
      const value = normalizeSerializedAssetFilterText(row.categoryCode)
      if (value && !options.has(value)) {
        options.set(value, label)
      }
    }

    return Array.from(options, ([value, label]) => ({ value, label }))
  }, [activeTab, assetDashboard.categoryDetails, assetDashboard.serializedRows])

  const filteredSerializedRows = useMemo(() => {
    if (activeTab !== "serialized") {
      return assetDashboard.serializedRows
    }

    return filterSerializedAssetRows(assetDashboard.serializedRows, {
        searchTerm: serializedSearchTerm,
        categoryFilter: serializedCategoryFilter,
      })
  }, [activeTab, assetDashboard.serializedRows, serializedCategoryFilter, serializedSearchTerm])

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

  const clearSerializedFilters = () => {
    setSerializedSearchTerm("")
    setSerializedCategoryFilter(ALL_SERIALIZED_ASSET_CATEGORY_FILTER)
  }

  const filterControls = activeTab === "serialized" ? (
    <div className="flex min-w-[320px] flex-1 flex-wrap items-center gap-3">
      <label className="relative min-w-[260px] flex-1">
        <Search
          size={15}
          className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[#8f98a8]"
        />
        <input
          aria-label="Search serialized assets"
          className={`${dashboardInputClass} pl-9`}
          onChange={(event) => setSerializedSearchTerm(event.target.value)}
          placeholder="Search computer, asset code, holder, model..."
          type="text"
          value={serializedSearchTerm}
        />
      </label>
      <select
        aria-label="Filter serialized assets by category"
        className={`${dashboardInputClass} w-full sm:w-[220px]`}
        onChange={(event) => setSerializedCategoryFilter(event.target.value)}
        value={serializedCategoryFilter}
      >
        {serializedCategoryOptions.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      <button
        className={dashboardShellSecondaryButtonClass}
        onClick={clearSerializedFilters}
        type="button"
      >
        Clear Filters
      </button>
    </div>
  ) : null

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
    <div className={dashboardShellClass}>
      <div className="w-full">
        <div className="flex flex-col items-start gap-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Boxes size={16} className="text-[#00d68f]" />
            <span className="text-[17px] font-bold tracking-[0.01em]">Asset Dashboard</span>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <button
              className={dashboardShellIconButtonClass}
              onClick={() => void assetDashboard.refreshDashboard()}
              type="button"
              disabled={assetDashboard.isRefreshingDashboard || assetDashboard.isLoadingDashboard}
              aria-label={
                assetDashboard.isRefreshingDashboard || assetDashboard.isLoadingDashboard
                  ? "Refreshing asset dashboard"
                  : "Refresh asset dashboard"
              }
              aria-busy={
                assetDashboard.isRefreshingDashboard || assetDashboard.isLoadingDashboard
              }
              title="Refresh"
            >
              <RefreshCw
                className={
                  assetDashboard.isRefreshingDashboard || assetDashboard.isLoadingDashboard
                    ? "animate-spin"
                    : undefined
                }
                size={16}
              />
            </button>
            <button
              className={dashboardShellPrimaryButtonClass}
              onClick={assetImport.openImportWizard}
              type="button"
              disabled={!auth.canImportData}
            >
              <FileSpreadsheet size={16} />
              Import Asset
            </button>
            <button
              className={dashboardShellSecondaryButtonClass}
              onClick={assetImport.openManualAssetPanel}
              type="button"
              disabled={!auth.isAdminAccount}
            >
              <PlusCircle size={16} />
              Add Asset
            </button>

            {filterControls}
          </div>

          {assetDashboard.statusMessage && (
            <div className="mt-4 rounded-[10px] border border-emerald-500/25 bg-emerald-500/8 px-3 py-2 text-xs text-emerald-100">
              {assetDashboard.statusMessage}
            </div>
          )}

          <div className="grid w-full gap-4 sm:grid-cols-2 xl:grid-cols-5">
            {assetDashboard.isLoadingDashboard && !assetDashboard.summary
              ? Array.from({ length: 5 }, (_, index) => (
                  <div
                    key={index}
                    className="flex min-h-[98px] flex-col justify-between rounded-[10px] border border-[#252d3b] bg-[#0b0f15] px-5 py-4"
                  >
                    <div className="h-3 w-24 animate-pulse rounded bg-[#1c2430]" />
                    <div className="mt-4 h-8 w-14 animate-pulse rounded bg-[#1c2430]" />
                  </div>
                ))
              : summaryCards.map((card) => (
                  <div
                    key={card.key}
                    className="flex min-h-[98px] flex-col justify-between rounded-[10px] border border-[#252d3b] bg-[#0b0f15] px-5 py-4"
                  >
                    <div className={dashboardShellLabelClass}>{card.label}</div>
                    <div className="mt-4 text-[28px] font-bold leading-none text-white">{card.value}</div>
                  </div>
                ))}
          </div>

          <div className="flex flex-wrap items-center gap-3">
            {(["serialized", "quantity", "categories"] as const).map((tab) => (
              <button
                key={tab}
                className={
                  activeTab === tab
                    ? "rounded-full border border-[#00d68f] bg-[#0d1f18] px-5 py-2 text-sm font-semibold text-[#00d68f]"
                    : "rounded-full border border-[#293244] bg-[#1a202b] px-5 py-2 text-sm font-semibold text-[#8f98a8] transition hover:border-[#354055] hover:bg-[#202737] hover:text-slate-100"
                }
                onClick={() => setActiveTab(tab)}
                type="button"
              >
                {getAssetDashboardTabLabel(tab)}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="mt-6">
        {activeTab === "serialized" ? (
          <SerializedDashboardTable
            key={activeUserScope}
            activeUserScope={activeUserScope}
            assetDashboard={assetDashboard}
            filteredRows={filteredSerializedRows}
          />
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

const dashboardSurfaceClass =
  "rounded-[12px] border border-slate-800 bg-[#1c2128]"
const dashboardInnerSurfaceClass =
  "rounded-[10px] border border-slate-800 bg-[#0d1117]"
const dashboardPrimaryButtonClass =
  "inline-flex items-center gap-2 rounded-[10px] border border-emerald-500/70 bg-emerald-500 px-3 py-2 text-xs font-semibold text-[#03130d] transition hover:bg-emerald-400 disabled:opacity-50"
const dashboardSecondaryButtonClass =
  "inline-flex items-center gap-2 rounded-[10px] border border-slate-700 bg-slate-800 px-3 py-2 text-xs font-semibold text-slate-100 transition hover:bg-slate-700 disabled:opacity-50"
const dashboardMutedTextClass = "text-slate-400"
const dashboardInputClass =
  "w-full rounded-[10px] border border-slate-800 bg-[#0d1117] px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 outline-none transition focus:border-emerald-500/45"
const dashboardTableShellClass =
  "overflow-hidden rounded-[12px] border border-slate-800 bg-[#0d1117]"
const dashboardTableHeadClass =
  "bg-[#1c2128] text-[11px] uppercase tracking-[0.08em] text-slate-400"

function SerializedDashboardTable({
  activeUserScope,
  assetDashboard,
  filteredRows,
}: {
  activeUserScope: string
  assetDashboard: AssetDashboardState
  filteredRows: AssetDashboardState["serializedRows"]
}) {
  const {
    orderedColumns,
    sortedRows,
    effectiveWidths,
    sort,
    draggingColumnKey,
    toggleSort,
    handleHeaderDragStart,
    handleHeaderDrop,
    beginColumnResize,
    setDraggingColumnKey,
  } = useSerializedAssetGridState({
    activeUserScope,
    rows: filteredRows,
  })

  if (assetDashboard.isLoadingDashboard && assetDashboard.serializedRows.length === 0) {
    return (
      <div className={`flex items-center gap-2 px-4 py-4 text-sm ${dashboardInnerSurfaceClass} ${dashboardMutedTextClass}`}>
        <LoaderCircle className="animate-spin" size={16} />
        Loading serialized assets...
      </div>
    )
  }

  if (assetDashboard.serializedRows.length === 0) {
    return (
      <div className="rounded-[12px] border border-dashed border-[#31394a] bg-[#0b0f15] px-4 py-8 text-center text-sm text-[#8f98a8]">
        {getAssetDashboardEmptyStateLabel("serialized")}
      </div>
    )
  }

  return (
    <div className="space-y-3">
      {filteredRows.length === 0 ? (
        <div className="space-y-3">
          <div className="rounded-[12px] border border-dashed border-[#31394a] bg-[#0b0f15] px-4 py-8 text-center text-sm text-[#8f98a8]">
            No serialized assets match the current filters.
          </div>
        </div>
      ) : (
        <>
          <div className={dashboardTableShellClass}>
            <div className="overflow-x-auto">
              <table className="min-w-max text-left text-[13px]">
              <thead className={dashboardTableHeadClass}>
                <tr>
                  {orderedColumns.map((column) => {
                    const sortIndicator =
                      sort.key !== column.key ? (
                        <ArrowUpDown size={12} />
                      ) : sort.direction === "asc" ? (
                        <span className="text-[10px]">ASC</span>
                      ) : (
                        <span className="text-[10px]">DESC</span>
                      )

                    return (
                      <th
                        key={column.key}
                        className={`group relative border-r border-slate-800 last:border-r-0 ${
                          draggingColumnKey === column.key ? "bg-slate-800/85" : ""
                        }`}
                        style={{
                          minWidth: column.minWidth,
                          width: effectiveWidths[column.key],
                        }}
                        draggable
                        onDragStart={() => handleHeaderDragStart(column.key)}
                        onDragOver={(event) => event.preventDefault()}
                        onDrop={() => handleHeaderDrop(column.key)}
                        onDragEnd={() => setDraggingColumnKey(null)}
                      >
                        <button
                          className="flex w-full items-center gap-2 px-3 py-2.5 pr-5 text-left font-semibold"
                          onClick={() => toggleSort(column.key)}
                          type="button"
                        >
                          <GripVertical
                            size={12}
                            className="shrink-0 text-slate-600 transition group-hover:text-slate-500"
                          />
                          <span className="truncate">{column.label}</span>
                          <span className="ml-auto inline-flex shrink-0 items-center text-slate-500">
                            {sortIndicator}
                          </span>
                        </button>
                        <span
                          className="absolute inset-y-0 right-0 flex w-3 cursor-col-resize items-center justify-center"
                          onMouseDown={(event) => beginColumnResize(column.key, event)}
                        >
                          <span className="h-5 w-px bg-slate-700 transition group-hover:bg-emerald-400/60" />
                        </span>
                      </th>
                    )
                  })}
                </tr>
              </thead>
              <tbody>
                {sortedRows.map((row) => (
                  <tr key={row.assetId} className="border-t border-[#202736] align-top">
                    {orderedColumns.map((column) => (
                      <td
                        key={`${row.assetId}-${column.key}`}
                        className={`px-3 py-2.5 align-top ${
                          column.key === "id" || column.key === "assetName"
                            ? "font-semibold text-slate-100"
                            : dashboardMutedTextClass
                        }`}
                        style={{
                          minWidth: column.minWidth,
                          width: effectiveWidths[column.key],
                        }}
                      >
                        {renderSerializedCellValue(column.key, row)}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
            </div>
          </div>
        </>
      )}
    </div>
  )
}

function renderSerializedCellValue(
  columnKey: SerializedAssetColumnKey,
  row: AssetDashboardState["serializedRows"][number],
) {
  switch (columnKey) {
    case "id":
      return <span className="whitespace-nowrap">{row.assetCode}</span>
    case "category":
      return row.categoryName ?? row.categoryCode ?? "\u2014"
    case "computerName":
      return (
        <span className="whitespace-nowrap text-slate-100">
          {resolveSerializedAssetComputerName(row.assetCode, row.computerName) || "\u2014"}
        </span>
      )
    case "assetName":
      return resolveSerializedAssetName(
        row.assetCode,
        row.displayName,
        row.displayNameShort,
      ) || "\u2014"
    case "model":
      return row.model ?? "\u2014"
    case "serialNumber":
      return row.serialNumber ?? "\u2014"
    case "adapterNumber":
      return row.adapterNumber ?? "\u2014"
    case "usageLocation":
      return formatAssetDashboardUsageLocationLabel(row.usageLocation)
    case "note":
      return row.notes ?? "\u2014"
    case "status":
      return (
        <span className="rounded-[999px] border border-slate-700 bg-slate-900/70 px-2 py-0.5 text-[11px] font-semibold capitalize text-slate-100">
          {formatAssetDashboardStatusLabel(row.status)}
        </span>
      )
    case "holder":
      return (
        <div className="whitespace-pre-line">
          {formatAssetDashboardHolderLabel(row.holderFullName, row.holderEmployeeId)}
        </div>
      )
  }
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
      <div className={`flex items-center gap-2 px-4 py-4 text-sm ${dashboardInnerSurfaceClass} ${dashboardMutedTextClass}`}>
        <LoaderCircle className="animate-spin" size={16} />
        Loading quantity stock...
      </div>
    )
  }

  if (assetDashboard.quantityRows.length === 0) {
    return (
      <div className="rounded-[12px] border border-dashed border-[#31394a] bg-[#0b0f15] px-4 py-8 text-center text-sm text-[#8f98a8]">
        {getAssetDashboardEmptyStateLabel("quantity")}
      </div>
    )
  }

  return (
    <div className={dashboardTableShellClass}>
      <div className="overflow-x-auto">
        <table className="min-w-full text-left text-[13px]">
          <thead className={dashboardTableHeadClass}>
            <tr>
              <th className="px-3 py-2.5 font-semibold">Category</th>
              <th className="px-3 py-2.5 font-semibold">Item Name</th>
              <th className="px-3 py-2.5 font-semibold">Brand</th>
              <th className="px-3 py-2.5 font-semibold">Model</th>
              <th className="px-3 py-2.5 font-semibold">Warehouse</th>
              <th className="px-3 py-2.5 font-semibold">On Hand</th>
              <th className="px-3 py-2.5 font-semibold">Assigned</th>
              <th className="px-3 py-2.5 font-semibold">Note</th>
              <th className="px-3 py-2.5 font-semibold">Action</th>
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
                <tr key={row.stockItemId} className="border-t border-[#202736] align-top">
                  <td className={`px-3 py-2.5 ${dashboardMutedTextClass}`}>{row.categoryName}</td>
                  <td className="px-3 py-2.5 font-semibold text-slate-100">{row.itemName}</td>
                  <td className={`px-3 py-2.5 ${dashboardMutedTextClass}`}>{row.brand ?? "—"}</td>
                  <td className={`px-3 py-2.5 ${dashboardMutedTextClass}`}>{row.model ?? "—"}</td>
                  <td className={`px-3 py-2.5 ${dashboardMutedTextClass}`}>{row.warehouse ?? "—"}</td>
                  <td className="px-3 py-2.5">
                    <input
                      className={`${dashboardInputClass} min-w-[110px] py-1.5 text-xs`}
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
                  <td className="px-3 py-2.5">
                    <input
                      className={`${dashboardInputClass} min-w-[110px] py-1.5 text-xs`}
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
                  <td className={`px-3 py-2.5 ${dashboardMutedTextClass}`}>{row.note ?? "—"}</td>
                  <td className="px-3 py-2.5">
                    <button
                      className={dashboardSecondaryButtonClass}
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
    <div className="grid gap-4 xl:grid-cols-[minmax(0,320px)_minmax(0,1fr)]">
      <div className={dashboardSurfaceClass}>
        <div className="p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="text-sm font-semibold text-slate-100">
              Category List
            </div>
            <div className={`mt-1 text-xs ${dashboardMutedTextClass}`}>
              IT can update names, tracking mode, and prefixes here.
            </div>
          </div>
          <button
            className={dashboardSecondaryButtonClass}
            onClick={openNewCategoryDraft}
            type="button"
            disabled={!auth.isAdminAccount}
          >
            <PlusCircle size={14} />
            New
          </button>
        </div>

        {assetDashboard.isLoadingCategories && assetDashboard.categoryDetails.length === 0 ? (
          <div className={`mt-4 flex items-center gap-2 px-4 py-4 text-sm ${dashboardInnerSurfaceClass} ${dashboardMutedTextClass}`}>
            <LoaderCircle className="animate-spin" size={16} />
            Loading categories...
          </div>
        ) : assetDashboard.categoryDetails.length === 0 ? (
          <div className={`mt-4 rounded-[12px] border border-dashed border-slate-700 bg-[#0d1117] px-4 py-8 text-center text-sm ${dashboardMutedTextClass}`}>
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
                      ? "border-[#00d68f] bg-[#0d1f18]"
                      : "border-[#283140] bg-[#0f141c] hover:bg-[#151b25]"
                  }`}
                  onClick={() => openExistingCategoryDraft(detail)}
                  type="button"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <div className="text-sm font-semibold text-slate-100">
                        {detail.categoryName}
                      </div>
                      <div className={`mt-1 text-[11px] uppercase tracking-[0.06em] ${dashboardMutedTextClass}`}>
                        {detail.categoryCode}
                      </div>
                    </div>
                    <div className="flex flex-wrap justify-end gap-1">
                      <span className="rounded-[999px] border border-[#31394a] bg-[#141a23] px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.05em] text-[#8f98a8]">
                        {detail.trackingMode}
                      </span>
                      <span
                        className={`rounded-[999px] border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.05em] ${
                          detail.isActive
                            ? "border-[#00d68f]/30 bg-[#0d1f18] text-[#55d8a5]"
                            : "border-[#31394a] text-[#8f98a8]"
                        }`}
                      >
                        {detail.isActive ? "Active" : "Inactive"}
                      </span>
                    </div>
                  </div>
                  <div className={`mt-3 text-xs ${dashboardMutedTextClass}`}>
                    Prefixes: {prefixSummary}
                  </div>
                  <div className={`mt-2 flex flex-wrap gap-3 text-[11px] ${dashboardMutedTextClass}`}>
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
      </div>

      <div className="rounded-[12px] border border-[#252d3b] bg-[#11161f] p-4">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="flex items-center gap-2 text-sm font-semibold text-slate-100">
              <PencilLine size={16} />
              {categoryDraft.id == null ? "Create Category" : "Edit Category"}
            </div>
            <div className={`mt-1 text-xs ${dashboardMutedTextClass}`}>
              Prefixes decide how imported workbook rows map into a category.
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              className={dashboardSecondaryButtonClass}
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
              className={dashboardPrimaryButtonClass}
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
          <label className={`flex flex-col gap-1 text-xs ${dashboardMutedTextClass}`}>
            Category Code
            <input
              className={dashboardInputClass}
              value={categoryDraft.categoryCode}
              onChange={(event) => updateCategoryDraft("categoryCode", event.target.value)}
              disabled={!auth.isAdminAccount}
            />
          </label>
          <label className={`flex flex-col gap-1 text-xs ${dashboardMutedTextClass}`}>
            Category Name
            <input
              className={dashboardInputClass}
              value={categoryDraft.categoryName}
              onChange={(event) => updateCategoryDraft("categoryName", event.target.value)}
              disabled={!auth.isAdminAccount}
            />
          </label>
          <label className={`flex flex-col gap-1 text-xs ${dashboardMutedTextClass}`}>
            Tracking Mode
            <select
              className={dashboardInputClass}
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
          <label className={`flex items-center gap-2 pt-6 text-xs ${dashboardMutedTextClass}`}>
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

        <div className="mt-5 rounded-[12px] border border-[#252d3b] bg-[#11161f] p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="text-sm font-semibold text-slate-100">Prefixes</div>
              <div className={`mt-1 text-xs ${dashboardMutedTextClass}`}>
                Add every workbook prefix that should map into this category.
              </div>
            </div>
            <button
              className={dashboardSecondaryButtonClass}
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
                className="grid gap-3 rounded-[10px] border border-[#283140] bg-[#0f141c] px-3 py-3 md:grid-cols-[minmax(0,1fr)_120px_52px]"
              >
                <label className={`flex flex-col gap-1 text-xs ${dashboardMutedTextClass}`}>
                  Prefix Value
                  <input
                    className={dashboardInputClass}
                    value={prefix.prefixValue}
                    onChange={(event) =>
                      updateCategoryPrefix(index, "prefixValue", event.target.value)
                    }
                    disabled={!auth.isAdminAccount}
                  />
                </label>
                <label className={`flex items-center gap-2 pt-6 text-xs ${dashboardMutedTextClass}`}>
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
                    className="inline-flex h-10 w-10 items-center justify-center rounded-[8px] border border-[#31394a] bg-[#141a23] text-[#8f98a8] transition hover:bg-[#1b2230] disabled:opacity-50"
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
          <div className={`mt-4 p-4 text-xs ${dashboardInnerSurfaceClass} ${dashboardMutedTextClass}`}>
            <div className="font-semibold text-slate-100">Current usage</div>
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
