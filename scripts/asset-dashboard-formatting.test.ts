import assert from "node:assert/strict"

import {
  buildDerivedComputerName,
  detectAssetImportWizardMapping,
  formatDerivedComputerNames,
  normalizeAssetDashboardUsageLocation,
  resolveAssetDashboardDisplayNameShort,
} from "../src/features/assets/assetImportModeConfig.ts"

const serializedHeaders = [
  "Assetcode ",
  "Category",
  "Asset Name",
  "Serrial Number ",
  "Usuage Location ",
]

const quantityHeaders = [
  "Asset code",
  "Category",
  "Asset Name",
  "Quantity ",
]

const serializedMapping = detectAssetImportWizardMapping(serializedHeaders, "serialized")
assert.equal(serializedMapping.assetCode, "Assetcode ")
assert.equal(serializedMapping.serialNumber, "Serrial Number ")
assert.equal(serializedMapping.usageLocation, "Usuage Location ")

const quantityMapping = detectAssetImportWizardMapping(quantityHeaders, "quantity")
assert.equal(quantityMapping.assetCode, "Asset code")
assert.equal(quantityMapping.quantity, "Quantity ")

assert.equal(normalizeAssetDashboardUsageLocation(" Tại CTY "), "office")
assert.equal(normalizeAssetDashboardUsageLocation("tại nhà"), "home")
assert.equal(normalizeAssetDashboardUsageLocation(""), null)

assert.equal(
  resolveAssetDashboardDisplayNameShort("VNMON709", null, "Dell 24 Monitor"),
  "Mon709",
)
assert.equal(
  resolveAssetDashboardDisplayNameShort("VNLAP293", null, "Dell Latitude 5540"),
  "Dell Latitude 5540",
)
assert.equal(
  resolveAssetDashboardDisplayNameShort("VNMON709", "  Mon709  ", "Dell 24 Monitor"),
  "Mon709",
)

assert.equal(buildDerivedComputerName("VNLAP122"), "ASWVNLAP122")
assert.equal(buildDerivedComputerName(" vnmacpro003 "), "ASWVNMACPRO003")
assert.equal(
  formatDerivedComputerNames(["VNMACPRO010", "VNLAP293"]),
  "ASWVNMACPRO010,\nASWVNLAP293",
)

console.log("asset-dashboard-formatting tests passed")
