import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import type { AssetImportBatchDetail, AssetImportRowRecord } from "../../types/staff"
import { AssetImportStagingDrawer } from "./AssetImportStagingDrawer"
import type { AssetImportStagingState } from "./useAssetImportStaging"

function stagedRow(id: number, over: Partial<AssetImportRowRecord> = {}): AssetImportRowRecord {
    return {
        id,
        batchId: 1,
        rowNumber: id + 1,
        rawValues: [],
        assetCode: `VNLAP${id}`,
        assetType: "Laptop",
        displayName: `ASWVNLAP${id}`,
        computerName: `ASWVNLAP${id}`,
        brand: "LENOVO",
        model: "21SX002QVA",
        serialNumber: `SN${id}`,
        usageLocation: "home",
        quantity: null,
        warehouse: null,
        notes: null,
        submittedStaffId: null,
        submittedFullName: null,
        submittedTeam: null,
        submittedPhoneNumber: null,
        resolvedEmployeeId: null,
        resolvedEmployeeRowId: null,
        resolvedFullName: null,
        resolvedTeamName: null,
        ownerMatchStatus: "not_applicable",
        ownerWarnings: [],
        validationErrors: [],
        status: "valid",
        isEdited: false,
        editedFields: [],
        importedAssetId: null,
        condition: "Spare",
        reviewNotes: [],
        ...over,
    }
}

function batchOf(rows: AssetImportRowRecord[]): AssetImportBatchDetail {
    return {
        id: 1,
        batchKey: "b1",
        importType: "serialized",
        sourceFileName: "Asset_November.xlsx",
        sourceFilePath: "E:/Asset_November.xlsx",
        sourceFileType: "xlsx",
        sheetName: "New",
        headerRow: 1,
        status: "pending_review",
        totalRows: rows.length,
        validRows: 0,
        errorRows: 0,
        importedRows: 0,
        skippedRows: 0,
        createdAt: "",
        updatedAt: "",
        headers: [],
        mapping: {} as AssetImportBatchDetail["mapping"],
        rows,
    }
}

function staging(rows: AssetImportRowRecord[], over: Partial<AssetImportStagingState> = {}): AssetImportStagingState {
    return {
        batch: batchOf(rows),
        pendingBatch: null,
        isOpen: true,
        isBusy: false,
        error: null,
        close: vi.fn(),
        checkPendingBatch: vi.fn(),
        loadStaging: vi.fn(),
        continueStaging: vi.fn(),
        updateRow: vi.fn().mockResolvedValue(undefined),
        setRowSkipped: vi.fn().mockResolvedValue(undefined),
        approve: vi.fn().mockResolvedValue({ importedCount: 1 }),
        discard: vi.fn().mockResolvedValue(true),
        ...over,
    } as AssetImportStagingState
}

const existingWithSerialUpdate = stagedRow(204, {
    status: "skipped",
    reviewNotes: ["Already in Staff Kit", "Serial will update: HK4FMG3 → HK4MFG3"],
})

describe("AssetImportStagingDrawer", () => {
    it("shows editable rows, the serial note, and counts new and serial updates on Approve", () => {
        render(<AssetImportStagingDrawer staging={staging([stagedRow(1), existingWithSerialUpdate])} setGlobalNotice={vi.fn()} />)

        expect(screen.getByLabelText("Serial for VNLAP1")).toHaveValue("SN1")
        expect(screen.getByLabelText("Condition for VNLAP1")).toHaveValue("spare")
        expect(screen.getByText("Serial will update: HK4FMG3 → HK4MFG3")).toBeInTheDocument()
        expect(screen.queryByText("Already in Staff Kit")).not.toBeInTheDocument()
        expect(screen.getByLabelText("Skip VNLAP204")).toBeDisabled()
        expect(screen.getByRole("button", { name: "Approve — 1 new · 1 serial updates" })).toBeEnabled()
    })

    it("saves an edited cell on blur", () => {
        const state = staging([stagedRow(1)])
        render(<AssetImportStagingDrawer staging={state} setGlobalNotice={vi.fn()} />)

        const serial = screen.getByLabelText("Serial for VNLAP1")
        fireEvent.change(serial, { target: { value: " PF5MBXRW " } })
        fireEvent.blur(serial)

        expect(state.updateRow).toHaveBeenCalledWith(1, "serialNumber", "PF5MBXRW")
    })

    it("disables Approve while an error row remains and shows its reason", () => {
        const error = stagedRow(2, {
            status: "error",
            validationErrors: ["Conflict: assetCode 'VNLAP2' and serialNumber 'S2' identify different existing assets"],
        })
        render(<AssetImportStagingDrawer staging={staging([stagedRow(1), error])} setGlobalNotice={vi.fn()} />)

        expect(screen.getByText(/identify different existing assets/)).toBeInTheDocument()
        expect(screen.getByText("Fix or skip 1 error row(s) to approve.")).toBeInTheDocument()
        expect(screen.getByRole("button", { name: /^Approve/ })).toBeDisabled()
    })

    it("approves and reports the result", async () => {
        const state = staging([stagedRow(1)])
        const setGlobalNotice = vi.fn()
        render(<AssetImportStagingDrawer staging={state} setGlobalNotice={setGlobalNotice} />)

        fireEvent.click(screen.getByRole("button", { name: /^Approve/ }))

        await waitFor(() => expect(setGlobalNotice).toHaveBeenCalledWith("Asset import approved: 1 new, 0 serial updates."))
        expect(state.approve).toHaveBeenCalled()
    })

    it("discards only after confirming", () => {
        const state = staging([stagedRow(1)])
        const confirm = vi.spyOn(window, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true)
        render(<AssetImportStagingDrawer staging={state} setGlobalNotice={vi.fn()} />)

        fireEvent.click(screen.getByRole("button", { name: "Discard" }))
        expect(state.discard).not.toHaveBeenCalled()
        fireEvent.click(screen.getByRole("button", { name: "Discard" }))
        expect(state.discard).toHaveBeenCalled()
        expect(confirm).toHaveBeenLastCalledWith("Discard this staged import? Nothing will be written.")
        confirm.mockRestore()
    })

    it("shows the auto-detected Type and saves a picked Type", () => {
        const updateRow = vi.fn()
        render(
            <AssetImportStagingDrawer
                staging={staging(
                    [stagedRow(1, { kind: "serialized" }), stagedRow(2, { assetType: "Mouse", kind: "quantity", serialNumber: null, quantity: "10" })],
                    { updateRow },
                )}
                setGlobalNotice={vi.fn()}
            />,
        )

        const laptopType = screen.getByLabelText("Type for VNLAP1") as HTMLSelectElement
        expect(laptopType.selectedOptions[0].textContent).toBe("Auto (Serial)")
        const mouseType = screen.getByLabelText("Type for VNLAP2") as HTMLSelectElement
        expect(mouseType.selectedOptions[0].textContent).toBe("Auto (Quantity)")

        fireEvent.change(laptopType, { target: { value: "quantity" } })
        expect(updateRow).toHaveBeenCalledWith(1, "kind", "quantity")
    })
})
