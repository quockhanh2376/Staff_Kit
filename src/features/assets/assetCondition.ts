import type { AssetCondition } from "../../types/staff"

/** The one in-stock rule: retired/disposed/lost rows are neither assigned nor in stock. */
export const isInStock = (row: { status: string }) => row.status === "in_stock"

export const CONDITION_LABELS: Record<AssetCondition, string> = {
  new: "New",
  spare: "Spare",
  broken: "Broken",
}

export const ASSET_CONDITIONS = Object.keys(CONDITION_LABELS) as AssetCondition[]
