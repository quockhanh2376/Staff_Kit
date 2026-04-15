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

console.log("asset-dashboard-phase1-ui tests passed")
