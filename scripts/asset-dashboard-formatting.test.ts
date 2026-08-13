import assert from "node:assert/strict"

import {
  buildDerivedComputerName,
  detectAssetImportWizardMapping,
  formatDerivedComputerNames,
  getRequiredAssetImportMappingKeys,
  hasRequiredAssetImportMapping,
  normalizeAssetDashboardUsageLocation,
  resolveAssetDashboardDisplayNameShort,
} from "../src/features/assets/assetImportModeConfig.ts"
import {
  buildAssetCategoryDraftFromDetail,
  buildAssetDashboardSummaryCards,
  formatAssetDashboardDisplayNameLines,
  formatAssetDashboardStatusLabel,
  formatAssetDashboardUsageLocationLabel,
  parseAssetDashboardQuantityDraft,
  validateAssetCategoryDraft,
} from "../src/features/assets/assetDashboardCopy.ts"
import { resolveSerializedAssetComputerName } from "../src/features/assets/serializedAssetGridConfig.ts"
import { shouldCloseAssetImportWizardAfterImport } from "../src/features/assets/assetImportCopy.ts"

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

const canonicalSerializedMapping = detectAssetImportWizardMapping(
  ["Asset Tag", "Asset Name", "Category", "Serial Number"],
  "serialized",
)
assert.equal(canonicalSerializedMapping.assetCode, "Asset Tag")
assert.deepEqual(getRequiredAssetImportMappingKeys("serialized"), [
  "assetCode",
  "category",
  "assetName",
])
assert.equal(hasRequiredAssetImportMapping(canonicalSerializedMapping, "serialized"), true)

const legacyInternalIdMapping = detectAssetImportWizardMapping(
  ["Asset ID", "Asset Name", "Category"],
  "serialized",
)
assert.equal(legacyInternalIdMapping.assetCode, undefined)
assert.equal(hasRequiredAssetImportMapping(legacyInternalIdMapping, "serialized"), false)

const legacyCompatibleMapping = detectAssetImportWizardMapping(
  ["Asset ID", "Asset Name", "Category", "Computer Name"],
  "serialized",
)
assert.equal(legacyCompatibleMapping.assetCode, undefined)
assert.equal(legacyCompatibleMapping.computerName, "Computer Name")

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
assert.equal(resolveSerializedAssetComputerName("VNLAP293", "ASWVNLAP293"), "ASWVNLAP293")
assert.equal(resolveSerializedAssetComputerName("VNMON709", null), "")

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

assert.equal(formatAssetDashboardUsageLocationLabel(" Táº¡i CTY "), "Tại CTY")
assert.equal(formatAssetDashboardUsageLocationLabel("táº¡i nhÃ "), "Tại Nhà")
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
  formatAssetDashboardDisplayNameLines("VNMON709", null, null as never),
  "Mon709",
)
assert.equal(
  formatAssetDashboardDisplayNameLines("VNLAP293", null, null as never),
  "VNLAP293",
)
assert.equal(
  formatAssetDashboardStatusLabel("awaiting_it_review"),
  "awaiting it review",
)
{
  const originalReplaceAll = String.prototype.replaceAll
  // Simulate older WebView runtimes that still lack String.prototype.replaceAll.
  Object.defineProperty(String.prototype, "replaceAll", {
    value: undefined,
    configurable: true,
    writable: true,
  })
  try {
    assert.equal(
      formatAssetDashboardStatusLabel("awaiting_it_review"),
      "awaiting it review",
    )
  } finally {
    Object.defineProperty(String.prototype, "replaceAll", {
      value: originalReplaceAll,
      configurable: true,
      writable: true,
    })
  }
}
assert.equal(parseAssetDashboardQuantityDraft(""), null)
assert.equal(parseAssetDashboardQuantityDraft(" 5 "), 5)
assert.equal(parseAssetDashboardQuantityDraft("-1"), null)
assert.equal(parseAssetDashboardQuantityDraft("4.5"), null)

const categoryDraft = buildAssetCategoryDraftFromDetail({
  id: 10,
  categoryCode: "dock",
  categoryName: "Dock Station",
  trackingMode: "serialized",
  prefixCode: "VNDOCK",
  qrRequired: false,
  isActive: true,
  assetCount: 0,
  stockItemCount: 0,
  prefixes: [
    {
      id: 1,
      prefixValue: "VNDOCK",
      isPrimary: true,
      isActive: true,
    },
  ],
})

assert.deepEqual(
  validateAssetCategoryDraft(categoryDraft, []),
  [],
)
assert.deepEqual(
  validateAssetCategoryDraft(
    {
      ...categoryDraft,
      prefixes: [
        { prefixValue: "VNDOCK", isPrimary: true },
        { prefixValue: "vnDock", isPrimary: false },
      ],
    },
    [],
  ),
  ["Prefix values must stay unique inside the same category."],
)
assert.deepEqual(
  validateAssetCategoryDraft(
    {
      ...categoryDraft,
      prefixes: [
        { prefixValue: "VNLAP", isPrimary: true },
      ],
    },
    [
      {
        id: 1,
        categoryCode: "laptop",
        categoryName: "Laptop",
        trackingMode: "serialized",
        prefixCode: "VNLAP",
        qrRequired: true,
        isActive: true,
        assetCount: 3,
        stockItemCount: 0,
        prefixes: [
          {
            id: 2,
            prefixValue: "VNLAP",
            isPrimary: true,
            isActive: true,
          },
        ],
      },
    ],
  ),
  ["Prefix VNLAP is already active in another category."],
)
assert.equal(shouldCloseAssetImportWizardAfterImport(3, 0), true)
assert.equal(shouldCloseAssetImportWizardAfterImport(0, 0), false)
assert.equal(shouldCloseAssetImportWizardAfterImport(2, 1), false)

console.log("asset-dashboard-formatting tests passed")
