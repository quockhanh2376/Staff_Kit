import { act, renderHook } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import type { EmployeeRecord } from "../../types/staff"

const api = vi.hoisted(() => ({
    searchEmployees: vi.fn(),
    listTeams: vi.fn(),
    listEmployeeGroupCounts: vi.fn(),
    previewEmployeeListMssqlSync: vi.fn(),
    applyEmployeeListMssqlSync: vi.fn(),
}))

vi.mock("../../services/staff-api", () => ({ staffApi: api }))

import { useEmployeeState } from "./useEmployeeState"

const baseOptions = {
    dbReady: true,
    isAuthenticated: true,
    reloadToken: 0,
    setGlobalError: vi.fn(),
}

describe("useEmployeeState search debounce and race condition handling", () => {
    beforeEach(() => {
        vi.useFakeTimers()
        vi.clearAllMocks()
        api.listTeams.mockResolvedValue([])
        api.listEmployeeGroupCounts.mockResolvedValue({
            employeeList: 0,
            onboarding: 0,
            offboarding: 0,
            internalMovement: 0,
            floating: 0,
            total: 0,
        })
        api.searchEmployees.mockResolvedValue({ items: [], total: 0 })
    })

    afterEach(() => {
        vi.useRealTimers()
    })

    it("debounces rapid typing by 250ms before calling searchEmployees", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))

        // Initial load with empty search term
        await act(async () => {
            await Promise.resolve()
        })
        expect(api.searchEmployees).toHaveBeenCalledTimes(1)
        expect(api.searchEmployees).toHaveBeenLastCalledWith(
            expect.objectContaining({ query: null }),
        )

        // Type 'A', 'An', 'Ant' rapidly
        act(() => {
            result.current.setSearchTerm("A")
        })
        act(() => {
            vi.advanceTimersByTime(100)
            result.current.setSearchTerm("An")
        })
        act(() => {
            vi.advanceTimersByTime(100)
            result.current.setSearchTerm("Ant")
        })

        // Still within 250ms of the last keystroke; search should not have fired for 'Ant' yet
        expect(api.searchEmployees).toHaveBeenCalledTimes(1)

        // Advance past 250ms
        await act(async () => {
            vi.advanceTimersByTime(250)
            await Promise.resolve()
        })

        expect(api.searchEmployees).toHaveBeenCalledTimes(2)
        expect(api.searchEmployees).toHaveBeenLastCalledWith(
            expect.objectContaining({ query: "Ant" }),
        )
    })

    it("discards stale responses if an earlier search resolves after a newer one", async () => {
        let resolveFirstSearch!: (val: { items: EmployeeRecord[]; total: number }) => void
        let resolveSecondSearch!: (val: { items: EmployeeRecord[]; total: number }) => void

        const firstPromise = new Promise<{ items: EmployeeRecord[]; total: number }>((res) => {
            resolveFirstSearch = res
        })
        const secondPromise = new Promise<{ items: EmployeeRecord[]; total: number }>((res) => {
            resolveSecondSearch = res
        })

        // Initial call resolves immediately
        api.searchEmployees.mockResolvedValueOnce({ items: [], total: 0 })

        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => {
            await Promise.resolve()
        })

        // 1st search returns firstPromise
        api.searchEmployees.mockReturnValueOnce(firstPromise)
        act(() => {
            result.current.setSearchTerm("slow")
        })
        await act(async () => {
            vi.advanceTimersByTime(250)
            await Promise.resolve()
        })

        // 2nd search returns secondPromise
        api.searchEmployees.mockReturnValueOnce(secondPromise)
        act(() => {
            result.current.setSearchTerm("fast")
        })
        await act(async () => {
            vi.advanceTimersByTime(250)
            await Promise.resolve()
        })

        // 2nd search resolves first with newer data
        await act(async () => {
            resolveSecondSearch({
                items: [{ id: 2, employeeId: "EE002", fullName: "Fast Result" }] as unknown as EmployeeRecord[],
                total: 1,
            })
            await Promise.resolve()
        })

        expect(result.current.employees).toEqual([
            expect.objectContaining({ employeeId: "EE002", fullName: "Fast Result" }),
        ])

        // Now 1st search resolves late with stale data
        await act(async () => {
            resolveFirstSearch({
                items: [{ id: 1, employeeId: "EE001", fullName: "Stale Result" }] as unknown as EmployeeRecord[],
                total: 1,
            })
            await Promise.resolve()
        })

        // Stale result MUST NOT overwrite newer result
        expect(result.current.employees).toEqual([
            expect.objectContaining({ employeeId: "EE002", fullName: "Fast Result" }),
        ])
    })

    it("immediately clears debounced search term when search term is cleared", async () => {
        const { result } = renderHook(() => useEmployeeState(baseOptions))
        await act(async () => {
            await Promise.resolve()
        })

        act(() => {
            result.current.setSearchTerm("query")
        })
        await act(async () => {
            vi.advanceTimersByTime(250)
            await Promise.resolve()
        })
        expect(api.searchEmployees).toHaveBeenLastCalledWith(
            expect.objectContaining({ query: "query" }),
        )

        // Clear search
        act(() => {
            result.current.setSearchTerm("")
        })
        await act(async () => {
            await Promise.resolve()
        })

        expect(api.searchEmployees).toHaveBeenLastCalledWith(
            expect.objectContaining({ query: null }),
        )
    })
})
