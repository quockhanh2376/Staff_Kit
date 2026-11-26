import { act, fireEvent, render, screen } from "@testing-library/react"
import { useEffect } from "react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import type { AssetDirectImportPreview, AssetDirectImportPreviewRow } from "../../types/staff"
import { AssetImportWizard } from "./AssetImportWizard"
import { useAssetDirectImportState, type AssetImportState } from "./useAssetDirectImportState"

const mocks = vi.hoisted(() => ({
    inspectAssetImportFile: vi.fn(),
    previewAssetImportFile: vi.fn(),
    importAssetImportFile: vi.fn(),
    listAssetCategories: vi.fn(),
    createAssetManually: vi.fn(),
}))

vi.mock("../../services/staff-api", () => ({
    staffApi: mocks,
}))

const inspection = {
    fileName: "assets.xlsx",
    fileType: "xlsx",
    selectedSheetName: "Assets",
    availableSheets: ["Assets"],
    headerRow: 1,
    headers: [],
    mapping: {},
    requiresManualMapping: false,
}

function previewRow(
    rowNumber: number,
    condition: AssetDirectImportPreviewRow["condition"],
    reviewNotes: string[] = [],
): AssetDirectImportPreviewRow {
    return {
        rowNumber,
        assetCode: `VNLAP00${rowNumber}`,
        assetType: "Laptop",
        computerName: null,
        displayName: `Laptop ${rowNumber}`,
        model: null,
        serialNumber: null,
        adapterNumber: null,
        quantity: null,
        usageLocation: null,
        notes: null,
        status: "valid",
        holderLabel: null,
        validationErrors: [],
        condition,
        reviewNotes,
    }
}

function preview(
    rows: AssetDirectImportPreviewRow[],
    over: Partial<AssetDirectImportPreview> = {},
): AssetDirectImportPreview {
    return {
        fileName: "assets.xlsx",
        sheetName: "Assets",
        importType: "serialized",
        totalRows: rows.length,
        validRows: rows.length,
        errorRows: 0,
        skippedRows: 0,
        conditionUpdateRows: 0,
        serialUpdateRows: 0,
        newCategories: [],
        rows,
        errors: [],
        ...over,
    }
}

const existingRow = (rowNumber: number, condition: AssetDirectImportPreviewRow["condition"]) => ({
    ...previewRow(rowNumber, condition),
    status: "skipped" as const,
})
const existingOnly = (rows: AssetDirectImportPreviewRow[], conditionUpdateRows: number) =>
    preview(rows, { validRows: 0, skippedRows: rows.length, conditionUpdateRows })

const current: { state: AssetImportState | null } = { state: null }
const state = () => current.state as AssetImportState

function Harness() {
    const hookState = useAssetDirectImportState({
        dbReady: true,
        isAuthenticated: true,
        reloadToken: 0,
        setGlobalError: vi.fn(),
        triggerReload: vi.fn(),
    })
    useEffect(() => {
        current.state = hookState
    })
    return <AssetImportWizard assetImport={hookState} />
}

async function renderPreview() {
    render(<Harness />)
    await act(async () =>
        state().prepareSelectedFile({
            filePath: "C:\\imports\\assets.xlsx",
            sheetName: "Assets",
            importType: "serialized",
        }),
    )
    await act(async () => {
        await state().handlePreviewImport()
    })
}

