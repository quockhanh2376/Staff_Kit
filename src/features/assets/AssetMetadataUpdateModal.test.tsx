import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import { AssetMetadataUpdateModal } from "./AssetMetadataUpdateModal"
import { staffApi } from "../../services/staff-api"
import type { AssetMetadataPreviewReport } from "../../types/staff"

vi.mock("../../services/staff-api", () => ({
    staffApi: {
        previewAssetMetadataReimport: vi.fn(),
        applyAssetMetadataReimport: vi.fn(),
    },
}))

describe("AssetMetadataUpdateModal", () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    const mockPreview: AssetMetadataPreviewReport = {
        fileName: "AssetList.xlsx",
        sheetName: "Asset",
        summary: {
            totalRows: 219,
            monitorRows: 209,
            metadataChanged: 196,
            unchanged: 10,
            missingAsset: 3,
            outOfScope: 10,
            invalid: 0,
        },
        rows: [
            {
                rowNumber: 2,
                rawAssetTag: "VNMON501",
                canonicalAssetCode: "VNMON501",
                rawCategory: "Monitor",
                status: "metadataChanged",
                statusReason: "2 field(s) to be updated",
                assetId: 101,
                currentDisplayName: "Monitor 24",
                currentBrand: "DELL",
                currentModel: null,
                currentSerialNumber: null,
                proposedDisplayName: "Dell UltraSharp U2422H",
                proposedBrand: "Dell",
                proposedModel: "U2422H",
                proposedSerialNumber: null,
                diffs: [
                    {
                        field: "display_name",
                        before: "Monitor 24",
                        after: "Dell UltraSharp U2422H",
                    },
                    {
                        field: "model",
                        before: null,
                        after: "U2422H",
                    },
                ],
            },
            {
                rowNumber: 3,
                rawAssetTag: "VNLAP504",
                canonicalAssetCode: "VNLAP504",
                rawCategory: "Laptop",
                status: "outOfScope",
                statusReason: "Excluded from metadata update: scope is limited to Monitor assets only",
                assetId: null,
                currentDisplayName: null,
                currentBrand: null,
                currentModel: null,
                currentSerialNumber: null,
                proposedDisplayName: "Dell Latitude 5420",
                proposedBrand: "Dell",
                proposedModel: "5420",
                proposedSerialNumber: null,
                diffs: [],
            },
        ],
    }

    it("renders nothing when closed", () => {
        const { container } = render(
            <AssetMetadataUpdateModal open={false} onClose={vi.fn()} />
        )
        expect(container.firstChild).toBeNull()
    })

    it("renders title, file input and browse button when open", () => {
        render(<AssetMetadataUpdateModal open={true} onClose={vi.fn()} />)
        expect(screen.getByText("Update Existing Asset Metadata")).toBeInTheDocument()
        expect(screen.getByPlaceholderText("Pick AssetList.xlsx workbook...")).toBeInTheDocument()
        expect(screen.getByText("Browse")).toBeInTheDocument()
    })

    it("displays preview results with summary cards, badges and diffs", async () => {
        vi.mocked(staffApi.previewAssetMetadataReimport).mockResolvedValue(mockPreview)

        render(<AssetMetadataUpdateModal open={true} onClose={vi.fn()} />)

        // Find the input and set value
        const input = screen.getByPlaceholderText("Pick AssetList.xlsx workbook...")
        fireEvent.change(input, { target: { value: "00_ExSource/AssetList.xlsx" } })

        // Click Preview
        const previewBtn = screen.getByText("Preview Metadata Update")
        fireEvent.click(previewBtn)

        await waitFor(() => {
            expect(screen.getByText("Total Rows")).toBeInTheDocument()
            expect(screen.getByText("219")).toBeInTheDocument()
            expect(screen.getByText("Monitor Rows")).toBeInTheDocument()
            expect(screen.getByText("209")).toBeInTheDocument()
            expect(screen.getByText("To Update")).toBeInTheDocument()
            expect(screen.getByText("196")).toBeInTheDocument()
            expect(screen.getByText("VNMON501")).toBeInTheDocument()
            expect(screen.getByText("Dell UltraSharp U2422H")).toBeInTheDocument()
        })
    })

    it("calls applyAssetMetadataReimport when approve button is clicked", async () => {
        vi.mocked(staffApi.previewAssetMetadataReimport).mockResolvedValue(mockPreview)
        vi.mocked(staffApi.applyAssetMetadataReimport).mockResolvedValue({
            totalProcessed: 219,
            updatedCount: 196,
            unchangedCount: 10,
            skippedCount: 13,
            errors: [],
        })

        const onSuccess = vi.fn()
        render(<AssetMetadataUpdateModal open={true} onClose={vi.fn()} onSuccess={onSuccess} />)

        const input = screen.getByPlaceholderText("Pick AssetList.xlsx workbook...")
        fireEvent.change(input, { target: { value: "00_ExSource/AssetList.xlsx" } })

        const previewBtn = screen.getByText("Preview Metadata Update")
        fireEvent.click(previewBtn)

        await waitFor(() => {
            expect(screen.getByText("Approve & Update Metadata (196)")).toBeInTheDocument()
        })

        const approveBtn = screen.getByText("Approve & Update Metadata (196)")
        fireEvent.click(approveBtn)

        await waitFor(() => {
            expect(staffApi.applyAssetMetadataReimport).toHaveBeenCalledWith({
                filePath: "00_ExSource/AssetList.xlsx",
            })
            expect(screen.getByText("Metadata Update Complete!")).toBeInTheDocument()
            expect(onSuccess).toHaveBeenCalled()
        })
    })
})
