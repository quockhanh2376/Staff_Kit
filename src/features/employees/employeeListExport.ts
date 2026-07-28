import writeExcelFile from "write-excel-file/universal"
import type { EmployeeRecord } from "../../types/staff"
import type { TableEditDrafts } from "../../types/app"
import { formatDate } from "../../lib/utils"
import { readEmployeeCellText } from "./employeeTableRules"
import { staffApi } from "../../services/staff-api"

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

export function escapeCsvCell(value: string): string {
    return /[",\r\n]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value
}

export function buildCsv({ employees, columns, drafts }: ExportEmployeeOptions): string {
    const rows = [
        columns.map((column) => column.label),
        ...buildExportRows({ employees, columns, drafts }),
    ]
    return `\uFEFF${rows.map((row) => row.map(escapeCsvCell).join(",")).join("\r\n")}\r\n`
}

export function buildXlsxSheetData({ employees, columns, drafts }: ExportEmployeeOptions) {
    return [
        columns.map((column) => ({ value: column.label, fontWeight: "bold" as const })),
        ...buildExportRows({ employees, columns, drafts }).map((row) => row.map((value) => ({ value }))),
    ]
}

async function saveBlob(blob: Blob, filename: string, extension: string): Promise<void> {
    const { save } = await import("@tauri-apps/plugin-dialog")
    const path = await save({
        defaultPath: filename,
        filters: [{ name: extension === "xlsx" ? "Excel Workbook" : "CSV", extensions: [extension] }],
    })
    if (!path) return
    const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()))
    await staffApi.writeExportFile(path, bytes)
}

export async function downloadCsv(options: ExportEmployeeOptions, filename: string): Promise<void> {
    await saveBlob(new Blob([buildCsv(options)], { type: "text/csv;charset=utf-8" }), filename, "csv")
}

export async function downloadXlsx(options: ExportEmployeeOptions, filename: string): Promise<void> {
    const blob = await createXlsxBlob(options)
    await saveBlob(blob, filename, "xlsx")
}

export async function createXlsxBlob(options: ExportEmployeeOptions): Promise<Blob> {
    return writeExcelFile(buildXlsxSheetData(options)).toBlob()
}
