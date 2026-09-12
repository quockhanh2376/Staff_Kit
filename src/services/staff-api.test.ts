import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

// Mock the Tauri invoke surface BEFORE importing staffApi.
const invokeMock = vi.fn()
vi.mock("@tauri-apps/api/core", () => ({
    invoke: (...args: unknown[]) => invokeMock(...args),
}))

// Stub the Tauri runtime globals so ensureTauriRuntime() passes.
beforeEach(() => {
    Object.defineProperty(window, "__TAURI__", {
        value: { core: { invoke: invokeMock } },
        configurable: true,
    })
})

import { staffApi } from "./staff-api"
import {
    getSessionToken,
    onUnauthorized,
    setSession,
    __resetSessionForTests,
} from "./session"

describe("staffApi session injection", () => {
    beforeEach(() => {
        __resetSessionForTests()
        invokeMock.mockReset()
    })
    afterEach(() => {
        __resetSessionForTests()
    })

    it("guarded call injects sessionToken when a session is held", async () => {
        setSession({ sessionToken: "real-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockResolvedValue([{ id: 1, employeeId: "EE1", fullName: "A" }])
        await staffApi.listEmployees({ limit: 10, offset: 0 })
        expect(invokeMock).toHaveBeenCalledTimes(1)
        const [command, args] = invokeMock.mock.calls[0]
        expect(command).toBe("list_employees")
        expect((args as Record<string, unknown>).sessionToken).toBe("real-token")
    })

    it("guarded call with NO session rejects client-side as AUTH_REQUIRED without invoking", async () => {
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)
        await expect(staffApi.listEmployees({ limit: 10, offset: 0 })).rejects.toThrow(
            "AUTH_REQUIRED",
        )
        expect(invokeMock).not.toHaveBeenCalled()
        expect(unauthorized).toHaveBeenCalledWith("AUTH_REQUIRED")
        expect(getSessionToken()).toBeNull()
    })

    it("public call omits sessionToken entirely", async () => {
        setSession({ sessionToken: "should-not-leak", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockResolvedValue("pong")
        await staffApi.ping()
        const [, args] = invokeMock.mock.calls[0]
        expect(args).toBeUndefined()
    })

    it("initDatabase is public (no token required)", async () => {
        invokeMock.mockResolvedValue({ initialized: true })
        await staffApi.initDatabase()
        const [, args] = invokeMock.mock.calls[0]
        expect(args).toBeUndefined()
    })

    it("login stores the returned session token in memory", async () => {
        invokeMock.mockResolvedValue({
            sessionToken: "minted-token",
            expiresAt: "2099-01-01T00:00:00Z",
            account: {
                id: 5,
                accountKey: "alice",
                displayName: "Alice",
                username: "alice",
                role: "admin",
                isSuperAdmin: false,
                isActive: true,
                forcePasswordReset: false,
                createdAt: "",
                updatedAt: "",
            },
        })
        // Login must not require an existing session.
        expect(getSessionToken()).toBeNull()
        const result = await staffApi.loginLocalAccount({ username: "alice", password: "pw" })
        expect(result.sessionToken).toBe("minted-token")
        expect(getSessionToken()).toBe("minted-token")
        // Login is session-exempt: no token injected into its invoke args.
        const [, loginArgs] = invokeMock.mock.calls[0]
        expect((loginArgs as Record<string, unknown>).sessionToken).toBeUndefined()
    })

    it("logout calls the backend with the current token then clears it", async () => {
        setSession({ sessionToken: "current-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockResolvedValue(undefined)
        await staffApi.logoutLocalAccount()
        const [command, args] = invokeMock.mock.calls[0]
        expect(command).toBe("logout_local_account")
        expect((args as Record<string, unknown>).sessionToken).toBe("current-token")
        expect(getSessionToken()).toBeNull()
    })

    it("logout with no session is a no-op (no IPC, no throw)", async () => {
        await expect(staffApi.logoutLocalAccount()).resolves.toBeUndefined()
        expect(invokeMock).not.toHaveBeenCalled()
        expect(getSessionToken()).toBeNull()
    })

    it("AUTH_REQUIRED backend error clears the session and notifies", async () => {
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockRejectedValue(new Error("AUTH_REQUIRED"))
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)
        await expect(staffApi.listTeams()).rejects.toThrow("AUTH_REQUIRED")
        expect(unauthorized).toHaveBeenCalledWith("AUTH_REQUIRED")
        expect(getSessionToken()).toBeNull()
    })

    it("AUTH_SESSION_EXPIRED backend error clears the session and notifies", async () => {
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockRejectedValue(new Error("AUTH_SESSION_EXPIRED"))
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)
        await expect(staffApi.listTeams()).rejects.toThrow("AUTH_SESSION_EXPIRED")
        expect(unauthorized).toHaveBeenCalledWith("AUTH_SESSION_EXPIRED")
        expect(getSessionToken()).toBeNull()
    })

    it("AUTH_FORBIDDEN backend error preserves the session (no logout)", async () => {
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockRejectedValue(new Error("AUTH_FORBIDDEN"))
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)
        await expect(staffApi.resetAllData()).rejects.toThrow("AUTH_FORBIDDEN")
        // Session-ending handler must NOT fire for FORBIDDEN.
        expect(unauthorized).not.toHaveBeenCalled()
        // Session is still held.
        expect(getSessionToken()).toBe("tok")
    })

    it("non-auth backend error does not clear the session or notify", async () => {
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockRejectedValue(new Error("assetCode already exists"))
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)
        await expect(staffApi.createAssetManually({} as never)).rejects.toThrow(
            "assetCode already exists",
        )
        expect(unauthorized).not.toHaveBeenCalled()
        expect(getSessionToken()).toBe("tok")
    })

    it("AUTH_CANNOT_DELETE_SELF preserves the session (not session-ending)", async () => {
        // Self-delete rejection must NOT log the user out. The token survives.
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockRejectedValue(new Error("AUTH_CANNOT_DELETE_SELF"))
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)
        await expect(staffApi.deleteLocalAccount(7)).rejects.toThrow("AUTH_CANNOT_DELETE_SELF")
        expect(unauthorized).not.toHaveBeenCalled()
        // Session preserved after rejected self-delete.
        expect(getSessionToken()).toBe("tok")
    })
})

