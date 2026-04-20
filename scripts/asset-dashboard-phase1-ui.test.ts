import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const assetDashboardSource = readFileSync("src/features/assets/AssetDashboard.tsx", "utf8")

assert.match(
  assetDashboardSource,
  /placeholder="Search computer, asset code, holder, model\.\.\."/,
)
assert.match(assetDashboardSource, /filterSerializedAssetRows\(/)
assert.match(assetDashboardSource, /if \(activeTab !== "serialized"\) \{/)
assert.match(assetDashboardSource, /const filterControls = activeTab === "serialized" \? \(/)
assert.match(assetDashboardSource, /\{\s*filterControls\s*\}/)
assert.match(assetDashboardSource, /rows:\s*filteredRows/)
assert.match(assetDashboardSource, /filteredRows=\{filteredSerializedRows\}/)
assert.match(assetDashboardSource, /No serialized assets match the current filters\./)
assert.match(assetDashboardSource, /Categories/)
assert.match(assetDashboardSource, /value=\{serializedCategoryFilter\}/)
assert.match(assetDashboardSource, /setSerializedCategoryFilter\(event\.target\.value\)/)
assert.match(assetDashboardSource, /serializedCategoryOptions\.map\(\(option\) => \(/)
assert.match(assetDashboardSource, /detail\.trackingMode !== "serialized"/)
assert.match(assetDashboardSource, /normalizeSerializedAssetFilterText\(detail\.categoryCode\)/)
assert.match(assetDashboardSource, /normalizeSerializedAssetFilterText\(row\.categoryCode\)/)
assert.match(assetDashboardSource, /aria-label="Clear serialized search and filters"/)
assert.match(assetDashboardSource, /setSerializedSearchTerm\(""\)/)
assert.match(
  assetDashboardSource,
  /setSerializedCategoryFilter\(ALL_SERIALIZED_ASSET_CATEGORY_FILTER\)/,
)
assert.match(assetDashboardSource, /aria-label="Search serialized assets"/)
assert.match(assetDashboardSource, /aria-label="Filter serialized assets by category"/)
assert.doesNotMatch(assetDashboardSource, /onClearFilters=\{clearSerializedFilters\}/)
assert.doesNotMatch(assetDashboardSource, /onSearchTermChange=\{setSerializedSearchTerm\}/)
assert.doesNotMatch(assetDashboardSource, /onCategoryFilterChange=\{setSerializedCategoryFilter\}/)

console.log("asset-dashboard-phase1-ui tests passed")
