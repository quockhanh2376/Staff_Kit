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
  if (!normalizedSearch) {
    return rows
  }

  return rows.filter((row) => buildSerializedAssetSearchText(row).includes(normalizedSearch))
}

export function normalizeSerializedAssetFilterText(
  value: string | null | undefined,
): string {
  return value?.trim().toLowerCase() ?? ""
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
