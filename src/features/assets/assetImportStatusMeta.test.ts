import { describe, expect, it } from "vitest"
import {
    getAssetImportStatusMeta,
    getAssetImportSummaryLabel,
} from "./assetImportStatusMeta"

describe("Asset import status labels", () => {
    it("distinguishes new, existing/skipped, and conflict rows", () => {
        expect(getAssetImportStatusMeta("valid").label).toBe("New")
        expect(getAssetImportStatusMeta("skipped").label).toBe("Existing / Skipped")
        expect(getAssetImportStatusMeta("error").label).toBe("Error / Conflict")
    })

    it("uses import-specific summary labels", () => {
        expect(getAssetImportSummaryLabel("valid")).toBe("New Rows")
        expect(getAssetImportSummaryLabel("skipped")).toBe("Existing / Skipped")
        expect(getAssetImportSummaryLabel("errors")).toBe("Error Rows")
    })
})
