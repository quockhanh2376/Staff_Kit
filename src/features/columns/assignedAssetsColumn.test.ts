import { describe, expect, it } from "vitest"
import { DEFAULT_VISIBLE_COLUMN_KEYS, DEFAULT_COLUMN_WIDTHS } from "../../lib/constants"
import {
    buildEmployeePayloadForSave,
    isEmployeeTableEditableColumn,
    readEmployeeCellText,
} from "../employees/employeeTableRules"
import type { EmployeeRecord } from "../../types/staff"

describe("Assigned Assets column defaults and rules", () => {
    it("includes assignedAssets in DEFAULT_VISIBLE_COLUMN_KEYS", () => {
        expect(DEFAULT_VISIBLE_COLUMN_KEYS).toContain("assignedAssets")
    })

    it("hides legacy dynamic asset fields by default for new profiles", () => {
        const legacyKeys = ["asset_name", "aset_name_2", "asset_name_2", "cateagory", "category", "location"]
        for (const key of legacyKeys) {
            expect(DEFAULT_VISIBLE_COLUMN_KEYS).not.toContain(key)
        }
    })

    it("defines default column width for assignedAssets", () => {
        expect(DEFAULT_COLUMN_WIDTHS.assignedAssets).toBeGreaterThan(0)
    })

    it("ensures assignedAssets is read-only in table edit mode", () => {
        expect(isEmployeeTableEditableColumn("assignedAssets")).toBe(false)
        expect(isEmployeeTableEditableColumn("assignedAssets", true)).toBe(false)
    })

    it("reads assignedAssets count from employee record", () => {
        const mockEmployee = {
            id: 1,
            employeeId: "EE1",
            fullName: "Test",
            assignedAssetsCount: 3,
            dynamicFields: {},
        } as unknown as EmployeeRecord

        expect(readEmployeeCellText(mockEmployee, "assignedAssets")).toBe("3")

        const mockEmployeeZero = {
            id: 2,
            employeeId: "EE2",
            fullName: "Test 2",
            dynamicFields: {},
        } as unknown as EmployeeRecord

        expect(readEmployeeCellText(mockEmployeeZero, "assignedAssets")).toBe("0")
    })

    it("ensures asset_name and asset_name_2 are strictly read-only in table edit mode", () => {
        expect(isEmployeeTableEditableColumn("asset_name")).toBe(false)
        expect(isEmployeeTableEditableColumn("asset_name_2")).toBe(false)
        expect(isEmployeeTableEditableColumn("aset_name_2")).toBe(false)
    })

    it("reads monitor asset names from dynamicFields", () => {
        const mockEmployee = {
            id: 1,
            employeeId: "EE1",
            fullName: "Test",
            dynamicFields: {
                asset_name: "MON614",
                asset_name_2: "MON615",
            },
        } as unknown as EmployeeRecord

        expect(readEmployeeCellText(mockEmployee, "asset_name")).toBe("MON614")
        expect(readEmployeeCellText(mockEmployee, "asset_name_2")).toBe("MON615")
    })

    it("excludes asset_name columns from save drafts to prevent dynamic value pollution", () => {
        const mockEmployee = {
            id: 1,
            employeeId: "EE1",
            fullName: "Test",
            staffGroup: "employee_list",
            dynamicFields: {
                asset_name: "MON614",
            },
        } as unknown as EmployeeRecord

        const payload = buildEmployeePayloadForSave(mockEmployee, {
            asset_name: "TRY_EDIT",
            asset_name_2: "TRY_EDIT_2",
            custom_note: "Allowed note",
        })

        expect(payload.dynamicFields).toEqual({
            asset_name: "MON614",
            custom_note: "Allowed note",
        })
    })
})
