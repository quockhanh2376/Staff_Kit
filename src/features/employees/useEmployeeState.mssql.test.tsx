import { act, renderHook } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { readFileSync } from "node:fs"
import { resolve } from "node:path"

const api = vi.hoisted(() => ({
    previewEmployeeListMssqlSync: vi.fn(),
    applyEmployeeListMssqlSync: vi.fn(),
    listTeams: vi.fn(),
    listEmployeeGroupCounts: vi.fn(),
    searchEmployees: vi.fn(),
}))

vi.mock("../../services/staff-api", () => ({ staffApi: api }))

import { getMssqlEmployeeUpdateErrorMessage, useEmployeeState } from "./useEmployeeState"

const baseOptions = {
    dbReady: false,
    isAuthenticated: false,
    reloadToken: 0,
    setGlobalError: vi.fn(),
}

beforeEach(() => {
    vi.clearAllMocks()
    api.previewEmployeeListMssqlSync.mockResolvedValue({
        previewId: "preview-1",
        createdAt: "2026-10-13T00:00:00Z",
        counts: { new: 1, changed: 2, unchanged: 3, missing: 4, conflicts: 0 },
        rows: [],
        conflicts: [],
    })
    api.applyEmployeeListMssqlSync.mockResolvedValue({
        counts: { new: 1, changed: 2, unchanged: 3, missing: 4, conflicts: 0 },
        completedAt: "2026-10-13T00:01:00Z",
    })
})

describe("employee MSSQL update flow", () => {
    it("requests a fixed-query preview and does not write before approval", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))

        await act(async () => {
            await result.current.updateEmployeeListFromMssql()
        })

        expect(api.previewEmployeeListMssqlSync).toHaveBeenCalledWith()
        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()
        expect(result.current.mssqlSyncPreview?.counts.missing).toBe(4)
        expect(result.current.mssqlUpdateMessage).toContain("Preview ready")
    })

    it("does not create a preview when the source query fails", async () => {
        const rawError = "MSSQL network connection failed: password=secret; host=sql.internal"
        api.previewEmployeeListMssqlSync.mockRejectedValue(new Error(rawError))
        const setGlobalError = vi.fn()
        const { result } = renderHook(() =>
            useEmployeeState({ ...baseOptions, setGlobalError }),
        )

        await act(async () => {
            await result.current.updateEmployeeListFromMssql()
        })

        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()
        expect(result.current.mssqlSyncPreview).toBeNull()
        expect(result.current.mssqlUpdateStatus).toBe("error")
        expect(result.current.mssqlUpdateMessage).toContain("Cannot connect")
        expect(result.current.mssqlUpdateMessage).not.toContain(rawError)
        expect(setGlobalError).toHaveBeenCalledWith(result.current.mssqlUpdateMessage)
    })

    it("applies only after the explicit approve action", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.updateEmployeeListFromMssql() })
        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()

        await act(async () => { await result.current.applyEmployeeListMssqlSync() })

        expect(api.applyEmployeeListMssqlSync).toHaveBeenCalledWith("preview-1")
        expect(result.current.mssqlSyncPreview).toBeNull()
        expect(result.current.mssqlUpdateStatus).toBe("success")
        expect(result.current.mssqlUpdateMessage).toContain("imported 1")
        expect(result.current.mssqlUpdateMessage).toContain("missing 4")
    })

    it("dismisses the preview without applying local changes", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.updateEmployeeListFromMssql() })
        act(() => result.current.cancelEmployeeListMssqlSync())
        expect(result.current.mssqlSyncPreview).toBeNull()
        expect(result.current.mssqlUpdateMessage).toContain("No local data was changed")
        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()
    })

    it.each([
        ["MSSQL authentication failed: password=secret", "authentication failed"],
        ["MSSQL query failed: SELECT * FROM employee_records", "query failed"],
        ["MSSQL source login has write-level permissions", "must be read-only"],
        ["unexpected backend detail with token=secret", "update failed"],
    ])("maps %s to safe actionable feedback", (rawError, expectedText) => {
        const message = getMssqlEmployeeUpdateErrorMessage(new Error(rawError))

        expect(message.toLowerCase()).toContain(expectedText)
        expect(message).not.toContain(rawError)
        expect(message).not.toContain("secret")
        expect(message).not.toContain("employee_records")
        expect(message).not.toContain("sql.internal")
    })
})

describe("employee MSSQL feedback rendering contract", () => {
    it("does not use the old fixed-width truncation", () => {
        const source = readFileSync(resolve(process.cwd(), "src/features/employees/EmployeeView.tsx"), "utf8")

        expect(source).not.toContain("max-w-[360px] truncate")
        expect(source).toContain("max-w-[520px] break-words text-xs leading-snug")
    })
})
