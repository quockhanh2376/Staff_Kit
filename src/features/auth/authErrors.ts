/**
 * Stable, machine-readable auth error codes emitted by the Rust backend
 * (`src-tauri/src/auth_session.rs`). The frontend matches these centrally to
 * decide session/UX behavior. Never log or display raw backend error details.
 *
 * @see src-tauri/src/auth_session.rs (AUTH_REQUIRED / AUTH_SESSION_EXPIRED / AUTH_FORBIDDEN)
 */
export const AUTH_REQUIRED = "AUTH_REQUIRED" as const
export const AUTH_SESSION_EXPIRED = "AUTH_SESSION_EXPIRED" as const
export const AUTH_FORBIDDEN = "AUTH_FORBIDDEN" as const

/// The actor attempted to delete the account they are currently signed in with.
/// The session is left intact; the action is simply rejected.
export const AUTH_CANNOT_DELETE_SELF = "AUTH_CANNOT_DELETE_SELF" as const

/// A stable error code that may also carry a domain-specific auth/business rule.
export type AuthErrorCode =
    | typeof AUTH_REQUIRED
    | typeof AUTH_SESSION_EXPIRED
    | typeof AUTH_FORBIDDEN
    | typeof AUTH_CANNOT_DELETE_SELF

/// True if the code is the self-delete rejection. Preserves the session.
export const isCannotDeleteSelf = (code: string | null | undefined): boolean =>
    code === AUTH_CANNOT_DELETE_SELF

/** A session-ending code: clear the session and return to login. */
export const isSessionEnding = (code: string | null | undefined): boolean =>
    code === AUTH_REQUIRED || code === AUTH_SESSION_EXPIRED

/** An authorization-denied code: keep the session, surface a user-facing error. */
export const isForbidden = (code: string | null | undefined): boolean => code === AUTH_FORBIDDEN

/** Any recognized auth/business-rule error code. */
const isAuthErrorCode = (code: string | null | undefined): code is AuthErrorCode =>
    isSessionEnding(code) || isForbidden(code) || isCannotDeleteSelf(code)

/**
 * Detect an auth error code in a thrown value. Backend auth failures arrive as
 * `invoke` rejections whose message is exactly the stable code (see
 * `AuthError`'s `Display` impl). Unknown errors do not match.
 */
export const detectAuthErrorCode = (error: unknown): AuthErrorCode | null => {
    const candidate = typeof error === "string" ? error : error instanceof Error ? error.message : null
    return isAuthErrorCode(candidate) ? candidate : null
}

/**
 * A user-facing message for a recognized auth/business-rule code. Used to surface
 * clear guidance without exposing raw backend details.
 */
export const authErrorCodeMessage = (code: AuthErrorCode): string => {
    switch (code) {
        case AUTH_CANNOT_DELETE_SELF:
            return "You cannot delete the account you are currently signed in with."
        case AUTH_FORBIDDEN:
            return "You do not have permission to perform this action."
        case AUTH_REQUIRED:
        case AUTH_SESSION_EXPIRED:
            return "Your session has ended. Please sign in again."
    }
}
