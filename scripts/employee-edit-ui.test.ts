import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const employeeView = readFileSync("src/features/employees/EmployeeView.tsx", "utf8")

assert.match(employeeView, /onClick=\{\(\) => edit\.startTableCellEdit\(employee, column\.key\)\}/)
assert.match(employeeView, /onDoubleClick=\{\(\) => edit\.startTableCellEdit\(employee, column\.key\)\}/)
assert.match(employeeView, /Click an editable cell to update it\./)
assert.match(employeeView, /Computer Name is editable for Super Admin/)

console.log("employee-edit-ui tests passed")
