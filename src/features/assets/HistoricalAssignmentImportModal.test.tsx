import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import { HistoricalAssignmentImportModal } from "./HistoricalAssignmentImportModal"
import { staffApi } from "../../services/staff-api"
import type { HistoricalAssignmentPreview } from "../../types/staff"

vi.mock("../../services/staff-api", () => ({
    staffApi: {
        previewHistoricalAssignmentImport: vi.fn(),
        applyHistoricalAssignmentImport: vi.fn(),
    },
}))

describe("HistoricalAssignmentImportModal", () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })
    const mockPreview: HistoricalAssignmentPreview = {
        fileName: "Asigned_Asset.xlsx",
        summary: {
            totalRows: 157,
            totalAssignments: 200,
            newAssets: 193,
            newAssignments: 0,
            existingSkipped: 0,
            excludedConflicts: 5,
            errors: 2,
            canApproveRemainder: true,
        },
        items: [
            {
                rowNumber: 2,
                rawStaffId: "ASWVN30",
                rawEmployeeName: "Unknown Staff",
                rawAssetCode: "MON540",
                location: "Tại công ty",
                status: "missingEmployee",
                statusReason: "Employee Staff ID 'ASWVN30' not found in database",
                employeeId: null,
                canonicalStaffId: null,
                canonicalEmployeeName: null,
                assetId: null,
                canonicalAssetCode: "VNMON540",
                currentHolderStaffId: null,
                currentHolderName: null,
            },
            {
                rowNumber: 7,
                rawStaffId: "ASWVN214",
                rawEmployeeName: "LÊ MINH DUY",
                rawAssetCode: "MON770",
                location: "Tại công ty",
                status: "conflict",
                statusReason: "Staff ID 'ASWVN214' belongs to 'Trần Nhật Phong' in database",
                employeeId: 10,
                canonicalStaffId: "ASWVN214",
                canonicalEmployeeName: "Trần Nhật Phong",
                assetId: null,
                canonicalAssetCode: "VNMON770",
                currentHolderStaffId: null,
                currentHolderName: null,
            },
            {
                rowNumber: 8,
                rawStaffId: "ASWVN1229",
                rawEmployeeName: "TRẦN TẤN QUÝ",
                rawAssetCode: "MON690",
                location: "Tại công ty",
                status: "newAsset",
                statusReason: "Asset 'VNMON690' will be auto-created as Monitor on approval",
                employeeId: 12,
                canonicalStaffId: "ASWVN1229",
                canonicalEmployeeName: "TRẦN TẤN QUÝ",
                assetId: null,
                canonicalAssetCode: "VNMON690",
                currentHolderStaffId: null,
                currentHolderName: null,
            },
        ],
    }

    it("renders file selection when open without preview", () => {
        render(<HistoricalAssignmentImportModal open={true} onClose={vi.fn()} />)
        expect(screen.getByText("Select Historical Monitor Workbook")).toBeInTheDocument()
        expect(screen.getByText("Preview Assignments")).toBeInTheDocument()
    })

    it("displays summary cards and quarantine alert on preview", async () => {
        vi.mocked(staffApi.previewHistoricalAssignmentImport).mockResolvedValue(mockPreview)

        render(<HistoricalAssignmentImportModal open={true} onClose={vi.fn()} />)

        const input = screen.getByPlaceholderText(/Choose \.xlsx file/)
        fireEvent.change(input, { target: { value: "00_ExSource/Asigned_Asset.xlsx" } })
        fireEvent.click(screen.getByText("Preview Assignments"))

        await waitFor(() => {
            expect(screen.getByText("New Assets")).toBeInTheDocument()
        })

        // Check summary counts
        expect(screen.getByText("193")).toBeInTheDocument() // newAssets
        expect(screen.getByText("5")).toBeInTheDocument() // excludedConflicts
        expect(screen.getAllByText("2").length).toBeGreaterThanOrEqual(1) // errors and row 2

        // Check quarantine warning
        expect(
            screen.getByText(/Quarantine Active: 5 conflicting assignment\(s\) excluded/)
        ).toBeInTheDocument()

        // Check table rows
        expect(screen.getByText("ASWVN30")).toBeInTheDocument()
        expect(screen.getByText("ASWVN214")).toBeInTheDocument()
        expect(screen.getByText("ASWVN1229")).toBeInTheDocument()
        expect(screen.getByText("Quarantined Conflict")).toBeInTheDocument()
        expect(screen.getByText("Missing Employee")).toBeInTheDocument()
        expect(screen.getByText("New Asset & Assign")).toBeInTheDocument()

        // Check approve button label
        expect(screen.getByText("Approve Valid Remainder (193)")).toBeInTheDocument()
    })

    it("approves valid remainder and renders success report", async () => {
        vi.mocked(staffApi.previewHistoricalAssignmentImport).mockResolvedValue(mockPreview)
        vi.mocked(staffApi.applyHistoricalAssignmentImport).mockResolvedValue({
            totalProcessed: 200,
            createdAssets: 193,
            createdAssignments: 193,
            skippedAlreadyAssigned: 0,
            excludedConflicts: 5,
            errors: [
                "Excluded row 2: Missing employee ASWVN30",
                "Excluded row 152: Missing employee ASWN1323",
            ],
        })

        const onSuccess = vi.fn()
        render(
            <HistoricalAssignmentImportModal
                open={true}
                onClose={vi.fn()}
                onSuccess={onSuccess}
            />
        )

        const input = screen.getByPlaceholderText(/Choose \.xlsx file/)
        fireEvent.change(input, { target: { value: "00_ExSource/Asigned_Asset.xlsx" } })
        fireEvent.click(screen.getByText("Preview Assignments"))

        await waitFor(() => {
            expect(screen.getByText("Approve Valid Remainder (193)")).toBeInTheDocument()
        })

        fireEvent.click(screen.getByText("Approve Valid Remainder (193)"))

        await waitFor(() => {
            expect(screen.getByText("Import Completed Successfully")).toBeInTheDocument()
        })

        expect(screen.getByText("Assets Created")).toBeInTheDocument()
        expect(screen.getByText("Assignments Created")).toBeInTheDocument()
        expect(onSuccess).toHaveBeenCalled()
    })

    it("cancels preview with zero writes", async () => {
        vi.mocked(staffApi.previewHistoricalAssignmentImport).mockResolvedValue(mockPreview)

        render(<HistoricalAssignmentImportModal open={true} onClose={vi.fn()} />)

        const input = screen.getByPlaceholderText(/Choose \.xlsx file/)
        fireEvent.change(input, { target: { value: "00_ExSource/Asigned_Asset.xlsx" } })
        fireEvent.click(screen.getByText("Preview Assignments"))

        await waitFor(() => {
            expect(screen.getByText("Cancel (Zero Writes)")).toBeInTheDocument()
        })

        fireEvent.click(screen.getByText("Cancel (Zero Writes)"))

        // Resets back to file selection
        expect(screen.getByText("Select Historical Monitor Workbook")).toBeInTheDocument()
        expect(staffApi.applyHistoricalAssignmentImport).not.toHaveBeenCalled()
    })
})
