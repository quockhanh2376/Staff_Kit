import type { EmployeeRecord } from "../../types/staff"
import type { TableEditDrafts } from "../../types/app"
import { formatDate } from "../../lib/utils"
import { readEmployeeCellText } from "./employeeTableRules"
import { buildCsv as buildSheetCsv, buildXlsxBlob, saveExport, type ExportSheet } from "../../lib/tableExport"

export type ExportColumn = { key: string; label: string }

export type ExportEmployeeOptions = {
    employees: EmployeeRecord[]
    columns: ExportColumn[]
    drafts: TableEditDrafts
}

function readExportValue(employee: EmployeeRecord, key: string, drafts: TableEditDrafts): string {
    const draftValue = drafts[employee.id]?.[key]
    if (draftValue !== undefined) return draftValue
    const value = readEmployeeCellText(employee, key)
    if (key === "rowNumber") return ""
    if (key === "aswStartDate" || key === "clientStartDate") return value ? formatDate(value) : ""
    return value
}

export function buildExportRows({ employees, columns, drafts }: ExportEmployeeOptions): string[][] {
    return employees.map((employee) =>
        columns.map((column) => readExportValue(employee, column.key, drafts)),
    )
}

function toSheet({ employees, columns, drafts }: ExportEmployeeOptions): ExportSheet {
    return {
        name: "Employees",
        headers: columns.map((column) => column.label),
        rows: buildExportRows({ employees, columns, drafts }),
    }
}

export function buildCsv(options: ExportEmployeeOptions): string {
    return buildSheetCsv(toSheet(options))
}

export function buildXlsxSheetData({ employees, columns, drafts }: ExportEmployeeOptions) {
    return [
        columns.map((column) => ({ value: column.label, fontWeight: "bold" as const })),
        ...buildExportRows({ employees, columns, drafts }).map((row) => row.map((value) => ({ value }))),
    ]
}

export async function downloadCsv(options: ExportEmployeeOptions, filename: string): Promise<string | null> {
    return saveExport(new Blob([buildCsv(options)], { type: "text/csv;charset=utf-8" }), filename, "csv")
}

export async function downloadXlsx(options: ExportEmployeeOptions, filename: string): Promise<string | null> {
    return saveExport(await createXlsxBlob(options), filename, "xlsx")
}

export async function createXlsxBlob(options: ExportEmployeeOptions): Promise<Blob> {
    return buildXlsxBlob([toSheet(options)])
}
