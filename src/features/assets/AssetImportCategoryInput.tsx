import type { ChangeEvent } from "react"

import type {
  AssetCategoryRecord,
  AssetImportMode,
} from "../../types/staff"
import {
  getAssetImportCategoryOptions,
  resolveAssetImportCategoryValue,
} from "./assetImportCategoryOptions"

type AssetImportCategoryInputProps = {
  assetCategories: readonly AssetCategoryRecord[]
  className?: string
  disabled?: boolean
  mode: AssetImportMode
  onChange: (nextValue: string) => void
  placeholder?: string
  value: string
}

export function AssetImportCategoryInput({
  assetCategories,
  className = "form-input",
  disabled = false,
  mode,
  onChange,
  placeholder = "Select Category",
  value,
}: AssetImportCategoryInputProps) {
  const options = getAssetImportCategoryOptions(assetCategories, mode, value)
  const resolvedValue = resolveAssetImportCategoryValue(assetCategories, mode, value)

  if (options.length === 0) {
    return (
      <input
        className={className}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        disabled={disabled}
        placeholder={placeholder}
      />
    )
  }

  return (
    <select
      className={className}
      value={resolvedValue}
      onChange={(event: ChangeEvent<HTMLSelectElement>) => onChange(event.target.value)}
      disabled={disabled}
    >
      <option value="">{placeholder}</option>
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  )
}
