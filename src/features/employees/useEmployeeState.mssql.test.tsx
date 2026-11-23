import { act, renderHook, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { readFileSync } from "node:fs"
import { resolve } from "node:path"

const api = vi.hoisted(() => ({
    stageEmployeeListSync: vi.fn(),
    getOpenEmployeeSyncBatch: vi.fn(),
    updateEmployeeSyncRow: vi.fn(),
    refreshEmployeeSyncRow: vi.fn(),
    discardEmployeeSyncBatch: vi.fn(),
    applyEmployeeSyncBatch: vi.fn(),
    listTeams: vi.fn(),
    listEmployeeGroupCounts: vi.fn(),
    searchEmployees: vi.fn(),
}))

vi.mock("../../services/staff-api", () => ({ staffApi: api }))

import { getMssqlEmployeeUpdateErrorMessage, useEmployeeState } from "./useEmployeeState"

const baseOptions = {
    dbReady: false,
    isAuthenticated: false,
    isAdmin: true,
    reloadToken: 0,
    setGlobalError: vi.fn(),
}

const row = (id: number, action = "apply") => ({
    id,
    staffCode: `ASWVN${id}`,
    kind: "new",
    localEmployeeId: null,
    localGroup: null,
    source: [],
    local: null,
    edits: null,
    action,
    notes: [],
    state: "pending",
    assetsHeld: 0,
    isFloating: false,
})

const batch = {
    id: 7,
    createdAt: "2026-10-13T00:00:00Z",
    counts: { new: 1, changed: 0, unchanged: 0, missing: 0, conflicts: 0 },
    rows: [row(1), row(2)],
}

beforeEach(() => {
    vi.clearAllMocks()
    api.listTeams.mockResolvedValue([])
    api.listEmployeeGroupCounts.mockResolvedValue({ employeeList: 0, onboarding: 0, offboarding: 0, internalMovement: 0 })
    api.searchEmployees.mockResolvedValue({ items: [], total: 0 })
    api.getOpenEmployeeSyncBatch.mockResolvedValue(null)
    api.stageEmployeeListSync.mockResolvedValue(batch)
    api.applyEmployeeSyncBatch.mockResolvedValue({
        added: 1, updated: 0, movedToEmployeeList: 0, offboarded: 0, skipped: 0, completedAt: "2026-10-13T00:01:00Z",
    })
})

describe("staged EE List sync state", () => {
    it("stages a new batch when none is open", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        let outcome: unknown
        await act(async () => { outcome = await result.current.startEmployeeSync() })
        expect(outcome).toBe("staged")
        expect(api.stageEmployeeListSync).toHaveBeenCalledWith(false)
        expect(result.current.syncBatch?.id).toBe(7)
    })

    it("startEmployeeSync returns open-exists when a batch is open", async () => {
        api.getOpenEmployeeSyncBatch.mockResolvedValue(batch)
        const { result } = renderHook(() => useEmployeeState({ ...baseOptions, dbReady: true, isAuthenticated: true }))
        await waitFor(() => expect(result.current.syncBatch).not.toBeNull())
        let outcome: unknown
        await act(async () => { outcome = await result.current.startEmployeeSync() })
        expect(outcome).toBe("open-exists")
        expect(api.stageEmployeeListSync).not.toHaveBeenCalled()
    })

    it("does not load the open batch for non-admins", async () => {
        renderHook(() => useEmployeeState({ ...baseOptions, isAdmin: false, dbReady: true, isAuthenticated: true }))
        await waitFor(() => expect(api.listTeams).toHaveBeenCalled())
        expect(api.getOpenEmployeeSyncBatch).not.toHaveBeenCalled()
    })

    it("replaceEmployeeSync stages with replace=true", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.replaceEmployeeSync() })
        expect(api.stageEmployeeListSync).toHaveBeenCalledWith(true)
        expect(result.current.syncBatch?.id).toBe(7)
    })

    it("updateSyncRow replaces the row from the response", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.startEmployeeSync() })
        api.updateEmployeeSyncRow.mockResolvedValue(row(2, "skip"))
        await act(async () => { await result.current.updateSyncRow(2, { action: "skip" }) })
        expect(api.updateEmployeeSyncRow).toHaveBeenCalledWith(2, { action: "skip" })
        expect(result.current.syncBatch?.rows.map((r) => r.action)).toEqual(["apply", "skip"])
    })

    it("refreshSyncRow replaces the row from the response", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.startEmployeeSync() })
        api.refreshEmployeeSyncRow.mockResolvedValue({ ...row(1), notes: ["x"] })
        await act(async () => { await result.current.refreshSyncRow(1) })
        expect(result.current.syncBatch?.rows[0].notes).toEqual(["x"])
    })

    it("resumeEmployeeSync sets the open batch on success", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        api.getOpenEmployeeSyncBatch.mockResolvedValue(batch)
        await act(async () => { await result.current.resumeEmployeeSync() })
        expect(result.current.syncBatch?.id).toBe(7)
    })

    it("resumeEmployeeSync failure keeps the existing batch and sets syncError", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.startEmployeeSync() })
        api.getOpenEmployeeSyncBatch.mockRejectedValue(new Error("MSSQL network connection failed"))
        await act(async () => { await result.current.resumeEmployeeSync() })
        expect(result.current.syncBatch?.id).toBe(7)
        expect(result.current.syncError).toContain("Cannot connect")
    })

    it("discardEmployeeSync clears the batch", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.startEmployeeSync() })
        await act(async () => { await result.current.discardEmployeeSync() })
        expect(api.discardEmployeeSyncBatch).toHaveBeenCalledWith(7)
        expect(result.current.syncBatch).toBeNull()
    })

    it("applyEmployeeSync clears batch and refreshes employees", async () => {
        const { result } = renderHook(() => useEmployeeState({ ...baseOptions, dbReady: true, isAuthenticated: true }))
        await waitFor(() => expect(api.searchEmployees).toHaveBeenCalledTimes(1))
        await act(async () => { await result.current.startEmployeeSync() })
        let applied: unknown
        await act(async () => { applied = await result.current.applyEmployeeSync() })
        expect(api.applyEmployeeSyncBatch).toHaveBeenCalledWith(7)
        expect(applied).toMatchObject({ added: 1 })
        expect(result.current.syncBatch).toBeNull()
        await waitFor(() => expect(api.searchEmployees).toHaveBeenCalledTimes(2))
    })

    it("shows a safe error and stages nothing when the source query fails", async () => {
        const rawError = "MSSQL network connection failed: password=secret; host=sql.internal"
        api.stageEmployeeListSync.mockRejectedValue(new Error(rawError))
        const setGlobalError = vi.fn()
        const { result } = renderHook(() => useEmployeeState({ ...baseOptions, setGlobalError }))
        await act(async () => { await result.current.startEmployeeSync() })
        expect(result.current.syncBatch).toBeNull()
        expect(result.current.syncError).toContain("Cannot connect")
        expect(result.current.syncError).not.toContain(rawError)
        expect(setGlobalError).toHaveBeenCalledWith(result.current.syncError)
    })
})

describe("employee MSSQL error mapping", () => {
    it("distinguishes verified write access from a failed permission check", () => {
        const detected = "Read-only verification found effective write-level permissions for this SQL login; syncing is blocked."
        expect(getMssqlEmployeeUpdateErrorMessage(new Error(detected))).toBe(detected)
    })

    it("does not echo an unexpected credential-like value inside a diagnostic", () => {
        const message = getMssqlEmployeeUpdateErrorMessage(
            new Error("Read-only permission diagnostic: SQL Server error 916: password=secret"),
        )
        expect(message).not.toContain("secret")
        expect(message).toContain("must be read-only")
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
