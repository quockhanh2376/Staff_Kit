import type {
  AssetCategoryDetailRecord,
  AssetCategoryPrefixInput,
  AssetDashboardSummary,
  AssetTrackingMode,
} from "../../types/staff"
import {
  normalizeAssetDashboardUsageLocation,
  resolveAssetDashboardDisplayNameShort,
} from "./assetImportModeConfig.ts"

export type AssetDashboardTabKey = "serialized" | "quantity" | "categories"
export type AssetCategoryDraft = {
  id: number | null
  categoryCode: string
  categoryName: string
  trackingMode: AssetTrackingMode
  qrRequired: boolean
  prefixes: AssetCategoryPrefixInput[]
}

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
  if (tab === "serialized") {
    return "Serialized"
  }
  if (tab === "quantity") {
    return "Quantity"
  }
  return "Categories"
}

export function getAssetDashboardEmptyStateLabel(tab: AssetDashboardTabKey): string {
  if (tab === "serialized") {
    return "No serialized assets are available yet."
  }
  if (tab === "quantity") {
    return "No quantity-tracked stock items are available yet."
  }
  return "No asset categories are available yet."
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

export function buildEmptyAssetCategoryDraft(): AssetCategoryDraft {
  return {
    id: null,
    categoryCode: "",
    categoryName: "",
    trackingMode: "serialized",
    qrRequired: false,
    prefixes: [
      {
        prefixValue: "",
        isPrimary: true,
      },
    ],
  }
}

export function buildAssetCategoryDraftFromDetail(
  detail: AssetCategoryDetailRecord,
): AssetCategoryDraft {
  return {
    id: detail.id,
    categoryCode: detail.categoryCode,
    categoryName: detail.categoryName,
    trackingMode: detail.trackingMode,
    qrRequired: detail.qrRequired,
    prefixes:
      detail.prefixes.length > 0
        ? detail.prefixes.map((prefix) => ({
            prefixValue: prefix.prefixValue,
            isPrimary: prefix.isPrimary,
          }))
        : [
            {
              prefixValue: "",
              isPrimary: true,
            },
          ],
  }
}

function normalizeDraftPrefixValue(value: string): string {
  return value.trim().toUpperCase()
}

export function validateAssetCategoryDraft(
  draft: AssetCategoryDraft,
  categoryDetails: AssetCategoryDetailRecord[],
): string[] {
  const errors: string[] = []
  const normalizedCode = draft.categoryCode.trim()
  const normalizedName = draft.categoryName.trim()
  const activePrefixes = draft.prefixes
    .map((prefix) => ({
      prefixValue: normalizeDraftPrefixValue(prefix.prefixValue),
      isPrimary: prefix.isPrimary,
    }))
    .filter((prefix) => prefix.prefixValue.length > 0)

  if (!normalizedCode) {
    errors.push("Category code is required.")
  }
  if (!normalizedName) {
    errors.push("Category name is required.")
  }

  if (draft.prefixes.some((prefix) => prefix.prefixValue.trim().length === 0)) {
    errors.push("Remove blank prefix rows or fill them before saving.")
  }

  const uniquePrefixes = new Set(activePrefixes.map((prefix) => prefix.prefixValue))
  if (uniquePrefixes.size !== activePrefixes.length) {
    errors.push("Prefix values must stay unique inside the same category.")
  }

  const primaryCount = activePrefixes.filter((prefix) => prefix.isPrimary).length
  if (activePrefixes.length > 0 && primaryCount !== 1) {
    errors.push("Select exactly one primary prefix.")
  }

  const conflictingPrefix = activePrefixes.find((draftPrefix) =>
    categoryDetails.some(
      (category) =>
        category.id !== draft.id &&
        category.isActive &&
        category.prefixes.some(
          (prefix) =>
            prefix.isActive &&
            prefix.prefixValue.toUpperCase() === draftPrefix.prefixValue,
        ),
    ),
  )

  if (conflictingPrefix) {
    errors.push(`Prefix ${conflictingPrefix.prefixValue} is already active in another category.`)
  }

  return errors
}
