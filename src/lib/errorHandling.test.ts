import { afterEach, describe, expect, it, vi } from "vitest"
import { classifyError } from "./errorHandling"

describe("asset import error mapping", () => {
    afterEach(() => {
        vi.restoreAllMocks()
    })

    it("maps an unrecognized asset header error to an actionable message and preserves the raw log", () => {
        const rawError = new Error("failed to detect an asset import header row in the workbook")
        const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined)

        const result = classifyError(rawError)

        expect(result.rawMessage).toBe(rawError.message)
        expect(result.userMessage).toBe(
            "This file does not contain recognizable asset import columns. Choose an asset workbook containing Asset Tag, Asset Name and Category.",
        )
        expect(result.isBusiness).toBe(false)
        expect(consoleError).toHaveBeenCalledWith("[Staff Kit] Unexpected error:", rawError)
    })
})
