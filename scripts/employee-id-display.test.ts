import assert from "node:assert/strict"

import { formatEmployeeIdForDisplay } from "../src/features/employees/employeeIdDisplay.ts"

assert.equal(formatEmployeeIdForDisplay("ASWVN1253"), "1253")
assert.equal(formatEmployeeIdForDisplay("ASWVN0007"), "0007")
assert.equal(formatEmployeeIdForDisplay("1253"), "1253")
assert.equal(formatEmployeeIdForDisplay("ASWVN"), "ASWVN")
assert.equal(formatEmployeeIdForDisplay("abc123xyz"), "abc123xyz")
assert.equal(formatEmployeeIdForDisplay("  ASWVN9988  "), "9988")

console.log("employee-id-display tests passed")