describe("AssetImportWizard condition review", () => {
    beforeEach(() => {
        mocks.inspectAssetImportFile.mockReset().mockResolvedValue(inspection)
        mocks.previewAssetImportFile.mockReset()
        mocks.importAssetImportFile.mockReset()
    })

    it("import preview highlights rows with review notes and shows the note text", async () => {
        mocks.previewAssetImportFile.mockResolvedValue(
            preview([
                previewRow(2, "spare", ["Unknown status \"Brokn\" — imported as Spare"]),
                previewRow(3, "new"),
            ]),
        )
        await renderPreview()

        const flagged = screen.getByTestId("asset-preview-row-2")
        expect(flagged).toHaveClass("search-highlight-cell")
        expect(flagged).toHaveTextContent("Unknown status \"Brokn\" — imported as Spare")
        expect(screen.getByTestId("asset-preview-row-3")).not.toHaveClass("search-highlight-cell")
    })

    it("changing a preview row condition re-previews and imports with the override", async () => {
        mocks.previewAssetImportFile
            .mockResolvedValueOnce(preview([previewRow(2, "spare"), previewRow(3, "new")]))
            .mockResolvedValueOnce(
                preview([previewRow(2, "broken", ["Condition will change: Spare → Broken"]), previewRow(3, "new")]),
            )
        mocks.importAssetImportFile.mockResolvedValue({
            fileName: "assets.xlsx",
            sheetName: "Assets",
            importType: "serialized",
            totalRows: 2,
            imported: 1,
            skipped: 1,
            failed: 0,
            importedAssetCodes: ["VNLAP003"],
            errors: [],
            conditionUpdatedRows: 1,
        })
        await renderPreview()

        await act(async () => {
            fireEvent.change(screen.getByRole("combobox", { name: "Condition row 2" }), {
                target: { value: "broken" },
            })
        })

        const overrides = [{ rowNumber: 2, condition: "broken" }]
        expect(mocks.previewAssetImportFile).toHaveBeenLastCalledWith(
            expect.objectContaining({ conditionOverrides: overrides }),
        )
        expect(screen.getByTestId("asset-preview-row-2")).toHaveTextContent("Condition will change: Spare → Broken")

        await act(async () => {
            fireEvent.click(screen.getByRole("button", { name: /approve/i }))
        })
        expect(mocks.importAssetImportFile).toHaveBeenCalledWith(
            expect.objectContaining({ conditionOverrides: overrides }),
        )
        expect(screen.getByText("Condition Updated: 1")).toBeInTheDocument()
    })

    it("existing-only file with no condition changes cannot be approved", async () => {
        mocks.previewAssetImportFile.mockResolvedValue(existingOnly([existingRow(2, "spare")], 0))
        await renderPreview()

        expect(screen.getByRole("button", { name: /approve/i })).toBeDisabled()
    })

    it("existing-only file with a condition change can be approved and imports the override", async () => {
        mocks.previewAssetImportFile
            .mockResolvedValueOnce(existingOnly([existingRow(2, "spare"), existingRow(3, "spare")], 0))
            .mockResolvedValueOnce(existingOnly([existingRow(2, "broken"), existingRow(3, "spare")], 1))
        mocks.importAssetImportFile.mockResolvedValue({
            fileName: "assets.xlsx",
            sheetName: "Assets",
            importType: "serialized",
            totalRows: 2,
            imported: 0,
            skipped: 2,
            failed: 0,
            importedAssetCodes: [],
            errors: [],
            conditionUpdatedRows: 1,
        })
        await renderPreview()
        expect(screen.getByRole("button", { name: /approve/i })).toBeDisabled()

        await act(async () => {
            fireEvent.change(screen.getByRole("combobox", { name: "Condition row 2" }), {
                target: { value: "broken" },
            })
        })
        expect(screen.getByText("Condition Changes").nextElementSibling).toHaveTextContent("1")
        const approve = screen.getByRole("button", { name: /approve/i })
        expect(approve).toBeEnabled()

        await act(async () => {
            fireEvent.click(approve)
        })
        expect(mocks.importAssetImportFile).toHaveBeenCalledWith(
            expect.objectContaining({ conditionOverrides: [{ rowNumber: 2, condition: "broken" }] }),
        )
    })

    it("manual create defaults to New and sends the picked condition", async () => {
        mocks.listAssetCategories.mockResolvedValue([])
        mocks.createAssetManually.mockResolvedValue({ assetCode: "VNLAP9" })
        render(<Harness />)
        await act(async () => state().openManualAssetPanel())

        const select = screen.getByLabelText("Condition")
        expect(select).toHaveValue("new")
        expect(screen.queryByRole("option", { name: "Broken" })).toBeNull()
        await act(async () => {
            state().handleManualAssetFieldChange("assetCode", "VNLAP9")
            state().handleManualAssetFieldChange("assetType", "Laptop")
            state().handleManualAssetFieldChange("displayName", "Dell")
        })
        fireEvent.change(select, { target: { value: "spare" } })
        await act(async () => {
            fireEvent.click(screen.getByRole("button", { name: "Create Serialized Asset" }))
        })
        expect(mocks.createAssetManually).toHaveBeenCalledWith(
            expect.objectContaining({ assetCode: "VNLAP9", condition: "spare" }),
        )
    })
})
