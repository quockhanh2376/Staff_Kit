import { act, renderHook } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { readFileSync } from "node:fs"
import { resolve } from "node:path"

const api = vi.hoisted(() => ({
    getMssqlConnectionDefaults: vi.fn(),
    testMssqlConnection: vi.fn(),
    importMssqlStaff: vi.fn(),
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
    api.getMssqlConnectionDefaults.mockResolvedValue({
        host: "  sql.example  ",
        port: 1433,
        user: "  staff-user  ",
        password: "secret",
    })
    api.testMssqlConnection.mockResolvedValue(true)
    api.importMssqlStaff.mockResolvedValue({ updated: 2, imported: 1, failed: 0 })
})

describe("employee MSSQL update flow", () => {
    it("trims host and username, and preflights before importing", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))

        await act(async () => {
            await result.current.updateEmployeeListFromMssql()
        })

        expect(api.testMssqlConnection).toHaveBeenCalledWith(
            "sql.example",
            1433,
            "staff-user",
            "secret",
        )
        expect(api.importMssqlStaff).toHaveBeenCalledWith(
            "sql.example",
            1433,
            "staff-user",
            "secret",
            undefined,
            "employee_list",
        )
        expect(api.testMssqlConnection.mock.invocationCallOrder[0]).toBeLessThan(
            api.importMssqlStaff.mock.invocationCallOrder[0],
        )
    })

    it("blocks the employee import when the preflight fails", async () => {
        const rawError = "MSSQL network connection failed: password=secret; host=sql.internal"
        api.testMssqlConnection.mockRejectedValue(new Error(rawError))
        const setGlobalError = vi.fn()
        const { result } = renderHook(() =>
            useEmployeeState({ ...baseOptions, setGlobalError }),
        )

        await act(async () => {
            await result.current.updateEmployeeListFromMssql()
        })

        expect(api.importMssqlStaff).not.toHaveBeenCalled()
        expect(result.current.mssqlUpdateStatus).toBe("error")
        expect(result.current.mssqlUpdateMessage).toContain("Cannot connect")
        expect(result.current.mssqlUpdateMessage).not.toContain(rawError)
        expect(setGlobalError).toHaveBeenCalledWith(result.current.mssqlUpdateMessage)
    })

    it.each([
        ["MSSQL authentication failed: password=secret", "authentication failed"],
        ["MSSQL query failed: SELECT * FROM employee_records", "query failed"],
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
