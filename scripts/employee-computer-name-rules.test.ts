import assert from "node:assert/strict"

import {
  buildEmployeePayloadForSave,
  collectDuplicateComputerEmployeeIds,
  extractComputerNameTokens,
  isEmployeeTableEditableColumn,
} from "../src/features/employees/employeeTableRules.ts"
import type { EmployeeRecord } from "../src/types/staff.ts"

const employeeA: EmployeeRecord = {
  id: 1,
  employeeId: "ASWVN1302",
  fullName: "Dang Thai Van",
  nickName: null,
  teamId: 1,
  teamName: "Examworks",
  project: null,
  jobTitle: null,
  email: null,
  cellphone: null,
  dateOfBirth: null,
  gender: null,
  aswStartDate: null,
  clientStartDate: null,
  contractEndDate: null,
  clientYearOfServices: null,
  startDate: null,
  computerName: "ASWVNMACPRO010,\nASWVNLAP293",
  storedComputerName: "LEGACY-RAW-01",
  notes: null,
  staffGroup: "employee_list",
  dynamicFields: {
    computer_2: "ASWVNLAP400",
  },
  updatedAt: "2026-04-06 10:00:00",
}

const employeeB: EmployeeRecord = {
  ...employeeA,
  id: 2,
  employeeId: "ASWVN1303",
  fullName: "Second User",
  computerName: "ASWVNLAP293",
  storedComputerName: "LEGACY-RAW-02",
  dynamicFields: {},
}

assert.deepEqual(extractComputerNameTokens("ASWVNMACPRO010,\nASWVNLAP293"), [
  "ASWVNMACPRO010",
  "ASWVNLAP293",
])

assert.equal(isEmployeeTableEditableColumn("computerName"), false)
assert.equal(isEmployeeTableEditableColumn("notes"), true)

const payload = buildEmployeePayloadForSave(employeeA, {
  notes: "updated",
})
assert.equal(payload.computerName, "LEGACY-RAW-01")
assert.equal(payload.notes, "updated")

const duplicateIds = collectDuplicateComputerEmployeeIds(
  [employeeA, employeeB],
  ["computerName", "computer_2"],
)
assert.deepEqual([...duplicateIds].sort((a, b) => a - b), [1, 2])

console.log("employee-computer-name-rules tests passed")
