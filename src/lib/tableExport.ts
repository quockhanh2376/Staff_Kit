import { staffApi } from "../services/staff-api"

export type ExportCell = string | number | null
/** boldRows: indexes into rows to set bold in Excel (sub-headers); ignored by CSV. */
export type ExportSheet = { name: string; headers: string[]; rows: ExportCell[][]; boldRows?: number[] }

// Phone numbers and plain numbers (+84 912..., -5) are not formulas.
const PLAIN_NUMBER = /^[+-]?[0-9][0-9 ().-]*$/

export function escapeCsvCell(value: string): string {
    const guarded = /^[=+\-@\t\r]/.test(value) && !PLAIN_NUMBER.test(value) ? `'${value}` : value
    return /[",\r\n]/.test(guarded) ? `"${guarded.replace(/"/g, '""')}"` : guarded
}

export function buildCsv(sheet: ExportSheet): string {
    const lines = [sheet.headers, ...sheet.rows].map((row) =>
        row.map((cell) => escapeCsvCell(cell === null ? "" : String(cell))).join(","),
    )
    return `\uFEFF${lines.join("\r\n")}\r\n`
}

export function xlsxSheetData(sheet: ExportSheet) {
    const bold = { fontWeight: "bold" as const }
    return [
        sheet.headers.map((value) => ({ value, ...bold })),
        ...sheet.rows.map((row, index) =>
            row.map((value) => (value === null ? null : { value, ...(sheet.boldRows?.includes(index) ? bold : {}) })),
        ),
    ]
}

export async function buildXlsxBlob(sheets: ExportSheet[]): Promise<Blob> {
    const { default: writeExcelFile } = await import("write-excel-file/universal")
    return writeExcelFile(sheets.map((sheet) => ({ sheet: sheet.name, data: xlsxSheetData(sheet) }))).toBlob()
}

export async function saveExport(blob: Blob, filename: string, ext: "csv" | "xlsx"): Promise<string | null> {
    const { save } = await import("@tauri-apps/plugin-dialog")
    const path = await save({
        defaultPath: filename,
        filters: [{ name: ext === "xlsx" ? "Excel Workbook" : "CSV", extensions: [ext] }],
    })
    if (!path) return null
    await staffApi.writeExportFile(path, Array.from(new Uint8Array(await blob.arrayBuffer())))
    return path
}
