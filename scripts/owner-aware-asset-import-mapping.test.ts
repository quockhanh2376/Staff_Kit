import assert from "node:assert/strict"

import {
  buildDerivedComputerName,
  formatDerivedComputerNames,
  getAssetImportOwnerFieldValue,
  rowHasOwnerSnapshot,
} from "../src/features/assets/assetImportModeConfig.ts"
import type { AssetImportRowRecord } from "../src/types/staff.ts"

const baseRow: AssetImportRowRecord = {
  id: 1,
  batchId: 2,
  rowNumber: 3,
  rawValues: [
    { header: "StaffID", value: "ASW1302" },
    { header: "Tên Nhân Viên", value: "Đặng Thái Văn" },
    { header: "Team", value: "Examworks" },
    { header: "Phone Number", value: "0900000000" },
  ],
  assetCode: "VNLAP293",
  assetType: "Laptop",
  displayName: "ASWVNLAP293",
  brand: "Dell",
  model: "Latitude 7440",
  serialNumber: null,
  quantity: null,
  warehouse: "HCM",
  notes: "Issued laptop",
  submittedStaffId: null,
  submittedFullName: null,
  submittedTeam: null,
  submittedPhoneNumber: null,
  resolvedEmployeeId: "ASWVN1302",
  resolvedEmployeeRowId: 44,
  resolvedFullName: "Đặng Thái Văn",
  resolvedTeamName: "Examworks",
  ownerMatchStatus: "warning",
  ownerWarnings: ["submitted team does not match employee master"],
  validationErrors: [],
  status: "valid",
  isEdited: false,
  editedFields: [],
  importedAssetId: null,
}

assert.equal(buildDerivedComputerName("VNLAP122"), "ASWVNLAP122")
assert.equal(buildDerivedComputerName("vnmacpro003"), "ASWVNMACPRO003")

assert.equal(
  formatDerivedComputerNames(["VNMACPRO010", "VNLAP293"]),
  "ASWVNMACPRO010,\nASWVNLAP293",
)

assert.equal(getAssetImportOwnerFieldValue(baseRow, "submittedStaffId"), "ASW1302")
assert.equal(getAssetImportOwnerFieldValue(baseRow, "submittedFullName"), "Đặng Thái Văn")
assert.equal(getAssetImportOwnerFieldValue(baseRow, "submittedTeam"), "Examworks")
assert.equal(getAssetImportOwnerFieldValue(baseRow, "submittedPhoneNumber"), "0900000000")
assert.equal(rowHasOwnerSnapshot(baseRow), true)

const editedRow: AssetImportRowRecord = {
  ...baseRow,
  submittedStaffId: "1302",
  submittedFullName: "Dang Thai Van",
}

assert.equal(getAssetImportOwnerFieldValue(editedRow, "submittedStaffId"), "1302")
assert.equal(getAssetImportOwnerFieldValue(editedRow, "submittedFullName"), "Dang Thai Van")
assert.equal(rowHasOwnerSnapshot(editedRow), true)

console.log("owner-aware-asset-import-mapping tests passed")
