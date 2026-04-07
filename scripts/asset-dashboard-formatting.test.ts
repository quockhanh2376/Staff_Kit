import assert from "node:assert/strict"

import {
  buildDerivedComputerName,
  detectAssetImportWizardMapping,
  formatDerivedComputerNames,
  normalizeAssetDashboardUsageLocation,
  resolveAssetDashboardDisplayNameShort,
} from "../src/features/assets/assetImportModeConfig.ts"
import {
  buildAssetDashboardSummaryCards,
  formatAssetDashboardDisplayNameLines,
  formatAssetDashboardStatusLabel,
  formatAssetDashboardUsageLocationLabel,
  parseAssetDashboardQuantityDraft,
} from "../src/features/assets/assetDashboardCopy.ts"

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

assert.equal(normalizeAssetDashboardUsageLocation(" Táº¡i CTY "), "office")
assert.equal(normalizeAssetDashboardUsageLocation("táº¡i nhÃ "), "home")
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

const summaryCards = buildAssetDashboardSummaryCards({
  totalSerializedAssets: 12,
  serializedInStock: 8,
  serializedAssigned: 4,
  totalQuantityOnHand: 55,
  totalQuantityAssigned: 9,
})
assert.deepEqual(
  summaryCards.map((card) => [card.label, card.value]),
  [
    ["Total Serialized", 12],
    ["Serialized In Stock", 8],
    ["Serialized Assigned", 4],
    ["Qty On Hand", 55],
    ["Qty Assigned", 9],
  ],
)

assert.equal(formatAssetDashboardUsageLocationLabel(" Tại CTY "), "Tại CTY")
assert.equal(formatAssetDashboardUsageLocationLabel("tại nhà"), "Tại Nhà")
assert.equal(formatAssetDashboardUsageLocationLabel(""), "—")

assert.equal(
  formatAssetDashboardDisplayNameLines("VNMON709", null, "Dell 24 Monitor"),
  "Mon709\nDell 24 Monitor",
)
assert.equal(
  formatAssetDashboardDisplayNameLines("VNLAP293", null, "Dell Latitude 5540"),
  "Dell Latitude 5540",
)
assert.equal(
  formatAssetDashboardStatusLabel("awaiting_it_review"),
  "awaiting it review",
)
assert.equal(parseAssetDashboardQuantityDraft(""), null)
assert.equal(parseAssetDashboardQuantityDraft(" 5 "), 5)
assert.equal(parseAssetDashboardQuantityDraft("-1"), null)
assert.equal(parseAssetDashboardQuantityDraft("4.5"), null)

console.log("asset-dashboard-formatting tests passed")
