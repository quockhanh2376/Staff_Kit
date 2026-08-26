import { act, renderHook } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { AssetDirectImportPreview, AssetDirectImportPreviewRow } from "../../types/staff"
import { useAssetDirectImportState } from "./useAssetDirectImportState"

const mocks = vi.hoisted(() => ({
    inspectAssetImportFile: vi.fn(),
    previewAssetImportFile: vi.fn(),
}))

vi.mock("../../services/staff-api", () => ({
    staffApi: {
        inspectAssetImportFile: mocks.inspectAssetImportFile,
        previewAssetImportFile: mocks.previewAssetImportFile,
    },
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

function previewForStatuses(statuses: string[]): AssetDirectImportPreview {
    const rows: AssetDirectImportPreviewRow[] = statuses.map((status, index) => ({
        rowNumber: index + 2,
        assetCode: `ASSET-${index + 1}`,
        assetType: "Laptop",
        computerName: null,
        displayName: `Asset ${index + 1}`,
        model: null,
        serialNumber: null,
        adapterNumber: null,
        quantity: null,
        usageLocation: null,
        notes: null,
        status,
        holderLabel: null,
        validationErrors: status === "error" ? ["Conflict"] : [],
    }))

    return {
        fileName: "assets.xlsx",
        sheetName: "Assets",
        importType: "serialized",
        totalRows: rows.length,
        validRows: rows.filter((row) => row.status === "valid").length,
        errorRows: rows.filter((row) => row.status === "error").length,
        skippedRows: rows.filter((row) => row.status === "skipped").length,
        rows,
        errors: [],
    }
}

async function previewAssetRows(statuses: string[]) {
    mocks.inspectAssetImportFile.mockResolvedValue(inspection)
    mocks.previewAssetImportFile.mockResolvedValue(previewForStatuses(statuses))

    const hook = renderHook(() =>
        useAssetDirectImportState({
            dbReady: true,
            isAuthenticated: true,
            reloadToken: 0,
            setGlobalError: vi.fn(),
            triggerReload: vi.fn(),
        }),
    )

    await act(async () =>
        hook.result.current.prepareSelectedFile({
            filePath: "C:\\imports\\assets.xlsx",
            sheetName: "Assets",
            importType: "serialized",
        }),
    )
    await act(async () => {
        await hook.result.current.handlePreviewImport()
    })

    return hook
}

describe("useAssetDirectImportState approval gating", () => {
    beforeEach(() => {
        mocks.inspectAssetImportFile.mockReset()
        mocks.previewAssetImportFile.mockReset()
    })

    it.each([
        ["enables approval for six existing and four new rows", ["skipped", "skipped", "skipped", "skipped", "skipped", "skipped", "valid", "valid", "valid", "valid"], false],
        ["disables approval for ten existing rows", Array(10).fill("skipped"), true],
        ["disables approval when a conflict remains beside new rows", ["skipped", "skipped", "skipped", "skipped", "skipped", "valid", "valid", "valid", "valid", "error"], true],
        ["disables approval when malformed or conflict rows remain beside new rows", ["valid", "valid", "valid", "valid", "error", "error", "error", "error", "error", "error"], true],
    ])("%s", async (_name, statuses, expectedDisabled) => {
        const hook = await previewAssetRows(statuses as string[])

        expect(hook.result.current.previewApproveDisabled).toBe(expectedDisabled)
    })
})
