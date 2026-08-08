/**
 * SEC-001 Phase D3 — post-success session handling for super_admin database
 * operations that invalidate ALL backend sessions (reset_all_data, restore
 * snapshot, restore from file, move database).
 *
 * Each success path must proactively clear the in-memory session, surface the
 * re-login message, and reset app state — rather than letting the NEXT IPC call
 * discover the dead session. Failures must preserve the session.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { act, renderHook } from "@testing-library/react"

// Mock the Tauri invoke surface BEFORE importing the hook/staffApi.
const invokeMock = vi.fn()
vi.mock("@tauri-apps/api/core", () => ({
    invoke: (...args: unknown[]) => invokeMock(...args),
}))

// Mock the dialog plugin (used by handleRestoreFromFile).
const openFileDialogMock = vi.fn()
vi.mock("@tauri-apps/plugin-dialog", () => ({
    open: (...args: unknown[]) => openFileDialogMock(...args),
}))

// Stub the Tauri runtime globals so ensureTauriRuntime() passes.
beforeEach(() => {
    Object.defineProperty(window, "__TAURI__", {
        value: { core: { invoke: invokeMock } },
        configurable: true,
    })
})

import { useSettingsState } from "./useSettingsState"
import {
    getSessionToken,
    setSession,
    __resetSessionForTests,
} from "../../services/session"

const noop = () => {}

const baseOptions = {
    dbReady: true,
    isAuthenticated: true,
    isAdminAccount: true,
    reloadToken: 0,
    setGlobalError: noop,
    triggerReload: noop,
    onLogout: noop,
}

beforeEach(() => {
    __resetSessionForTests()
    invokeMock.mockReset()
    // Defaults used by the settings bootstrap effect. Returned in order:
    //   getBackupSettings, getDbCustomPath, getBorrowLanSettings
    invokeMock.mockImplementation(async (command: string) => {
        switch (command) {
            case "get_backup_settings":
                return { backupDirectoryPath: "C:/backups", autoBackupEnabled: false }
            case "get_db_custom_path":
                return null
            case "get_borrow_lan_settings":
                return { host: "127.0.0.1", port: 8787 }
            case "get_borrow_lan_token_status":
                return { ready: false }
            case "get_borrow_lan_status":
                return { running: false, tokenReady: false, bindHost: "127.0.0.1", port: 8787 }
            case "list_history_snapshots":
                return []
            default:
                return undefined
        }
    })
})

afterEach(() => {
    __resetSessionForTests()
})

describe("reset_all_data — clears ALL sessions on success (Phase D3)", () => {
    it("clears the in-memory session, surfaces the message, and resets app state on success", async () => {
        const onLogout = vi.fn()
        const setGlobalError = vi.fn()

        setSession({ sessionToken: "super-token", expiresAt: "2099-01-01T00:00:00Z" })
        const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true)

        const { result } = renderHook(() =>
            useSettingsState({ ...baseOptions, onLogout, setGlobalError }),
        )
        // Let the bootstrap effect settle.
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        expect(getSessionToken()).toBe("super-token")

        invokeMock.mockResolvedValueOnce(true)

        await act(async () => {
            await result.current.handleResetAllData()
        })

        confirmSpy.mockRestore()

        expect(getSessionToken()).toBeNull()
        expect(setGlobalError).toHaveBeenCalledWith(
            "Database was reset/restored. Please sign in again.",
        )
        expect(onLogout).toHaveBeenCalledTimes(1)
    })

    it("preserves the session when the user cancels the confirmation", async () => {
        const onLogout = vi.fn()
        const setGlobalError = vi.fn()

        setSession({ sessionToken: "super-token", expiresAt: "2099-01-01T00:00:00Z" })
        const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false)

        const { result } = renderHook(() =>
            useSettingsState({ ...baseOptions, onLogout, setGlobalError }),
        )
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        await act(async () => {
            await result.current.handleResetAllData()
        })

        confirmSpy.mockRestore()

        expect(getSessionToken()).toBe("super-token")
        expect(onLogout).not.toHaveBeenCalled()
        // reset_all_data must NOT have been invoked.
        expect(
            invokeMock.mock.calls.some(([cmd]) => cmd === "reset_all_data"),
        ).toBe(false)
    })

    it("preserves the session when the backend rejects", async () => {
        const onLogout = vi.fn()
        const setGlobalError = vi.fn()

        setSession({ sessionToken: "super-token", expiresAt: "2099-01-01T00:00:00Z" })
        const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true)

        const { result } = renderHook(() =>
            useSettingsState({ ...baseOptions, onLogout, setGlobalError }),
        )
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        invokeMock.mockRejectedValueOnce(new Error("AUTH_FORBIDDEN"))

        await act(async () => {
            await result.current.handleResetAllData()
        })

        confirmSpy.mockRestore()

        // FORBIDDEN is not session-ending at the wrapper either, but in any case
        // the success-path clear must NOT run on failure.
        expect(getSessionToken()).toBe("super-token")
        expect(onLogout).not.toHaveBeenCalled()
    })
})

describe("restore_history_snapshot — clears ALL sessions on success (Phase D3)", () => {
    it("clears the in-memory session, surfaces the message, and resets app state", async () => {
        const onLogout = vi.fn()
        const setGlobalError = vi.fn()

        setSession({ sessionToken: "super-token", expiresAt: "2099-01-01T00:00:00Z" })
        const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true)

        const { result } = renderHook(() =>
            useSettingsState({ ...baseOptions, onLogout, setGlobalError }),
        )
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        invokeMock.mockResolvedValueOnce(undefined) // restore_history_snapshot

        await act(async () => {
            await result.current.handleRestoreSnapshot("snap-001.sqlite3")
        })

        confirmSpy.mockRestore()

        expect(getSessionToken()).toBeNull()
        expect(setGlobalError).toHaveBeenCalledWith(
            "Database was reset/restored. Please sign in again.",
        )
        expect(onLogout).toHaveBeenCalledTimes(1)
    })
})

describe("restore_database_from_file — clears ALL sessions on success (Phase D3)", () => {
    it("clears the in-memory session, surfaces the message, and resets app state", async () => {
        const onLogout = vi.fn()
        const setGlobalError = vi.fn()

        setSession({ sessionToken: "super-token", expiresAt: "2099-01-01T00:00:00Z" })
        openFileDialogMock.mockResolvedValue("C:/backups/old.sqlite3")
        const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true)

        const { result } = renderHook(() =>
            useSettingsState({ ...baseOptions, onLogout, setGlobalError }),
        )
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        invokeMock.mockResolvedValueOnce(undefined) // restore_database_from_file

        await act(async () => {
            await result.current.handleRestoreFromFile()
        })

        confirmSpy.mockRestore()

        expect(getSessionToken()).toBeNull()
        expect(setGlobalError).toHaveBeenCalledWith(
            "Database was reset/restored. Please sign in again.",
        )
        expect(onLogout).toHaveBeenCalledTimes(1)
    })
})

describe("move_database_to — clears ALL sessions on success (Phase D3)", () => {
    it("clears the in-memory session, surfaces the message, and resets app state", async () => {
        const onLogout = vi.fn()
        const setGlobalError = vi.fn()

        setSession({ sessionToken: "super-token", expiresAt: "2099-01-01T00:00:00Z" })

        const { result } = renderHook(() =>
            useSettingsState({ ...baseOptions, onLogout, setGlobalError }),
        )
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        // Step 1: seed the path, then prime the inline confirmation (no IPC).
        await act(async () => {
            result.current.setDbCustomPathInput("C:/new-location")
        })
        await act(async () => {
            await result.current.handleMoveDatabase()
        })
        expect(result.current.dbMovePending).toBe(true)

        // Step 2: the second submit performs the move (mock the success result).
        invokeMock.mockResolvedValueOnce("COPIED:C:/new-location/staff.sqlite3")
        await act(async () => {
            await result.current.handleMoveDatabase()
        })

        expect(getSessionToken()).toBeNull()
        expect(setGlobalError).toHaveBeenCalledWith(
            "Database was reset/restored. Please sign in again.",
        )
        expect(onLogout).toHaveBeenCalledTimes(1)
    })

    it("preserves the session when the move fails", async () => {
        const onLogout = vi.fn()
        const setGlobalError = vi.fn()

        setSession({ sessionToken: "super-token", expiresAt: "2099-01-01T00:00:00Z" })

        const { result } = renderHook(() =>
            useSettingsState({ ...baseOptions, onLogout, setGlobalError }),
        )
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        await act(async () => {
            result.current.setDbCustomPathInput("C:/new-location")
        })
        await act(async () => {
            await result.current.handleMoveDatabase()
        })

        invokeMock.mockRejectedValueOnce(new Error("disk full"))
        await act(async () => {
            await result.current.handleMoveDatabase()
        })

        expect(getSessionToken()).toBe("super-token")
        expect(onLogout).not.toHaveBeenCalled()
    })
})

describe("standard-user startup avoids admin commands (Regression B)", () => {
    it("standard user (isAdminAccount=false) invokes no admin settings/snapshot commands", async () => {
        setSession({ sessionToken: "user-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockResolvedValue([])

        renderHook(() =>
            useSettingsState({
                ...baseOptions,
                isAdminAccount: false,
                isAuthenticated: true,
            }),
        )
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        const adminCommands = [
            "get_backup_settings",
            "get_db_custom_path",
            "get_borrow_lan_settings",
            "list_history_snapshots",
        ]
        for (const cmd of adminCommands) {
            const calls = invokeMock.mock.calls.filter(([c]) => c === cmd)
            expect(calls).toHaveLength(0)
        }
    })

    it("standard user startup produces no global error", async () => {
        const setGlobalError = vi.fn()
        setSession({ sessionToken: "user-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockResolvedValue([])

        renderHook(() =>
            useSettingsState({
                ...baseOptions,
                isAdminAccount: false,
                isAuthenticated: true,
                setGlobalError,
            }),
        )
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        expect(setGlobalError).not.toHaveBeenCalled()
    })
})

describe("Borrow / Return LAN automatic lifecycle", () => {
    it("starts a stopped server once and issues one token", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        const { result, rerender } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        invokeMock.mockImplementation(async (command: string) => {
            if (command === "start_borrow_lan_server") {
                return { running: true, tokenReady: false, bindHost: "127.0.0.1", port: 8787 }
            }
            if (command === "issue_borrow_lan_token") return "session-token"
            return undefined
        })

        await act(async () => {
            await result.current.ensureBorrowLanReady()
            await result.current.ensureBorrowLanReady()
        })
        rerender()
        await act(async () => {
            await result.current.ensureBorrowLanReady()
        })

        expect(invokeMock.mock.calls.filter(([command]) => command === "start_borrow_lan_server")).toHaveLength(1)
        expect(invokeMock.mock.calls.filter(([command]) => command === "issue_borrow_lan_token")).toHaveLength(1)
        expect(result.current.lanAutoStartState).toBe("ready")
        expect(result.current.borrowLanQrUrl).toBe("http://127.0.0.1:8787/borrow#t=session-token")
    })

    it("does not restart or regenerate when the running session is already token-ready", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_backup_settings") return { backupDirectoryPath: "C:/backups", autoBackupEnabled: false }
            if (command === "get_db_custom_path") return null
            if (command === "get_borrow_lan_settings") return { host: "127.0.0.1", port: 8787 }
            if (command === "get_borrow_lan_token_status") return { ready: true }
            if (command === "get_borrow_lan_status") return { running: true, tokenReady: true, bindHost: "127.0.0.1", port: 8787 }
            return undefined
        })
        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
            await result.current.ensureBorrowLanReady()
            await result.current.ensureBorrowLanReady()
        })

        expect(invokeMock.mock.calls.filter(([command]) => command === "start_borrow_lan_server")).toHaveLength(0)
        expect(invokeMock.mock.calls.filter(([command]) => command === "issue_borrow_lan_token")).toHaveLength(0)
        expect(result.current.lanAutoStartState).toBe("ready")
    })

    it("shows safe retry state after failure and deduplicates concurrent retry calls", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        let rejectStart: ((error: Error) => void) | null = null
        let startAttempts = 0
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_backup_settings") return { backupDirectoryPath: "C:/backups", autoBackupEnabled: false }
            if (command === "get_db_custom_path") return null
            if (command === "get_borrow_lan_settings") return { host: "127.0.0.1", port: 8787 }
            if (command === "get_borrow_lan_token_status") return { ready: false }
            if (command === "get_borrow_lan_status") return { running: false, tokenReady: false, bindHost: "127.0.0.1", port: 8787 }
            if (command === "start_borrow_lan_server") {
                startAttempts += 1
                if (startAttempts === 1) throw new Error("Address already in use at 127.0.0.1:8787")
                return new Promise((_, reject) => { rejectStart = reject })
            }
            return undefined
        })
        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
            await result.current.ensureBorrowLanReady()
        })
        expect(result.current.lanAutoStartState).toBe("error")
        expect(result.current.lanAutoStartError).toBe(
            "LAN server could not start on the configured port. Check the LAN settings or close the process using that port, then retry.",
        )

        let retryOne!: Promise<void>
        let retryTwo!: Promise<void>
        act(() => {
            retryOne = result.current.ensureBorrowLanReady()
            retryTwo = result.current.ensureBorrowLanReady()
        })
        expect(retryOne).toBe(retryTwo)
        expect(startAttempts).toBe(2)
        const failRetry = rejectStart as ((error: Error) => void) | null
        failRetry?.(new Error("second failure"))
        await act(async () => { await retryOne })
        expect(startAttempts).toBe(2)
    })
})
