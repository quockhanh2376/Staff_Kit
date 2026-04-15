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
assert.match(assetDashboardSource, />\s*Clear Filters\s*</)
assert.match(assetDashboardSource, /setSerializedSearchTerm\(""\)/)
assert.match(
  assetDashboardSource,
  /setSerializedCategoryFilter\(ALL_SERIALIZED_ASSET_CATEGORY_FILTER\)/,
)
assert.match(assetDashboardSource, /onClearFilters=\{clearSerializedFilters\}/)

console.log("asset-dashboard-phase1-ui tests passed")
