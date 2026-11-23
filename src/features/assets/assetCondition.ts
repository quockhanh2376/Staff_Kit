import type { AssetCondition } from "../../types/staff"

export const CONDITION_LABELS: Record<AssetCondition, string> = {
  new: "New",
  spare: "Spare",
  broken: "Broken",
}

export const ASSET_CONDITIONS = Object.keys(CONDITION_LABELS) as AssetCondition[]
