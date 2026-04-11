import type { AssetDashboardSerializedRecord } from "../../types/staff"

export type SerializedAssetColumnKey =
  | "id"
  | "category"
  | "computerName"
  | "assetName"
  | "model"
  | "serialNumber"
  | "adapterNumber"
  | "usageLocation"
  | "note"
  | "status"
  | "holder"

export type SerializedAssetGridSort = {
  key: SerializedAssetColumnKey | null
  direction: "asc" | "desc" | null
}

export type SerializedAssetColumnDefinition = {
  key: SerializedAssetColumnKey
  label: string
  defaultWidth: number
  minWidth: number
}

export const DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER: SerializedAssetColumnKey[] = [
  "id",
  "category",
  "computerName",
  "assetName",
  "model",
  "serialNumber",
  "adapterNumber",
  "usageLocation",
  "note",
  "status",
  "holder",
]

export const SERIALIZED_ASSET_COLUMN_MAP: Record<
  SerializedAssetColumnKey,
  SerializedAssetColumnDefinition
> = {
  id: { key: "id", label: "ID", defaultWidth: 160, minWidth: 120 },
  category: { key: "category", label: "Category", defaultWidth: 120, minWidth: 110 },
  computerName: {
    key: "computerName",
    label: "Computer Name",
    defaultWidth: 190,
    minWidth: 150,
  },
  assetName: {
    key: "assetName",
    label: "Asset Name",
    defaultWidth: 190,
    minWidth: 150,
  },
  model: { key: "model", label: "Model", defaultWidth: 160, minWidth: 130 },
  serialNumber: {
    key: "serialNumber",
    label: "Serial Number",
    defaultWidth: 150,
    minWidth: 130,
  },
  adapterNumber: {
    key: "adapterNumber",
    label: "Adapter Number",
    defaultWidth: 150,
    minWidth: 130,
  },
  usageLocation: {
    key: "usageLocation",
    label: "Usage Location",
    defaultWidth: 150,
    minWidth: 130,
  },
  note: { key: "note", label: "Note", defaultWidth: 180, minWidth: 140 },
  status: { key: "status", label: "Status", defaultWidth: 120, minWidth: 100 },
  holder: { key: "holder", label: "Holder", defaultWidth: 180, minWidth: 150 },
}

const STORAGE_PREFIX = "staffkit:asset-dashboard-serialized-grid"

export function buildSerializedAssetGridStorageKeys(activeUserScope: string) {
  return {
    order: `${STORAGE_PREFIX}:${activeUserScope}:order`,
    widths: `${STORAGE_PREFIX}:${activeUserScope}:widths`,
  }
}

export function cycleSerializedAssetSort(
  current: SerializedAssetGridSort,
  key: SerializedAssetColumnKey,
): SerializedAssetGridSort {
  if (current.key !== key) {
    return { key, direction: "asc" }
  }
  if (current.direction === "asc") {
    return { key, direction: "desc" }
  }
  return { key: null, direction: null }
}

export function sortSerializedAssetRows(
  rows: AssetDashboardSerializedRecord[],
  sort: SerializedAssetGridSort,
): AssetDashboardSerializedRecord[] {
  if (!sort.key || !sort.direction) {
    return rows
  }

  const direction = sort.direction === "asc" ? 1 : -1
  const sortKey = sort.key
  return [...rows].sort((left, right) => {
    const leftValue = getComparableValue(left, sortKey)
    const rightValue = getComparableValue(right, sortKey)

    const leftBlank = leftValue.length === 0
    const rightBlank = rightValue.length === 0
    if (leftBlank && rightBlank) {
      return left.assetId - right.assetId
    }
    if (leftBlank) {
      return 1
    }
    if (rightBlank) {
      return -1
    }

    const compared = leftValue.localeCompare(rightValue, undefined, {
      numeric: true,
      sensitivity: "base",
    })
    if (compared !== 0) {
      return compared * direction
    }

    return left.assetId - right.assetId
  })
}

function getComparableValue(
  row: AssetDashboardSerializedRecord,
  key: SerializedAssetColumnKey,
): string {
  switch (key) {
    case "id":
      return normalizeComparableText(row.assetCode)
    case "category":
      return normalizeComparableText(row.categoryName ?? row.categoryCode)
    case "computerName":
      return normalizeComparableText(
        resolveSerializedAssetComputerName(row.assetCode, row.computerName),
      )
    case "assetName":
      return normalizeComparableText(
        resolveSerializedAssetName(row.assetCode, row.displayName, row.displayNameShort),
      )
    case "model":
      return normalizeComparableText(row.model)
    case "serialNumber":
      return normalizeComparableText(row.serialNumber)
    case "adapterNumber":
      return normalizeComparableText(row.adapterNumber)
    case "usageLocation":
      return normalizeComparableText(row.usageLocation)
    case "note":
      return normalizeComparableText(row.notes)
    case "status":
      return normalizeComparableText(row.status)
    case "holder":
      return normalizeComparableText(
        [row.holderFullName, row.holderEmployeeId].filter(Boolean).join(" "),
      )
  }
}

export function resolveSerializedAssetComputerName(
  assetCode: string,
  computerName: string | null | undefined,
): string {
  const normalizedComputerName = computerName?.trim()
  if (normalizedComputerName) {
    return normalizedComputerName
  }

  const normalizedAssetCode = assetCode.trim().toUpperCase()
  if (!normalizedAssetCode) {
    return ""
  }

  return `ASW${normalizedAssetCode}`
}

export function resolveSerializedAssetName(
  assetCode: string,
  displayName: string | null | undefined,
  displayNameShort: string | null | undefined,
): string {
  const normalizedDisplayName = displayName?.trim()
  if (normalizedDisplayName) {
    return normalizedDisplayName
  }

  const normalizedDisplayNameShort = displayNameShort?.trim()
  if (normalizedDisplayNameShort) {
    return normalizedDisplayNameShort
  }

  return assetCode.trim()
}

function normalizeComparableText(value: string | null | undefined): string {
  return value?.trim() ?? ""
}
