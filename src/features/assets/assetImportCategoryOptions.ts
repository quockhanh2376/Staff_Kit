import type {
  AssetCategoryRecord,
  AssetImportMode,
} from "../../types/staff"

export type AssetImportCategoryOption = {
  value: string
  label: string
}

export function resolveAssetImportCategoryValue(
  categories: readonly AssetCategoryRecord[],
  mode: AssetImportMode,
  currentValue: string,
): string {
  const trimmedValue = currentValue.trim()
  if (!trimmedValue) {
    return ""
  }

  const matchedCategory = getActiveModeCategories(categories, mode).find(
    (category) =>
      normalizeCategoryValue(category.categoryCode) === normalizeCategoryValue(trimmedValue) ||
      normalizeCategoryValue(category.categoryName) === normalizeCategoryValue(trimmedValue),
  )

  return matchedCategory?.categoryName ?? trimmedValue
}

export function getAssetImportCategoryOptions(
  categories: readonly AssetCategoryRecord[],
  mode: AssetImportMode,
  currentValue?: string,
): AssetImportCategoryOption[] {
  const activeModeCategories = getActiveModeCategories(categories, mode)
  const resolvedCurrentValue = currentValue
    ? resolveAssetImportCategoryValue(categories, mode, currentValue)
    : ""

  const options = activeModeCategories.map((category) => ({
    value: category.categoryName,
    label: category.categoryName,
  }))

  if (
    resolvedCurrentValue &&
    !options.some(
      (option) =>
        normalizeCategoryValue(option.value) === normalizeCategoryValue(resolvedCurrentValue),
    )
  ) {
    return [
      {
        value: resolvedCurrentValue,
        label: `${resolvedCurrentValue} (current value)`,
      },
      ...options,
    ]
  }

  return options
}

function getActiveModeCategories(
  categories: readonly AssetCategoryRecord[],
  mode: AssetImportMode,
): readonly AssetCategoryRecord[] {
  return categories.filter(
    (category) => category.isActive && category.trackingMode === mode,
  )
}

function normalizeCategoryValue(value: string): string {
  return value.trim().toLowerCase()
}
