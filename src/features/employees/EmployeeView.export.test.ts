import { readFileSync } from "node:fs"
import { resolve } from "node:path"
import { afterEach, describe, expect, it, vi } from "vitest"

const employeeView = readFileSync(resolve(process.cwd(), "src/features/employees/EmployeeView.tsx"), "utf8")
const tableEdit = readFileSync(resolve(process.cwd(), "src/features/employees/useTableEdit.ts"), "utf8")

describe("Employee List export integration contract", () => {
    afterEach(() => {
        vi.useRealTimers()
    })

    it("exposes the required selection and export controls", () => {
        expect(employeeView).toContain("Select Page")
        expect(employeeView).toContain("Select All ({emp.totalEmployees})")
        expect(employeeView).toContain("Unselect Page")
        expect(employeeView).toContain("Clear All ({edit.selectedMoveEmployeeIds.length})")
        expect(employeeView).toContain("Export ({edit.selectedMoveEmployeeIds.length})")
        expect(employeeView).toContain("Export as Excel (.xlsx)")
        expect(employeeView).toContain("await downloadXlsx(options")
        expect(employeeView).toContain("setGlobalError(getUserErrorMessage(error))")
    })

    it("keeps export controls admin-only and selection ID-based", () => {
        expect(employeeView).toContain("{isAdminAccount && (")
        expect(employeeView).toContain("edit.selectedMoveEmployeeIds.length")
        expect(tableEdit).toContain("const selectEmployeeIds")
        expect(tableEdit).toContain("const unselectCurrentPageEmployees")
    })

    it("places both dropdown panels above their triggers with collision-safe scrolling", () => {
        expect(employeeView).toContain("bottom-full right-0")
        expect(employeeView).toContain("max-h-[min(60vh,240px)]")
        expect(employeeView).toContain("isUnselectMenuOpen ? \"▲\" : \"▼\"")
        expect(employeeView).toContain("isExportMenuOpen ? \"▲\" : \"▼\"")
    })

    it("uses resettable three-second timers and cleans them up", () => {
        vi.useFakeTimers()
        const close = vi.fn()
        let timer = window.setTimeout(close, 3000)

        vi.advanceTimersByTime(2999)
        expect(close).not.toHaveBeenCalled()
        window.clearTimeout(timer)
        timer = window.setTimeout(close, 3000)
        vi.advanceTimersByTime(2999)
        expect(close).not.toHaveBeenCalled()
        vi.advanceTimersByTime(1)
        expect(close).toHaveBeenCalledTimes(1)
        window.clearTimeout(timer)
        expect(employeeView).toContain("clearMenuTimer(unselectTimerRef)")
        expect(employeeView).toContain("clearMenuTimer(exportTimerRef)")
        expect(employeeView).toContain("document.addEventListener(\"pointerdown\"")
        expect(employeeView).toContain("closeUnselectMenu()")
        expect(employeeView).toContain("closeExportMenu()")
    })

    it("clears selection only for search and filter criteria", () => {
        expect(employeeView).toContain("emp.searchTerm")
        expect(employeeView).toContain("emp.teamFilter")
        expect(employeeView).toContain("emp.startDateFilter")
        expect(employeeView).not.toContain("emp.currentPage, emp.rowsPerPage")
    })
})
