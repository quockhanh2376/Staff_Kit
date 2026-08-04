import { describe, expect, it } from "vitest"
import {
    AUTH_CANNOT_DELETE_SELF,
    AUTH_FORBIDDEN,
    AUTH_REQUIRED,
    AUTH_SESSION_EXPIRED,
    authErrorCodeMessage,
    detectAuthErrorCode,
    isCannotDeleteSelf,
    isForbidden,
    isSessionEnding,
} from "./authErrors"

describe("authErrors", () => {
    it("exposes four distinct stable codes", () => {
        expect(AUTH_REQUIRED).toBe("AUTH_REQUIRED")
        expect(AUTH_SESSION_EXPIRED).toBe("AUTH_SESSION_EXPIRED")
        expect(AUTH_FORBIDDEN).toBe("AUTH_FORBIDDEN")
        expect(AUTH_CANNOT_DELETE_SELF).toBe("AUTH_CANNOT_DELETE_SELF")
        expect(
            new Set([AUTH_REQUIRED, AUTH_SESSION_EXPIRED, AUTH_FORBIDDEN, AUTH_CANNOT_DELETE_SELF])
                .size,
        ).toBe(4)
    })

    it("isCannotDeleteSelf recognizes only AUTH_CANNOT_DELETE_SELF", () => {
        expect(isCannotDeleteSelf(AUTH_CANNOT_DELETE_SELF)).toBe(true)
        expect(isCannotDeleteSelf(AUTH_FORBIDDEN)).toBe(false)
        expect(isCannotDeleteSelf(AUTH_REQUIRED)).toBe(false)
        expect(isCannotDeleteSelf(null)).toBe(false)
    })

    it("detectAuthErrorCode matches AUTH_CANNOT_DELETE_SELF", () => {
        expect(detectAuthErrorCode(AUTH_CANNOT_DELETE_SELF)).toBe(AUTH_CANNOT_DELETE_SELF)
        expect(detectAuthErrorCode(new Error(AUTH_CANNOT_DELETE_SELF))).toBe(AUTH_CANNOT_DELETE_SELF)
    })

    it("authErrorCodeMessage surfaces the self-delete guidance", () => {
        expect(authErrorCodeMessage(AUTH_CANNOT_DELETE_SELF)).toContain(
            "cannot delete the account you are currently signed in with",
        )
    })

    it("isSessionEnding recognizes AUTH_REQUIRED and AUTH_SESSION_EXPIRED only", () => {
        expect(isSessionEnding(AUTH_REQUIRED)).toBe(true)
        expect(isSessionEnding(AUTH_SESSION_EXPIRED)).toBe(true)
        expect(isSessionEnding(AUTH_FORBIDDEN)).toBe(false)
        expect(isSessionEnding(null)).toBe(false)
        expect(isSessionEnding("something-else")).toBe(false)
    })

    it("isForbidden recognizes AUTH_FORBIDDEN only", () => {
        expect(isForbidden(AUTH_FORBIDDEN)).toBe(true)
        expect(isForbidden(AUTH_REQUIRED)).toBe(false)
        expect(isForbidden(AUTH_SESSION_EXPIRED)).toBe(false)
    })

    it("detectAuthErrorCode matches a string code", () => {
        expect(detectAuthErrorCode(AUTH_FORBIDDEN)).toBe(AUTH_FORBIDDEN)
        expect(detectAuthErrorCode("AUTH_REQUIRED")).toBe(AUTH_REQUIRED)
    })

    it("detectAuthErrorCode matches an Error whose message is a stable code", () => {
        expect(detectAuthErrorCode(new Error(AUTH_SESSION_EXPIRED))).toBe(AUTH_SESSION_EXPIRED)
        expect(detectAuthErrorCode(new Error("AUTH_FORBIDDEN"))).toBe(AUTH_FORBIDDEN)
    })

    it("detectAuthErrorCode returns null for unknown errors (no false match)", () => {
        expect(detectAuthErrorCode("incorrect username or password")).toBeNull()
        expect(detectAuthErrorCode(new Error("assetCode already exists"))).toBeNull()
        expect(detectAuthErrorCode({} as unknown)).toBeNull()
        expect(detectAuthErrorCode(undefined)).toBeNull()
    })
})
