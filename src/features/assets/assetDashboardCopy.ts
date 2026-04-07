import type { AssetDashboardSummary } from "../../types/staff"
import {
  normalizeAssetDashboardUsageLocation,
  resolveAssetDashboardDisplayNameShort,
} from "./assetImportModeConfig.ts"

export type AssetDashboardTabKey = "serialized" | "quantity"

type AssetDashboardSummaryCard = {
  key: keyof AssetDashboardSummary
  label: string
  value: number
}

const SUMMARY_CARD_LABELS: Record<keyof AssetDashboardSummary, string> = {
  totalSerializedAssets: "Total Serialized",
  serializedInStock: "Serialized In Stock",
  serializedAssigned: "Serialized Assigned",
  totalQuantityOnHand: "Qty On Hand",
  totalQuantityAssigned: "Qty Assigned",
}

const SUMMARY_CARD_KEYS = [
  "totalSerializedAssets",
  "serializedInStock",
  "serializedAssigned",
  "totalQuantityOnHand",
  "totalQuantityAssigned",
] as const satisfies readonly (keyof AssetDashboardSummary)[]

export function buildAssetDashboardSummaryCards(
  summary: AssetDashboardSummary,
): AssetDashboardSummaryCard[] {
  return SUMMARY_CARD_KEYS.map((key) => ({
    key,
    label: SUMMARY_CARD_LABELS[key],
    value: summary[key],
  }))
}

export function getAssetDashboardDescription(): string {
  return "Review serialized assets and quantity stock in one place, then jump into the staged import wizard or manual asset entry when IT needs to add more inventory."
}

export function getAssetDashboardTabLabel(tab: AssetDashboardTabKey): string {
  return tab === "serialized" ? "Serialized" : "Quantity"
}

export function getAssetDashboardEmptyStateLabel(tab: AssetDashboardTabKey): string {
  return tab === "serialized"
    ? "No serialized assets are available yet."
    : "No quantity-tracked stock items are available yet."
}

export function formatAssetDashboardUsageLocationLabel(
  usageLocation: string | null,
): string {
  const normalized = normalizeAssetDashboardUsageLocation(usageLocation)
  if (normalized === "office") {
    return "T\u1ea1i CTY"
  }
  if (normalized === "home") {
    return "T\u1ea1i Nh\u00e0"
  }
  return "\u2014"
}

export function formatAssetDashboardDisplayNameLines(
  assetCode: string,
  displayNameShort: string | null,
  displayName: string,
): string {
  const normalizedDisplayName = displayName.trim()
  const shortName = resolveAssetDashboardDisplayNameShort(
    assetCode,
    displayNameShort,
    displayName,
  )?.trim()

  if (!shortName) {
    return normalizedDisplayName
  }

  if (shortName.toLowerCase() === normalizedDisplayName.toLowerCase()) {
    return shortName
  }

  return `${shortName}\n${normalizedDisplayName}`
}

export function formatAssetDashboardHolderLabel(
  fullName: string | null,
  employeeId: string | null,
): string {
  const normalizedFullName = fullName?.trim()
  const normalizedEmployeeId = employeeId?.trim()

  if (normalizedFullName && normalizedEmployeeId) {
    return `${normalizedFullName}\n${normalizedEmployeeId}`
  }
  if (normalizedFullName) {
    return normalizedFullName
  }
  if (normalizedEmployeeId) {
    return normalizedEmployeeId
  }
  return "\u2014"
}

export function formatAssetDashboardStatusLabel(status: string): string {
  return status.replaceAll("_", " ")
}

export function parseAssetDashboardQuantityDraft(value: string): number | null {
  const trimmed = value.trim()
  if (!/^\d+$/.test(trimmed)) {
    return null
  }

  const parsed = Number(trimmed)
  return Number.isSafeInteger(parsed) ? parsed : null
}
