import { beforeEach, describe, expect, it, vi } from "vitest"

// jsdom's Blob has no arrayBuffer()
const toBytes = (blob: Blob) =>
    new Promise<Uint8Array>((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer))
        reader.onerror = () => reject(reader.error)
        reader.readAsArrayBuffer(blob)
    })
const fakeBlob = { arrayBuffer: async () => new Uint8Array([120]).buffer } as unknown as Blob

const { save, writeExportFile } = vi.hoisted(() => ({ save: vi.fn(), writeExportFile: vi.fn() }))
vi.mock("@tauri-apps/plugin-dialog", () => ({ save }))
vi.mock("../services/staff-api", () => ({ staffApi: { writeExportFile } }))

import { buildCsv, buildXlsxBlob, escapeCsvCell, saveExport } from "./tableExport"

beforeEach(() => {
    save.mockReset()
    writeExportFile.mockReset()
})

describe("tableExport", () => {
    it("escapeCsvCell guards formulas but not phone numbers", () => {
        expect(escapeCsvCell("=1+1")).toBe("'=1+1")
        expect(escapeCsvCell("@SUM(A1)")).toBe("'@SUM(A1)")
        expect(escapeCsvCell("+84 912 345 678")).toBe("+84 912 345 678")
        expect(escapeCsvCell("-5")).toBe("-5")
        expect(escapeCsvCell("a,b")).toBe('"a,b"')
    })

    it("buildCsv writes BOM and CRLF", () => {
        const csv = buildCsv({ name: "S", headers: ["A", "B"], rows: [["x", 2], ["y", null]] })
        expect(csv).toBe("\uFEFFA,B\r\nx,2\r\ny,\r\n")
    })

    it("buildXlsxBlob creates a ZIP with typed cells", async () => {
        const blob = await buildXlsxBlob([{ name: "S", headers: ["A"], rows: [[3]] }])
        const bytes = await toBytes(blob)
        expect(Array.from(bytes.slice(0, 4))).toEqual([0x50, 0x4b, 0x03, 0x04])
    })

    it("saveExport returns null when dialog cancelled", async () => {
        save.mockResolvedValue(null)
        expect(await saveExport(fakeBlob, "f.csv", "csv")).toBeNull()
        expect(writeExportFile).not.toHaveBeenCalled()
    })

    it("saveExport writes the file and returns the path", async () => {
        save.mockResolvedValue("C:/f.csv")
        expect(await saveExport(fakeBlob, "f.csv", "csv")).toBe("C:/f.csv")
        expect(writeExportFile).toHaveBeenCalledWith("C:/f.csv", [120])
    })
})
