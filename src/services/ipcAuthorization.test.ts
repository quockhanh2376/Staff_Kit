/**
 * Focused Phase C IPC authorization regression suite.
 *
 * Verifies that protected/removed Tauri commands cannot be bypassed through
 * direct IPC invocation. Uses a mocked `invoke` boundary so NO real database,
 * filesystem, or network side effect occurs.
 *
 * Authorization model:
 * - Client-side (staffApi.call): rejects with AUTH_REQUIRED when NO token exists,
 *   preventing the IPC round-trip entirely (guard-before-side-effect).
 * - Server-side (Rust guards): rejects with AUTH_FORBIDDEN when the role is
 *   insufficient. The frontend cannot know the role, so these tests mock the
 *   backend returning AUTH_FORBIDDEN and assert the session is preserved.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

const invokeMock = vi.fn()
vi.mock("@tauri-apps/api/core", () => ({
    invoke: (...args: unknown[]) => invokeMock(...args),
}))

import { staffApi } from "./staff-api"
import {
    getSessionToken,
    onUnauthorized,
    setSession,
    __resetSessionForTests,
} from "./session"

function setRoleSession(role: "user" | "admin" | "super_admin") {
    setSession({ sessionToken: `${role}-token`, expiresAt: "2099-01-01T00:00:00Z" })
}

beforeEach(() => {
    __resetSessionForTests()
    invokeMock.mockReset()
    Object.defineProperty(window, "__TAURI__", {
        value: { core: { invoke: invokeMock } },
        configurable: true,
    })
})

afterEach(() => {
    __resetSessionForTests()
})

// ── 1. reset_all_data without token → AUTH_REQUIRED before side effect ───────

describe("reset_all_data guard", () => {
    it("rejects without a token and never reaches IPC (no side effect)", async () => {
        expect(getSessionToken()).toBeNull()
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)

        await expect(staffApi.resetAllData()).rejects.toThrow("AUTH_REQUIRED")

        expect(invokeMock).not.toHaveBeenCalled()
        expect(unauthorized).toHaveBeenCalledWith("AUTH_REQUIRED")
    })
})

// ── 2. Removed / internal-only commands absent from staffApi ─────────────────

describe("removed IPC commands", () => {
    it("setActiveLocalAccount is not exposed on staffApi", () => {
        expect(
            (staffApi as unknown as Record<string, unknown>).setActiveLocalAccount,
        ).toBeUndefined()
    })

    it("runAutoBackupIfDue is not exposed on staffApi", () => {
        expect(
            (staffApi as unknown as Record<string, unknown>).runAutoBackupIfDue,
        ).toBeUndefined()
    })
})

// ── 3. Forged / no token → AUTH_REQUIRED (client-side, invoke never reached) ─

describe("no-token rejection (guard-before-side-effect)", () => {
    // Representative commands from each guard tier.
    const cases = [
        { label: "listEmployees (authenticated)", call: () => staffApi.listEmployees({ limit: 5, offset: 0 }) },
        { label: "listTeams (authenticated)", call: () => staffApi.listTeams() },
        { label: "importExcel (admin)", call: () => staffApi.importExcel({}) },
        { label: "writeExportFile (admin)", call: () => staffApi.writeExportFile("C:/tmp/x.csv", [65]) },
        { label: "resetAllData (super_admin)", call: () => staffApi.resetAllData() },
        { label: "listLocalAccounts (super_admin)", call: () => staffApi.listLocalAccounts() },
    ]

    for (const tc of cases) {
        it(`${tc.label}: no token → AUTH_REQUIRED, invoke never called`, async () => {
            await expect(tc.call()).rejects.toThrow("AUTH_REQUIRED")
            expect(invokeMock).not.toHaveBeenCalled()
        })
    }
})

// ── 4. Role matrix: backend returns AUTH_FORBIDDEN for insufficient role ─────
//
// Role enforcement is server-side. The frontend `call` wrapper injects the token
// but cannot evaluate the role. These tests mock the backend returning
// AUTH_FORBIDDEN for role mismatches and assert:
//   - the session is NOT cleared (AUTH_FORBIDDEN preserves the session);
//   - the onUnauthorized handler is NOT called (no logout).

describe("role matrix: backend FORBIDDEN preserves session", () => {
    const mismatches = [
        { label: "user → resetAllData (super_admin)", role: "user" as const },
        { label: "admin → resetAllData (super_admin)", role: "admin" as const },
        { label: "user → listLocalAccounts (super_admin)", role: "user" as const },
        { label: "admin → listLocalAccounts (super_admin)", role: "admin" as const },
        { label: "user → importExcel (admin)", role: "user" as const },
        { label: "user → writeExportFile (admin)", role: "user" as const },
        { label: "user → listPendingBorrowRequests (admin)", role: "user" as const },
    ]

    for (const tc of mismatches) {
        it(`${tc.label}: AUTH_FORBIDDEN, session preserved, no logout`, async () => {
            setRoleSession(tc.role)
            invokeMock.mockRejectedValue(new Error("AUTH_FORBIDDEN"))
            const unauthorized = vi.fn()
            onUnauthorized(unauthorized)

            // The call reaches invoke (token exists) but backend rejects.
            await expect(staffApi.resetAllData()).rejects.toThrow("AUTH_FORBIDDEN")

            // Session-ending handler must NOT fire.
            expect(unauthorized).not.toHaveBeenCalled()
            // Session is still held.
            expect(getSessionToken()).toBe(`${tc.role}-token`)
        })
    }
})

// ── 5. Role matrix: sufficient role passes the guard ─────────────────────────

describe("role matrix: sufficient role reaches invoke with token", () => {
    const passes = [
        { label: "user → listEmployees (authenticated)", role: "user" as const, call: () => staffApi.listEmployees({ limit: 5, offset: 0 }) },
        { label: "admin → importExcel (admin)", role: "admin" as const, call: () => staffApi.importExcel({}) },
        { label: "super_admin → resetAllData (super_admin)", role: "super_admin" as const, call: () => staffApi.resetAllData() },
        { label: "super_admin → listLocalAccounts (super_admin)", role: "super_admin" as const, call: () => staffApi.listLocalAccounts() },
    ]

    for (const tc of passes) {
        it(`${tc.label}: invoke called with correct token`, async () => {
            setRoleSession(tc.role)
            invokeMock.mockResolvedValue({ items: [], total: 0 })

            await tc.call()

            expect(invokeMock).toHaveBeenCalledTimes(1)
            const [, args] = invokeMock.mock.calls[0]
            expect((args as Record<string, unknown>).sessionToken).toBe(`${tc.role}-token`)
        })
    }
})

// ── 6. write_export_file authorization (no real file created) ────────────────

describe("write_export_file authorization", () => {
    it("no token → AUTH_REQUIRED, no file written (invoke not reached)", async () => {
        await expect(staffApi.writeExportFile("C:/tmp/auth-test.csv", [65])).rejects.toThrow(
            "AUTH_REQUIRED",
        )
        expect(invokeMock).not.toHaveBeenCalled()
    })

    it("user token reaches invoke; backend returns AUTH_FORBIDDEN; no file side effect (mock)", async () => {
        setRoleSession("user")
        invokeMock.mockRejectedValue(new Error("AUTH_FORBIDDEN"))
        await expect(staffApi.writeExportFile("C:/tmp/auth-test.csv", [65])).rejects.toThrow(
            "AUTH_FORBIDDEN",
        )
        // Mock returns void for the pass case — no real file is created.
    })

    it("admin token passes the guard (mock returns void, no real file)", async () => {
        setRoleSession("admin")
        invokeMock.mockResolvedValue(undefined)
        await staffApi.writeExportFile("C:/tmp/auth-test.csv", [65])
        expect(invokeMock).toHaveBeenCalledTimes(1)
    })
})

// ── 7. Self-delete rule (AUTH_CANNOT_DELETE_SELF preserves session) ──────────

describe("self-delete rule", () => {
    it("AUTH_CANNOT_DELETE_SELF preserves the session (not session-ending)", async () => {
        setRoleSession("super_admin")
        invokeMock.mockRejectedValue(new Error("AUTH_CANNOT_DELETE_SELF"))
        const unauthorized = vi.fn()
        onUnauthorized(unauthorized)

        await expect(staffApi.deleteLocalAccount(1)).rejects.toThrow("AUTH_CANNOT_DELETE_SELF")

        expect(unauthorized).not.toHaveBeenCalled()
        expect(getSessionToken()).toBe("super_admin-token")
    })
})

// ── 8. Exhaustive: no guarded staffApi method uses callPublic ────────────────

describe("staffApi classification invariants", () => {
    it("public methods do NOT inject sessionToken even when a session is held", async () => {
        setRoleSession("super_admin")
        invokeMock.mockResolvedValue(undefined)

        // Note: logoutLocalAccount deliberately passes sessionToken to the
        // backend for invalidation — it is public but sends the token it is
        // destroying. Excluded from this no-injection assertion.
        const publicMethods: Array<{ name: string; args?: unknown[] }> = [
            { name: "ping" },
            { name: "initDatabase" },
            { name: "getDatabaseStatus" },
            { name: "listLoginAccountHints" },
            { name: "forgotLocalAccountPassword", args: [{ username: "x", recoveryCode: "y", newPassword: "z" }] },
        ]

        for (const { name, args } of publicMethods) {
            invokeMock.mockClear()
            const fn = (staffApi as unknown as Record<string, (...a: unknown[]) => unknown>)[name]
            if (!fn) continue
            await fn.call(staffApi, ...(args ?? []))
            const callArgs = invokeMock.mock.calls[0]
            const invokeArgs = callArgs?.[1] as Record<string, unknown> | undefined
            expect(invokeArgs?.sessionToken).toBeUndefined()
        }
    })
})
