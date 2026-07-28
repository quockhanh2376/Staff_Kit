import { describe, expect, it } from "vitest"
import type { EmployeeRecord } from "../../types/staff"
import { buildCsv, buildExportRows, buildXlsxSheetData, createXlsxBlob } from "./employeeListExport"

const employee: EmployeeRecord = {
    id: 7,
    employeeId: "EE-7",
    fullName: "Nguyen, An",
    nickName: null,
    teamId: null,
    teamName: null,
    project: "Project A",
    jobTitle: null,
    email: "an@example.com",
    cellphone: null,
    dateOfBirth: null,
    gender: null,
    aswStartDate: null,
    clientStartDate: null,
    contractEndDate: null,
    clientYearOfServices: null,
    startDate: null,
    computerName: null,
    storedComputerName: null,
    notes: null,
    staffGroup: "employee_list",
    dynamicFields: { location: "Hanoi" },
    updatedAt: "2026-07-28T00:00:00Z",
}

describe("employee list export projection", () => {
    it("preserves visible order, excludes hidden columns, overlays drafts, and keeps empty cells empty", () => {
        const options = {
            employees: [employee],
            columns: [
                { key: "fullName", label: "Name" },
                { key: "location", label: "Location" },
                { key: "teamName", label: "Team" },
            ],
            drafts: { 7: { fullName: "Draft Name" } },
        }

        expect(buildExportRows(options)).toEqual([["Draft Name", "Hanoi", ""]])
    })

    it("quotes CSV values and emits UTF-8 CSV headers", () => {
        const csv = buildCsv({
            employees: [{ ...employee, fullName: "Nguyen, An" }],
            columns: [{ key: "fullName", label: "Employee, Name" }],
            drafts: {},
        })

        expect(csv).toBe('\uFEFF"Employee, Name"\r\n"Nguyen, An"\r\n')
    })

    it("builds XLSX sheet data instead of CSV content", () => {
        const sheetData = buildXlsxSheetData({
            employees: [employee],
            columns: [{ key: "fullName", label: "Name" }],
            drafts: {},
        })

        expect(sheetData).toEqual([
            [{ value: "Name", fontWeight: "bold" }],
            [{ value: "Nguyen, An" }],
        ])
    })

    it("creates a real XLSX ZIP payload", async () => {
        const blob = await createXlsxBlob({
            employees: [employee],
            columns: [{ key: "fullName", label: "Name" }],
            drafts: {},
        })
        const bytes = await new Promise<Uint8Array>((resolve, reject) => {
            const reader = new FileReader()
            reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer))
            reader.onerror = () => reject(reader.error)
            reader.readAsArrayBuffer(blob)
        })
        expect(Array.from(bytes.slice(0, 4))).toEqual([0x50, 0x4b, 0x03, 0x04])
    })
})
