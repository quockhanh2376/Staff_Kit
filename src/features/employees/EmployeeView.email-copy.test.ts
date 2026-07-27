import { describe, expect, it } from "vitest"
import { readFileSync } from "node:fs"
import { resolve } from "node:path"

const employeeView = readFileSync(resolve(process.cwd(), "src/features/employees/EmployeeView.tsx"), "utf8")
const copyCell = readFileSync(resolve(process.cwd(), "src/features/employees/EmailCopyCell.tsx"), "utf8")

describe("EmployeeView email-copy integration contract", () => {
    it("renders semantic email cells only in normal mode and keeps edit precedence", () => {
        const editorIndex = employeeView.indexOf("if (isEditable && isActiveCell)")
        const emailIndex = employeeView.indexOf("column.dataType === \"email\"")
        expect(editorIndex).toBeGreaterThanOrEqual(0)
        expect(emailIndex).toBeGreaterThan(editorIndex)
        expect(employeeView).toContain("<EmployeeCellContent")
        expect(employeeView).toContain("isEditable={false}")
    })

    it("passes the raw value and highlighted display value to the shared cell renderer", () => {
        expect(employeeView).toContain("rawValue={rawCellValue}")
        expect(employeeView).toContain("displayValue={emailDisplayValue}")
        expect(employeeView).toContain("<HighlightText text={rawCellValue} query={emp.searchTerm} />")
    })

    it("keeps table click handlers while relying on the copy button to stop propagation", () => {
        expect(employeeView).toContain("onClick={() => edit.startTableCellEdit(employee, column.key)}")
        expect(copyCell).toContain("event.stopPropagation()")
    })
})
