import { act, renderHook } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

const api = vi.hoisted(() => ({
    moveEmployeesGroup: vi.fn(),
}))

vi.mock("../../services/staff-api", () => ({ staffApi: api }))

import { useTableEdit } from "./useTableEdit"

// The backend (db/employee.rs move_employees_group_conn) accepts these action values.
const BACKEND_ACTIONS = ["set_floating", "clear_floating", "reject_offer"]

describe("useTableEdit floating moves", () => {
    beforeEach(() => {
        vi.clearAllMocks()
        api.moveEmployeesGroup.mockResolvedValue(1)
        vi.spyOn(window, "confirm").mockReturnValue(true)
    })

    afterEach(() => {
        vi.restoreAllMocks()
    })

    it.each(["set_floating", "clear_floating"])("sends action %s that the backend accepts", async (target) => {
        const { result } = renderHook(() =>
            useTableEdit({
                employees: [],
                staffGroupFilter: "employee_list",
                canEditEmployeeTable: true,
                canEditEmployeeComputerName: true,
                triggerReload: vi.fn(),
                setGlobalError: vi.fn(),
            }),
        )

        act(() => {
            result.current.selectEmployeeIds([7])
            result.current.setMoveTargetGroup(target)
        })
        await act(async () => {
            await result.current.handleMoveSelectedEmployees()
        })

        expect(api.moveEmployeesGroup).toHaveBeenCalledTimes(1)
        const payload = api.moveEmployeesGroup.mock.calls[0][0]
        expect(payload.employeeIds).toEqual([7])
        expect(payload.action).toBe(target)
        expect(BACKEND_ACTIONS).toContain(payload.action)
    })
})
