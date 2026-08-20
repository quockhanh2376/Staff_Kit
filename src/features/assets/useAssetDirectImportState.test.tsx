import { act, renderHook } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { useAssetDirectImportState } from "./useAssetDirectImportState"

const mocks = vi.hoisted(() => ({
    listAssetCategories: vi.fn(),
}))

vi.mock("../../services/staff-api", () => ({
    staffApi: {
        listAssetCategories: mocks.listAssetCategories,
    },
}))

describe("useAssetDirectImportState initialization", () => {
    beforeEach(() => {
        mocks.listAssetCategories.mockReset()
    })

    it("does not load categories or surface an error before an import/manual panel is opened", async () => {
        const setGlobalError = vi.fn()
        mocks.listAssetCategories.mockRejectedValue(new Error("category lookup failed"))

        const { result } = renderHook(() =>
            useAssetDirectImportState({
                dbReady: true,
                isAuthenticated: true,
                reloadToken: 0,
                setGlobalError,
                triggerReload: vi.fn(),
            }),
        )

        await act(async () => {
            await Promise.resolve()
        })

        expect(result.current.isWizardOpen).toBe(false)
        expect(mocks.listAssetCategories).not.toHaveBeenCalled()
        expect(setGlobalError).not.toHaveBeenCalled()

        act(() => {
            result.current.openImportWizard()
        })

        expect(result.current.isWizardOpen).toBe(true)
        expect(mocks.listAssetCategories).not.toHaveBeenCalled()
        expect(setGlobalError).not.toHaveBeenCalled()
    })

    it("loads categories when the manual asset panel is opened", async () => {
        const categories = [
            {
                id: 1,
                categoryCode: "laptop",
                categoryName: "Laptop",
                trackingMode: "serialized",
                qrRequired: true,
                prefixCodes: ["VNLAP"],
            },
        ]
        mocks.listAssetCategories.mockResolvedValue(categories)

        const { result } = renderHook(() =>
            useAssetDirectImportState({
                dbReady: true,
                isAuthenticated: true,
                reloadToken: 0,
                setGlobalError: vi.fn(),
                triggerReload: vi.fn(),
            }),
        )

        await act(async () => {
            result.current.openManualAssetPanel()
            await Promise.resolve()
        })

        expect(mocks.listAssetCategories).toHaveBeenCalledTimes(1)
        expect(result.current.isWizardOpen).toBe(true)
        expect(result.current.panelMode).toBe("manual")
        expect(result.current.assetCategories).toEqual(categories)
    })
})
