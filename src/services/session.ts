/**
 * In-memory session store for the frontend (SEC-001 Phase B).
 *
 * The session token lives ONLY in this module's runtime memory. It is never
 * written to localStorage, sessionStorage, IndexedDB, the filesystem, config
 * files, URL/query parameters, or console output. An app restart clears this
 * state and forces re-login; the backend SessionStore is the authoritative
 * source of truth.
 *
 * Other modules read the token via `getSessionToken()` and subscribe to
 * unauthorized events via `onUnauthorized`. The central `staffApi` wrapper
 * injects the token into guarded calls.
 */

export type SessionData = {
    /** Opaque session token issued by the backend on login. Memory-only. */
    readonly sessionToken: string
    /** UX-only absolute expiry (RFC-3339). Backend remains authoritative. */
    readonly expiresAt: string
}

export type UnauthorizedReason = "AUTH_REQUIRED" | "AUTH_SESSION_EXPIRED"

type UnauthorizedListener = (reason: UnauthorizedReason) => void

// ── Module-scoped runtime state (never persisted) ────────────────────────────

let session: SessionData | null = null
const listeners = new Set<UnauthorizedListener>()

// ── Public API ───────────────────────────────────────────────────────────────

/** Current session token, or null if not authenticated. Memory-only. */
export const getSessionToken = (): string | null => session?.sessionToken ?? null

/** Current UX-only expiry timestamp, or null. */
export const getSessionExpiresAt = (): string | null => session?.expiresAt ?? null

/** Store the session returned by a successful login. Overwrites any prior. */
export const setSession = (data: SessionData): void => {
    session = {
        sessionToken: data.sessionToken,
        expiresAt: data.expiresAt,
    }
}

/** Clear the session (logout, expiry, or session-ending backend error). */
export const clearSession = (): void => {
    session = null
}

/** True when a session is held and its UX-only absolute expiry has passed. */
export const isSessionExpiredByClock = (): boolean => {
    if (!session) return true
    const expiry = Date.parse(session.expiresAt)
    if (Number.isNaN(expiry)) return false // unknown expiry: defer to backend
    return Date.now() >= expiry
}

/** Subscribe to unauthorized events (session-ending backend codes). */
export const onUnauthorized = (listener: UnauthorizedListener): (() => void) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}

/**
 * Notify subscribers that the backend rejected the session. Used by the central
 * staffApi wrapper when it detects AUTH_REQUIRED / AUTH_SESSION_EXPIRED. Clears
 * the in-memory session before notifying.
 */
export const notifyUnauthorized = (reason: UnauthorizedReason): void => {
    clearSession()
    for (const listener of listeners) {
        try {
            listener(reason)
        } catch {
            // A listener throwing must not break other listeners or the call site.
        }
    }
}

// ── Test-only helpers (gated by NODE_ENV so production never touches them) ───

/**
 * Reset all session state. Test-only: vitest sets NODE_ENV=test. A no-op in
 * production to make accidental import harmless.
 */
export const __resetSessionForTests = (): void => {
    if (process.env.NODE_ENV !== "test") return
    session = null
    listeners.clear()
}
