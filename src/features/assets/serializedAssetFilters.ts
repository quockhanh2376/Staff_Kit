import type { AssetDashboardSerializedRecord } from "../../types/staff"
import {
  resolveSerializedAssetComputerName,
  resolveSerializedAssetName,
} from "./serializedAssetGridConfig.ts"

export const ALL_SERIALIZED_ASSET_CATEGORY_FILTER = "__all__"

export type SerializedAssetFilterState = {
  searchTerm: string
  categoryFilter: string
}

export function filterSerializedAssetRows(
  rows: AssetDashboardSerializedRecord[],
  filters: SerializedAssetFilterState,
): AssetDashboardSerializedRecord[] {
  const normalizedSearch = normalizeSerializedAssetFilterText(filters.searchTerm)
  const normalizedCategory = normalizeSerializedAssetFilterText(filters.categoryFilter)

  return rows.filter((row) => {
    if (
      normalizedCategory &&
      normalizedCategory !== ALL_SERIALIZED_ASSET_CATEGORY_FILTER
    ) {
      const rowCategory = normalizeSerializedAssetFilterText(row.categoryCode)
      if (rowCategory !== normalizedCategory) {
        return false
      }
    }

    if (!normalizedSearch) {
      return true
    }

    return buildSerializedAssetSearchText(row).includes(normalizedSearch)
  })
}

export function normalizeSerializedAssetFilterText(
  value: string | null | undefined,
): string {
  return value?.trim().normalize("NFD").replace(/\p{M}/gu, "").toLowerCase() ?? ""
}

function buildSerializedAssetSearchText(row: AssetDashboardSerializedRecord): string {
  return [
    row.assetCode,
    resolveSerializedAssetComputerName(row.assetCode, row.computerName),
    resolveSerializedAssetName(row.assetCode, row.displayName, row.displayNameShort),
    row.model,
    row.serialNumber,
    row.holderFullName,
    row.holderEmployeeId,
  ]
    .map((value) => normalizeSerializedAssetFilterText(value))
    .filter(Boolean)
    .join(" ")
}
