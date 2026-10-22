import { render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import { AssetImportWizard } from "./AssetImportWizard"
import type { AssetDirectImportPreview } from "../../types/staff"

describe("AssetImportWizard — Auto-Create Category Preview", () => {
    const mockPreview: AssetDirectImportPreview = {
        fileName: "AssetList.xlsx",
        sheetName: "Sheet1",
        importType: "serialized",
        totalRows: 4,
        validRows: 4,
        errorRows: 0,
        skippedRows: 0,
        newCategories: [
            {
                categoryName: "H Phone",
                assetCount: 4,
            },
        ],
        rows: [
            {
                rowNumber: 2,
                assetCode: "VNPHONE001",
                assetType: "H Phone",
                computerName: null,
                displayName: "Samsung Galaxy S24",
                model: "Galaxy S24",
                serialNumber: "SN001",
                adapterNumber: null,
                quantity: null,
                usageLocation: null,
                notes: null,
                status: "valid",
                holderLabel: null,
                validationErrors: [],
            },
        ],
        errors: [],
    }

    const mockAssetImportState = {
        isWizardOpen: true,
        panelMode: "direct",
        isInspectingFile: false,
        isPreviewingImport: false,
        isApprovingImport: false,
        canPreviewCurrentFile: true,
        previewApproveDisabled: false,
        selectedFiles: ["D:/AssetList.xlsx"],
        inspection: null,
        preview: mockPreview,
        report: null,
        statusMessage: null,
        openWizard: vi.fn(),
        closeWizard: vi.fn(),
        handleSelectFiles: vi.fn(),
        handlePreviewImport: vi.fn(),
        handleApproveImport: vi.fn(),
        handleCancelPreview: vi.fn(),
        handleReset: vi.fn(),
    }

    it("displays new category summary in preview stats", () => {
        render(
            <AssetImportWizard
                assetImport={mockAssetImportState as unknown as Parameters<typeof AssetImportWizard>[0]["assetImport"]}
            />
        )

        expect(screen.getByText("New Category: H Phone")).toBeInTheDocument()
        expect(screen.getByText("4 assets")).toBeInTheDocument()
    })
})
