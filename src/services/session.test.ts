import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import {
    clearSession,
    getSessionExpiresAt,
    getSessionToken,
    isSessionExpiredByClock,
    notifyUnauthorized,
    onUnauthorized,
    setSession,
    __resetSessionForTests,
} from "./session"

describe("session (in-memory only)", () => {
    beforeEach(() => {
        __resetSessionForTests()
    })
    afterEach(() => {
        __resetSessionForTests()
        vi.restoreAllMocks()
    })

    it("starts unauthenticated with no token", () => {
        expect(getSessionToken()).toBeNull()
        expect(getSessionExpiresAt()).toBeNull()
    })

    it("setSession stores the token in memory and getSessionToken returns it", () => {
        setSession({ sessionToken: "tok-123", expiresAt: "2099-01-01T00:00:00Z" })
        expect(getSessionToken()).toBe("tok-123")
        expect(getSessionExpiresAt()).toBe("2099-01-01T00:00:00Z")
    })

    it("clearSession removes the token", () => {
        setSession({ sessionToken: "tok-123", expiresAt: "2099-01-01T00:00:00Z" })
        clearSession()
        expect(getSessionToken()).toBeNull()
        expect(getSessionExpiresAt()).toBeNull()
    })

    it("isSessionExpiredByClock returns false for a far-future expiry", () => {
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })
        expect(isSessionExpiredByClock()).toBe(false)
    })

    it("isSessionExpiredByClock returns true for a past expiry", () => {
        setSession({ sessionToken: "tok", expiresAt: "2000-01-01T00:00:00Z" })
        expect(isSessionExpiredByClock()).toBe(true)
    })

    it("isSessionExpiredByClock defers to backend on unparseable expiry (returns false)", () => {
        setSession({ sessionToken: "tok", expiresAt: "not-a-date" })
        expect(isSessionExpiredByClock()).toBe(false)
    })

    it("does not persist to localStorage, sessionStorage, or IndexedDB", () => {
        const lsSet = vi.spyOn(Storage.prototype, "setItem")
        const ssSet = vi.spyOn(Storage.prototype, "setItem")
        setSession({ sessionToken: "secret-token", expiresAt: "2099-01-01T00:00:00Z" })
        expect(getSessionToken()).toBe("secret-token")
        // No persistence calls were made for the token.
        for (const call of lsSet.mock.calls) {
            expect(String(call[1])).not.toContain("secret-token")
        }
        for (const call of ssSet.mock.calls) {
            expect(String(call[1])).not.toContain("secret-token")
        }
    })

    it("never logs the token to the console", () => {
        const logSpy = vi.spyOn(console, "log").mockImplementation(() => {})
        const infoSpy = vi.spyOn(console, "info").mockImplementation(() => {})
        const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {})
        const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {})
        setSession({ sessionToken: "do-not-log-me", expiresAt: "2099-01-01T00:00:00Z" })
        clearSession()
        notifyUnauthorized("AUTH_REQUIRED")
        for (const spy of [logSpy, infoSpy, warnSpy, errorSpy]) {
            for (const call of spy.mock.calls) {
                expect(String(call)).not.toContain("do-not-log-me")
            }
        }
    })

    it("onUnauthorized fires for AUTH_REQUIRED and clears the session", () => {
        const listener = vi.fn()
        const unsub = onUnauthorized(listener)
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })
        expect(getSessionToken()).toBe("tok")

        notifyUnauthorized("AUTH_REQUIRED")

        expect(listener).toHaveBeenCalledWith("AUTH_REQUIRED")
        expect(getSessionToken()).toBeNull()
        unsub()
    })

    it("onUnauthorized fires for AUTH_SESSION_EXPIRED and clears the session", () => {
        const listener = vi.fn()
        const unsub = onUnauthorized(listener)
        setSession({ sessionToken: "tok", expiresAt: "2099-01-01T00:00:00Z" })

        notifyUnauthorized("AUTH_SESSION_EXPIRED")

        expect(listener).toHaveBeenCalledWith("AUTH_SESSION_EXPIRED")
        expect(getSessionToken()).toBeNull()
        unsub()
    })

    it("unsubscribe stops further notifications (timer/listener cleanup)", () => {
        const listener = vi.fn()
        const unsub = onUnauthorized(listener)
        unsub()
        notifyUnauthorized("AUTH_REQUIRED")
        expect(listener).not.toHaveBeenCalled()
    })

    it("a listener throwing does not break other listeners or the call site", () => {
        const boom = vi.fn(() => {
            throw new Error("boom")
        })
        const ok = vi.fn()
        onUnauthorized(boom)
        onUnauthorized(ok)
        expect(() => notifyUnauthorized("AUTH_REQUIRED")).not.toThrow()
        expect(boom).toHaveBeenCalled()
        expect(ok).toHaveBeenCalledWith("AUTH_REQUIRED")
    })
})
