import { act, renderHook, waitFor } from "@testing-library/react"
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
    api.listTeams.mockResolvedValue([])
    api.listEmployeeGroupCounts.mockResolvedValue({ employeeList: 0, onboarding: 0, offboarding: 0, internalMovement: 0 })
    api.searchEmployees.mockResolvedValue({ items: [], total: 0 })
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

    it("shows the backend's bounded permission diagnostic when Update cannot create a preview", async () => {
        const diagnostic = "Read-only permission diagnostic: SQL Server error 916: metadata access denied."
        api.previewEmployeeListMssqlSync.mockRejectedValue(new Error(diagnostic))
        const { result } = renderHook(() => useEmployeeState(baseOptions))

        await act(async () => { await result.current.updateEmployeeListFromMssql() })

        expect(result.current.mssqlUpdateMessage).toBe(diagnostic)
        expect(result.current.mssqlSyncPreview).toBeNull()
        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()
    })

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

    it("refreshes the displayed EE List after approved local apply", async () => {
        const { result } = renderHook(() => useEmployeeState({ ...baseOptions, dbReady: true, isAuthenticated: true }))
        await waitFor(() => expect(api.searchEmployees).toHaveBeenCalledTimes(1))

        await act(async () => { await result.current.updateEmployeeListFromMssql() })
        expect(api.searchEmployees).toHaveBeenCalledTimes(1)
        await act(async () => { await result.current.applyEmployeeListMssqlSync() })

        await waitFor(() => expect(api.searchEmployees).toHaveBeenCalledTimes(2))
        expect(api.searchEmployees).toHaveBeenLastCalledWith(expect.objectContaining({ staffGroup: "employee_list", offset: 0 }))
    })

    it("dismisses the preview with Skip without applying local changes", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.updateEmployeeListFromMssql() })
        act(() => result.current.skipEmployeeListMssqlSync())
        expect(result.current.mssqlSyncPreview).toBeNull()
        expect(result.current.mssqlUpdateMessage).toContain("No local data was changed")
        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()
    })

    it("cancelEmployeeListMssqlSync alias works identically to skip", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.updateEmployeeListFromMssql() })
        act(() => result.current.cancelEmployeeListMssqlSync())
        expect(result.current.mssqlSyncPreview).toBeNull()
        expect(result.current.mssqlUpdateMessage).toContain("No local data was changed")
        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()
    })

    it("suppresses preview and shows 10-second no-change message when all records are unchanged", async () => {
        vi.useFakeTimers()
        api.previewEmployeeListMssqlSync.mockResolvedValueOnce({
            previewId: "preview-unchanged",
            createdAt: "2026-10-15T00:00:00Z",
            counts: { new: 0, changed: 0, unchanged: 10, missing: 0, conflicts: 0 },
            rows: [],
            conflicts: [],
        })
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.updateEmployeeListFromMssql() })

        expect(result.current.mssqlSyncPreview).toBeNull()
        expect(result.current.mssqlUpdateStatus).toBe("success")
        expect(result.current.mssqlUpdateMessage).toBe("✓ No changes")
        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()

        act(() => {
            vi.advanceTimersByTime(10000)
        })
        expect(result.current.mssqlUpdateMessage).toBeNull()
        expect(result.current.mssqlUpdateStatus).toBe("idle")
        vi.useRealTimers()
    })

    it("does not invoke apply API when records are missing-only (report-only)", async () => {
        api.previewEmployeeListMssqlSync.mockResolvedValueOnce({
            previewId: "preview-missing-only",
            createdAt: "2026-10-15T00:00:00Z",
            counts: { new: 0, changed: 0, unchanged: 5, missing: 3, conflicts: 0 },
            rows: [],
            conflicts: [],
        })
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.updateEmployeeListFromMssql() })
        expect(result.current.mssqlSyncPreview?.counts.missing).toBe(3)

        await act(async () => { await result.current.applyEmployeeListMssqlSync() })
        expect(api.applyEmployeeListMssqlSync).not.toHaveBeenCalled()
    })

    it("does not invoke apply API when preview contains blocking conflicts", async () => {
        api.previewEmployeeListMssqlSync.mockResolvedValueOnce({
            previewId: "preview-conflict",
            createdAt: "2026-10-15T00:00:00Z",
            counts: { new: 2, changed: 1, unchanged: 5, missing: 0, conflicts: 1 },
            rows: [],
            conflicts: [{ code: "ASWVN999", reason: "Belongs to onboarding" }],
        })
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => { await result.current.updateEmployeeListFromMssql() })
        expect(result.current.mssqlSyncPreview?.counts.conflicts).toBe(1)

        await act(async () => { await result.current.applyEmployeeListMssqlSync() })
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

    it("renders exactly two preview buttons: Skip | Approve with proper gating", () => {
        const source = readFileSync(resolve(process.cwd(), "src/features/employees/EmployeeView.tsx"), "utf8")

        expect(source).toContain(">Skip</button>")
        expect(source).toContain(">{emp.isApplyingMssqlSync ? \"Applying…\" : \"Approve\"}</button>")
        expect(source).not.toContain("Approve and update EE List")
        expect(source).not.toContain(">Cancel</button>")
        expect(source).toContain("onClick={emp.skipEmployeeListMssqlSync}")
        expect(source).toContain("const hasImportableChanges =")
        expect(source).toContain("const canApprove = hasImportableChanges && !hasConflicts")
    })

    it("anchors the preview as a floating popover and eliminates unchanged rows UI", () => {
        const source = readFileSync(resolve(process.cwd(), "src/features/employees/EmployeeView.tsx"), "utf8")

        // Floating popover anchored to Update button
        expect(source).toContain("absolute left-0 top-full z-40 mt-2")
        expect(source).toContain('role="dialog"')
        expect(source).toContain('aria-label="EE List sync preview"')

        // Unchanged rows UI completely removed
        expect(source).not.toContain("Unchanged rows (")
        expect(source).not.toContain("unchangedRows")

        // View details toggle
        expect(source).toContain("View details")
        expect(source).toContain("Hide details")

        // Mobile card renders team pill
        expect(source).toContain("getTeamPillClasses")
        expect(source).toContain("employee.teamName")
    })
})
