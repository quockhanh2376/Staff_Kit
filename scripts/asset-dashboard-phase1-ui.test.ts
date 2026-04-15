import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const assetDashboardSource = readFileSync("src/features/assets/AssetDashboard.tsx", "utf8")

assert.match(
  assetDashboardSource,
  /placeholder="Search computer, asset code, holder, model\.\.\."/,
)
assert.match(assetDashboardSource, /filterSerializedAssetRows\(/)
assert.match(assetDashboardSource, /rows:\s*filteredRows/)
assert.match(assetDashboardSource, /No serialized assets match the current filters\./)
assert.match(assetDashboardSource, /All Categories/)
assert.match(assetDashboardSource, /value=\{categoryFilter\}/)
assert.match(assetDashboardSource, /onCategoryFilterChange\(event\.target\.value\)/)
assert.match(assetDashboardSource, /categoryOptions\.map\(\(option\) => \(/)
assert.match(assetDashboardSource, /detail\.trackingMode !== "serialized"/)
assert.match(assetDashboardSource, /normalizeSerializedAssetFilterText\(detail\.categoryCode\)/)
assert.match(assetDashboardSource, /normalizeSerializedAssetFilterText\(row\.categoryCode\)/)

console.log("asset-dashboard-phase1-ui tests passed")
