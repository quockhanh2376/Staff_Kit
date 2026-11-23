import { renderHook, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { staffApi } from "../../services/staff-api"
import { useTeamState } from "./useTeamState"

describe("useTeamState", () => {
    beforeEach(() => {
        vi.restoreAllMocks()
    })

    it("loads teams only once the Team view is enabled", async () => {
        const listTeams = vi.spyOn(staffApi, "listTeams").mockResolvedValue([])
        const baseOptions = {
            dbReady: true,
            isAuthenticated: true,
            reloadToken: 0,
            setGlobalError: vi.fn(),
            triggerReload: vi.fn(),
        }

        const { rerender } = renderHook((props: { enabled: boolean }) =>
            useTeamState({ ...baseOptions, enabled: props.enabled }),
            { initialProps: { enabled: false } },
        )
        expect(listTeams).not.toHaveBeenCalled()

        rerender({ enabled: true })
        await waitFor(() => expect(listTeams).toHaveBeenCalledTimes(1))
    })
})
