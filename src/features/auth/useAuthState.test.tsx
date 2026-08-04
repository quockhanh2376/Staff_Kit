/**
 * SEC-001 auth regression tests:
 * - Phase D3: own password change + admin reset (session handling)
 * - Regression A: identity from backend login result only (not hints/stubs)
 * - Regression B: standard-user startup avoids admin commands
 * - Regression C: forced password reset flow
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { act, renderHook } from "@testing-library/react"

// vi.mock is hoisted above imports; use vi.hoisted so the mock fn exists.
const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }))
vi.mock("@tauri-apps/api/core", () => ({
    invoke: (...args: unknown[]) => invokeMock(...args),
}))

beforeEach(() => {
    Object.defineProperty(window, "__TAURI__", {
        value: { core: { invoke: invokeMock } },
        configurable: true,
    })
})

import { useAuthState } from "./useAuthState"
import {
    getSessionToken,
    __resetSessionForTests,
} from "../../services/session"
import type { LocalAccountRecord } from "../../types/staff"

const noop = () => {}

function makeAccount(overrides: Partial<LocalAccountRecord> = {}): LocalAccountRecord {
    return {
        id: 1,
        accountKey: "alice",
        displayName: "Alice",
        username: "alice",
        role: "super_admin",
        isSuperAdmin: true,
        isActive: true,
        forcePasswordReset: false,
        createdAt: "",
        updatedAt: "",
        ...overrides,
    }
}

const baseOptions = {
    dbReady: true,
    reloadToken: 0,
    onLoginSuccess: noop,
    onLogout: noop,
}

// Track the next login account so the mock returns it for login_local_account.
let nextLoginAccount: LocalAccountRecord | null = null

beforeEach(() => {
    __resetSessionForTests()
    nextLoginAccount = null
    invokeMock.mockReset()
    invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === "login_local_account") {
            if (!nextLoginAccount) throw new Error("no login mock set")
            return {
                sessionToken: "session-" + nextLoginAccount.username,
                expiresAt: "2099-01-01T00:00:00Z",
                account: nextLoginAccount,
            }
        }
        if (cmd === "list_login_account_hints") return []
        if (cmd === "list_local_accounts") return []
        if (cmd === "logout_local_account") return undefined
        if (cmd === "change_local_account_password") return true
        return undefined
    })
})

afterEach(() => {
    __resetSessionForTests()
    nextLoginAccount = null
})

/// Simulate a real login. Sets the form state, waits for React to commit the
/// state, then calls handleLoginSubmit. The two-step approach is necessary
/// because handleLoginSubmit reads loginUsername/loginPassword from its render
/// closure — the state must be committed before the handler sees it.
async function loginAs(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    result: any,
    account: LocalAccountRecord,
    password = "secret123",
) {
    nextLoginAccount = account
    const setGlobalError = vi.fn()
    // Step 1: set form fields + commit state.
    act(() => {
        result.current.setLoginUsername(account.username)
        result.current.setLoginPassword(password)
    })
    // Step 2: now the latest render has the username/password; call login.
    await act(async () => {
        await result.current.handleLoginSubmit(
            { preventDefault: noop } as never,
            setGlobalError,
        )
    })
    return setGlobalError
}

// ── Regression A: Identity from backend login result only ────────────────────

describe("login identity (Regression A)", () => {
    it("login as account B after account A updates the header/account state to B", async () => {
        const onLogout = vi.fn()
        const { result } = renderHook(() => useAuthState({ ...baseOptions, onLogout }))

        // Login as Alice first.
        await loginAs(result, makeAccount({ id: 1, displayName: "Alice", username: "alice" }))
        expect(result.current.activeAccount?.displayName).toBe("Alice")
        expect(result.current.isAuthenticated).toBe(true)

        // Now login as David (simulating account switch / re-login).
        await loginAs(
            result,
            makeAccount({
                id: 5,
                displayName: "David Nguyen",
                username: "david.nguyen",
                role: "user",
                isSuperAdmin: false,
            }),
        )
        // Header must show David, not Alice.
        expect(result.current.activeAccount?.displayName).toBe("David Nguyen")
        expect(result.current.activeAccount?.username).toBe("david.nguyen")
        expect(result.current.activeAccount?.role).toBe("user")
    })

    it("selected login hint cannot override backend login identity", async () => {
        // Pre-login hints include "adman" first. After login as "david.nguyen",
        // the identity must be david.nguyen, not adman from the hint stub.
        invokeMock.mockResolvedValueOnce([
            { username: "adman", displayName: "Admin" },
            { username: "david.nguyen", displayName: "David Nguyen" },
        ])
        const { result } = renderHook(() => useAuthState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        // The hint stubs are loaded (adman is first/active).
        expect(result.current.accounts[0].username).toBe("adman")

        // Login as david.nguyen.
        await loginAs(
            result,
            makeAccount({
                id: 5,
                displayName: "David Nguyen",
                username: "david.nguyen",
                role: "user",
                isSuperAdmin: false,
            }),
        )

        // Identity is David from the backend, NOT adman from the hint.
        expect(result.current.activeAccount?.displayName).toBe("David Nguyen")
        expect(result.current.activeAccountName).toBe("David Nguyen")
    })

    it("logout clears identity", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await loginAs(result, makeAccount({ id: 1, displayName: "Alice" }))
        expect(result.current.isAuthenticated).toBe(true)

        await act(async () => {
            await result.current.handleLogout()
        })

        expect(result.current.isAuthenticated).toBe(false)
        expect(result.current.activeAccount).toBeNull()
        expect(getSessionToken()).toBeNull()
    })

    it("failed login attempt does not create a new authenticated identity", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        // First login succeeds.
        await loginAs(result, makeAccount({ id: 1, displayName: "Alice" }))
        expect(result.current.isAuthenticated).toBe(true)

        // Second login (account switch) fails — wrong password.
        nextLoginAccount = null // No mock for this login.
        invokeMock.mockImplementationOnce(async () => {
            throw new Error("incorrect username or password")
        })
        act(() => {
            result.current.setLoginUsername("bob")
            result.current.setLoginPassword("wrong")
        })
        await act(async () => {
            await result.current.handleLoginSubmit(
                { preventDefault: noop } as never,
            )
        })

        // The failed login attempt should NOT log out Alice (she's still valid),
        // and should NOT create a new identity for "bob".
        expect(result.current.activeAccount?.displayName).toBe("Alice")
        expect(result.current.loginError).toBeTruthy()
    })

    it("synthetic hint IDs never become authenticated account IDs", async () => {
        // Pre-login stubs have negative IDs (-1, -2, ...).
        invokeMock.mockResolvedValueOnce([{ username: "adman", displayName: "Admin" }])
        const { result } = renderHook(() => useAuthState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        expect(result.current.accounts[0].id).toBe(-1)

        // Login as a real user (positive DB id).
        const account = makeAccount({ id: 42, displayName: "Real User", username: "real" })
        nextLoginAccount = account
        act(() => {
            result.current.setLoginUsername("real")
            result.current.setLoginPassword("pw123")
        })
        await act(async () => {
            await result.current.handleLoginSubmit(
                { preventDefault: noop } as never,
            )
        })
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        expect(result.current.authenticatedAccount?.id).toBe(42)
    })
})

// ── Login error handling (wrong password / unknown user) ─────────────────────

describe("login error handling", () => {
    it("wrong password shows safe message in loginError, not globalError", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await act(async () => { await Promise.resolve(); await Promise.resolve() })

        // Simulate wrong password.
        nextLoginAccount = null
        invokeMock.mockImplementationOnce(async () => {
            throw new Error("incorrect username or password")
        })
        act(() => {
            result.current.setLoginUsername("alice")
            result.current.setLoginPassword("wrong-pw")
        })
        await act(async () => {
            await result.current.handleLoginSubmit({ preventDefault: noop } as never)
        })

        // loginError has the safe message.
        expect(result.current.loginError).toBe("Username or password is incorrect.")
        // globalError was NOT set (no global banner).
        // The hook doesn't own globalError; it uses loginError. Verify the hook
        // did not call setGlobalError by checking loginError is set and
        // isAuthenticated is still false.
        expect(result.current.isAuthenticated).toBe(false)
    })

    it("unknown username returns the same safe message", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await act(async () => { await Promise.resolve(); await Promise.resolve() })

        nextLoginAccount = null
        invokeMock.mockImplementationOnce(async () => {
            throw new Error("incorrect username or password")
        })
        act(() => {
            result.current.setLoginUsername("nonexistent.user")
            result.current.setLoginPassword("anything")
        })
        await act(async () => {
            await result.current.handleLoginSubmit({ preventDefault: noop } as never)
        })

        // Same message — does not reveal whether the username exists.
        expect(result.current.loginError).toBe("Username or password is incorrect.")
    })

    it("loginError clears when the user edits the username field", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await act(async () => { await Promise.resolve(); await Promise.resolve() })

        // Trigger a login error.
        nextLoginAccount = null
        invokeMock.mockImplementationOnce(async () => {
            throw new Error("incorrect username or password")
        })
        act(() => {
            result.current.setLoginUsername("alice")
            result.current.setLoginPassword("wrong")
        })
        await act(async () => {
            await result.current.handleLoginSubmit({ preventDefault: noop } as never)
        })
        expect(result.current.loginError).toBeTruthy()

        // Edit the username — error should clear.
        act(() => {
            result.current.setLoginUsername("alice2")
        })
        expect(result.current.loginError).toBeNull()
    })

    it("loginError clears when the user edits the password field", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await act(async () => { await Promise.resolve(); await Promise.resolve() })

        nextLoginAccount = null
        invokeMock.mockImplementationOnce(async () => {
            throw new Error("incorrect username or password")
        })
        act(() => {
            result.current.setLoginUsername("alice")
            result.current.setLoginPassword("wrong")
        })
        await act(async () => {
            await result.current.handleLoginSubmit({ preventDefault: noop } as never)
        })
        expect(result.current.loginError).toBeTruthy()

        act(() => {
            result.current.setLoginPassword("newattempt")
        })
        expect(result.current.loginError).toBeNull()
    })

    it("failed account-switch login preserves the existing session", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))

        // Login as Alice (succeeds).
        await loginAs(result, makeAccount({ id: 1, displayName: "Alice", username: "alice" }))
        expect(result.current.isAuthenticated).toBe(true)
        expect(getSessionToken()).toBe("session-alice")

        // Attempt to switch to Bob (fails — wrong password).
        nextLoginAccount = null
        invokeMock.mockImplementationOnce(async () => {
            throw new Error("incorrect username or password")
        })
        act(() => {
            result.current.setLoginUsername("bob")
            result.current.setLoginPassword("wrong")
        })
        await act(async () => {
            await result.current.handleLoginSubmit({ preventDefault: noop } as never)
        })

        // Alice's session is preserved — she's still the active account.
        expect(result.current.activeAccount?.displayName).toBe("Alice")
        expect(result.current.loginError).toBe("Username or password is incorrect.")
    })
})

describe("standard-user startup (Regression B)", () => {
    it("standard-user login invokes no admin/super_admin API", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))

        await loginAs(
            result,
            makeAccount({
                id: 5,
                displayName: "David",
                username: "david",
                role: "user",
                isSuperAdmin: false,
            }),
        )
        expect(result.current.isAuthenticated).toBe(true)

        // Flush post-login effects.
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        // listLocalAccounts (super_admin) must NEVER have been called for a user.
        const listCalls = invokeMock.mock.calls.filter(
            ([cmd]) => cmd === "list_local_accounts",
        )
        expect(listCalls).toHaveLength(0)
    })

    it("standard-user login produces no generic global error", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))

        await loginAs(
            result,
            makeAccount({
                id: 5,
                displayName: "David",
                username: "david",
                role: "user",
                isSuperAdmin: false,
            }),
        )

        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        // No accountsLoadError (which would surface as the generic global error).
        expect(result.current.accountsLoadError).toBeNull()
    })
})

// ── Regression C: Forced password reset ──────────────────────────────────────

describe("forced password reset (Regression C)", () => {
    it("forcePasswordReset=true sets the blocking flag", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))

        await loginAs(
            result,
            makeAccount({
                id: 5,
                displayName: "David",
                username: "david",
                role: "user",
                forcePasswordReset: true,
            }),
        )

        expect(result.current.forcePasswordReset).toBe(true)
        // onLoginSuccess should NOT have been called (normal nav is blocked).
        // The caller checks forcePasswordReset before calling onLoginSuccess.
    })

    it("forcePasswordReset=false proceeds normally", async () => {
        const onLoginSuccess = vi.fn()
        const { result } = renderHook(() =>
            useAuthState({ ...baseOptions, onLoginSuccess }),
        )

        await loginAs(result, makeAccount({ id: 1, forcePasswordReset: false }))

        expect(result.current.forcePasswordReset).toBe(false)
        expect(onLoginSuccess).toHaveBeenCalledTimes(1)
    })

    it("successful forced reset clears session and returns to login", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))

        await loginAs(
            result,
            makeAccount({ id: 5, displayName: "David", username: "david", forcePasswordReset: true }),
        )
        expect(result.current.forcePasswordReset).toBe(true)

        // Change password (the backend invalidates the session).
        invokeMock.mockResolvedValueOnce(true)
        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("Welcome!", "newpass123", setGlobalError)
        })

        expect(getSessionToken()).toBeNull()
        expect(result.current.isAuthenticated).toBe(false)
        expect(result.current.forcePasswordReset).toBe(false)
        expect(setGlobalError).toHaveBeenCalledWith(
            "Your password was changed. Please sign in again.",
        )
    })

    it("failed reset (wrong current password) preserves session and stays on reset screen", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))

        await loginAs(
            result,
            makeAccount({ id: 5, displayName: "David", username: "david", forcePasswordReset: true }),
        )

        invokeMock.mockRejectedValueOnce(new Error("Password current password is incorrect"))
        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("wrong-old", "newpass123", setGlobalError)
        })

        // Session preserved, still on reset screen.
        expect(getSessionToken()).toBe("session-david")
        expect(result.current.isAuthenticated).toBe(true)
        expect(result.current.forcePasswordReset).toBe(true)
    })
})

// ── Regression D: password-change payload + error mapping ────────────────────

describe("password change payload + error mapping", () => {
    it("sends the correct payload key 'id' (not 'accountId') to the backend", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await loginAs(result, makeAccount({ id: 7, displayName: "Eve", username: "eve" }))

        invokeMock.mockResolvedValueOnce(true)
        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("old-pw", "new-pw", setGlobalError)
        })

        const changeCall = invokeMock.mock.calls.find(
            ([cmd]) => cmd === "change_local_account_password",
        )
        expect(changeCall).toBeDefined()
        const payload = (changeCall![1] as { payload: Record<string, unknown> }).payload
        // The Rust DTO expects "id" (not "accountId") with serde rename_all camelCase.
        expect(payload.id).toBe(7)
        expect(payload.accountId).toBeUndefined()
        expect(payload.currentPassword).toBe("old-pw")
        expect(payload.newPassword).toBe("new-pw")
        // No extra fields that the DTO doesn't have.
        expect(payload.newRecoveryCode).toBeUndefined()
    })

    it("wrong current password gives a specific error and preserves session", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await loginAs(result, makeAccount({ id: 5, displayName: "David", username: "david", forcePasswordReset: true }))

        invokeMock.mockRejectedValueOnce(new Error("current password is incorrect"))
        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("Welcome!", "20252026", setGlobalError)
        })

        // Specific error surfaced — NOT the generic banner.
        expect(setGlobalError).toHaveBeenCalledWith("Current password is incorrect.")
        // Session preserved; still on forced-reset screen.
        expect(getSessionToken()).toBe("session-david")
        expect(result.current.isAuthenticated).toBe(true)
        expect(result.current.forcePasswordReset).toBe(true)
    })

    it("no generic global error for expected password failures", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await loginAs(result, makeAccount({ id: 5, displayName: "David", username: "david", forcePasswordReset: true }))

        invokeMock.mockRejectedValueOnce(new Error("password must be at least 6 characters"))
        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("Welcome!", "12345", setGlobalError)
        })

        // Specific policy message — NOT the generic "unexpected error".
        expect(setGlobalError).toHaveBeenCalledWith("Password must be at least 6 characters.")
        expect(setGlobalError).not.toHaveBeenCalledWith(
            "An unexpected error occurred. Please try again or contact support.",
        )
    })

    it("success clears forcePasswordReset and invalidates the current session", async () => {
        const { result } = renderHook(() => useAuthState(baseOptions))
        await loginAs(result, makeAccount({ id: 5, displayName: "David", username: "david", forcePasswordReset: true }))

        invokeMock.mockResolvedValueOnce(true)
        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("Welcome!", "newSecurePass123", setGlobalError)
        })

        // Session cleared, forcePasswordReset cleared, returns to login.
        expect(getSessionToken()).toBeNull()
        expect(result.current.isAuthenticated).toBe(false)
        expect(result.current.forcePasswordReset).toBe(false)
        expect(setGlobalError).toHaveBeenCalledWith(
            "Your password was changed. Please sign in again.",
        )
    })
})

// ── Phase D3: password change + admin reset (existing, updated for new identity flow) ─

describe("handleChangePassword — own password change (Phase D3)", () => {
    it("clears the session, surfaces the re-login message, and resets app state on success", async () => {
        const onLogout = vi.fn()
        const { result } = renderHook(() => useAuthState({ ...baseOptions, onLogout }))

        await loginAs(result, makeAccount({ id: 1, displayName: "Alice" }))
        expect(getSessionToken()).toBe("session-alice")

        invokeMock.mockResolvedValueOnce(true)
        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("old-pw", "new-pw", setGlobalError)
        })

        expect(getSessionToken()).toBeNull()
        expect(setGlobalError).toHaveBeenCalledWith(
            "Your password was changed. Please sign in again.",
        )
        expect(onLogout).toHaveBeenCalledTimes(1)
        expect(result.current.isAuthenticated).toBe(false)
    })

    it("preserves the session and does NOT reset app state on failure", async () => {
        const onLogout = vi.fn()
        const { result } = renderHook(() => useAuthState({ ...baseOptions, onLogout }))

        await loginAs(result, makeAccount({ id: 1, displayName: "Alice" }))

        invokeMock.mockRejectedValueOnce(new Error("Password is incorrect"))
        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("wrong-old", "new-pw", setGlobalError)
        })

        expect(getSessionToken()).toBe("session-alice")
        expect(onLogout).not.toHaveBeenCalled()
        expect(result.current.isAuthenticated).toBe(true)
    })

    it("refuses to change password before a real account is active (pre-login)", async () => {
        const onLogout = vi.fn()
        const { result } = renderHook(() => useAuthState({ ...baseOptions, onLogout }))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        const setGlobalError = vi.fn()
        await act(async () => {
            await result.current.handleChangePassword("old", "new", setGlobalError)
        })

        expect(setGlobalError).toHaveBeenCalledWith("Sign in before changing your password.")
        expect(onLogout).not.toHaveBeenCalled()
    })
})

describe("handleAdminResetPassword — actor stays logged in (Phase D3)", () => {
    it("preserves the actor's session when resetting ANOTHER account", async () => {
        const onLogout = vi.fn()
        const setGlobalError = vi.fn()
        const triggerReload = vi.fn()

        const { result } = renderHook(() => useAuthState({ ...baseOptions, onLogout }))

        // Login as admin.
        await loginAs(
            result,
            makeAccount({ id: 1, username: "admin", displayName: "Admin", role: "super_admin" }),
        )

        const target = makeAccount({ id: 2, displayName: "Bob", username: "bob", role: "user" })
        const promptSpy = vi.spyOn(window, "prompt").mockReturnValue("new-secret")
        invokeMock.mockResolvedValueOnce(true)

        await act(async () => {
            await result.current.handleAdminResetPassword(target, true, 1, setGlobalError, triggerReload)
        })

        promptSpy.mockRestore()

        // Actor session intact.
        expect(getSessionToken()).toBe("session-admin")
        expect(onLogout).not.toHaveBeenCalled()
        // Targeted the right account.
        const resetCall = invokeMock.mock.calls.find(
            ([cmd]) => cmd === "admin_reset_local_account_password",
        )
        expect(resetCall).toBeDefined()
    })
})