describe("staffApi Phase C command classification", () => {
    beforeEach(() => {
        __resetSessionForTests()
        invokeMock.mockReset()
        Object.defineProperty(window, "__TAURI__", {
            value: { core: { invoke: invokeMock } },
            configurable: true,
        })
    })

    it("setActiveLocalAccount is no longer exposed (removed from IPC)", () => {
        expect((staffApi as unknown as Record<string, unknown>).setActiveLocalAccount).toBeUndefined()
    })

    it("listLoginAccountHints is public (no token injected)", async () => {
        invokeMock.mockResolvedValue([])
        await staffApi.listLoginAccountHints()
        const [, args] = invokeMock.mock.calls[0]
        expect(args).toBeUndefined()
    })

    it("listLocalAccounts is guarded (injects token)", async () => {
        setSession({ sessionToken: "admin-tok", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockResolvedValue([])
        await staffApi.listLocalAccounts()
        const [, args] = invokeMock.mock.calls[0]
        expect((args as Record<string, unknown>).sessionToken).toBe("admin-tok")
    })

    it("listLocalAccounts with no session rejects as AUTH_REQUIRED", async () => {
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)
        await expect(staffApi.listLocalAccounts()).rejects.toThrow("AUTH_REQUIRED")
        expect(invokeMock).not.toHaveBeenCalled()
        expect(unauthorized).toHaveBeenCalledWith("AUTH_REQUIRED")
    })

    it("writeExportFile requires a session (guarded)", async () => {
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockResolvedValue(undefined)
        await staffApi.writeExportFile("C:/tmp/x.csv", [65])
        const [, args] = invokeMock.mock.calls[0]
        expect((args as Record<string, unknown>).sessionToken).toBe("tok")
    })

    it("writeExportFile with no session is rejected before IPC", async () => {
        await expect(staffApi.writeExportFile("C:/tmp/x.csv", [65])).rejects.toThrow("AUTH_REQUIRED")
        expect(invokeMock).not.toHaveBeenCalled()
    })
})
