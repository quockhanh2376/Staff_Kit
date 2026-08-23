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
    it("persists a newly detected LAN IP before building the QR URL", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) =>
            command === "get_borrow_lan_settings"
                ? { enabled: true, host: "127.0.0.1", port: 8787, borrowUrl: "http://127.0.0.1:8787/borrow" }
                : command === "get_borrow_lan_status"
                    ? { running: false, tokenReady: false, bindHost: "127.0.0.1", port: 8787 }
                    : command === "detect_borrow_lan_host"
                        ? "192.168.2.1"
                        : command === "update_borrow_lan_settings"
                            ? { enabled: true, host: "192.168.2.1", port: 8787, borrowUrl: "http://192.168.2.1:8787/borrow" }
                            : command === "start_borrow_lan_server"
                                ? { running: true, tokenReady: false, bindHost: "192.168.2.1", port: 8787 }
                                : command === "issue_borrow_lan_token"
                                    ? "session-token"
                                    : undefined,
        )
        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
            await result.current.ensureBorrowLanReady()
        })

        expect(result.current.borrowLanHostInput).toBe("192.168.2.1")
        expect(result.current.borrowLanDetectedHost).toBe("192.168.2.1")
        expect(result.current.borrowLanSettings?.host).toBe("192.168.2.1")
        expect(result.current.borrowLanQrUrl).toBe("http://192.168.2.1:8787/borrow#t=session-token")
        expect(invokeMock.mock.calls.filter(([command]) => command === "update_borrow_lan_settings")).toHaveLength(1)
    })

    it("keeps the saved LAN host when detection returns no address", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) =>
            command === "get_borrow_lan_settings"
                ? { enabled: true, host: "192.168.2.1", port: 8787, borrowUrl: "http://192.168.2.1:8787/borrow" }
                : command === "detect_borrow_lan_host"
                    ? null
                    : undefined,
        )

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        expect(result.current.borrowLanSettings?.host).toBe("192.168.2.1")
        expect(result.current.borrowLanHostInput).toBe("192.168.2.1")
        expect(invokeMock.mock.calls.some(([command]) => command === "update_borrow_lan_settings")).toBe(false)
    })

    it("validates the port before saving LAN settings", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        await act(async () => { result.current.handleBorrowLanHostInputChange("127.0.0.1") })
        await act(async () => { result.current.setBorrowLanPortInput("70000") })
        expect(result.current.borrowLanPortInput).toBe("70000")

        await act(async () => {
            await result.current.handleSaveBorrowLanSettings()
        })

        expect(result.current.borrowLanMessage).toBe("LAN port must be between 1 and 65535.")
        expect(invokeMock.mock.calls.some(([command]) => command === "update_borrow_lan_settings")).toBe(false)
    })

    it("clears the persistent stop message and leaves the inactive status visible", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })

        invokeMock.mockImplementation(async (command: string) => {
            if (command === "stop_borrow_lan_server") {
                return { running: false, tokenReady: false, bindHost: "127.0.0.1", port: 8787 }
            }
            return undefined
        })

        await act(async () => {
            await result.current.handleStopBorrowLanServer()
        })

        expect(result.current.lanServerAlive).toBe(false)
        expect(result.current.lanTokenReady).toBe(false)
        expect(result.current.borrowLanMessage).toBe("")
    })

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

    it("keeps the QR payload on saved LAN settings while Host and Port edits remain unsaved", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_borrow_lan_settings") return { host: "192.168.2.1", port: 8787 }
            if (command === "get_borrow_lan_token_status") return { ready: false }
            if (command === "get_borrow_lan_status") return { running: false, tokenReady: false, bindHost: "192.168.2.1", port: 8787 }
            if (command === "start_borrow_lan_server") return { running: true, tokenReady: false, bindHost: "192.168.2.1", port: 8787 }
            if (command === "issue_borrow_lan_token") return "session-token"
            if (command === "update_borrow_lan_settings") return {
                enabled: true,
                host: "192.168.2.99",
                port: 9999,
                borrowUrl: "http://192.168.2.99:9999/borrow",
            }
            return undefined
        })

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
            await result.current.ensureBorrowLanReady()
        })

        expect(result.current.borrowLanQrUrl).toBe("http://192.168.2.1:8787/borrow#t=session-token")
        await act(async () => {
            result.current.handleBorrowLanHostInputChange("192.168.2.99")
            result.current.setBorrowLanPortInput("9999")
        })

        expect(result.current.borrowLanHostInput).toBe("192.168.2.99")
        expect(result.current.borrowLanPortInput).toBe("9999")
        expect(result.current.borrowLanQrUrl).toBe("http://192.168.2.1:8787/borrow#t=session-token")

        await act(async () => {
            await result.current.handleSaveBorrowLanSettings()
        })

        expect(result.current.borrowLanSettings?.host).toBe("192.168.2.99")
        expect(result.current.borrowLanQrUrl).toBe("")
        expect(result.current.borrowLanRestartRequired).toBe(true)
        expect(result.current.borrowLanMessage).toBe("LAN settings saved. Restart Borrow / Return to apply.")
    })

    it("restarts on the saved LAN settings and issues a fresh QR token", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_borrow_lan_settings") return { enabled: true, host: "192.168.2.1", port: 8787, borrowUrl: "http://192.168.2.1:8787/borrow" }
            if (command === "get_borrow_lan_token_status") return { ready: false }
            if (command === "get_borrow_lan_status") return { running: true, tokenReady: false, bindHost: "192.168.2.1", port: 8787 }
            if (command === "update_borrow_lan_settings") return { enabled: true, host: "192.168.2.99", port: 9999, borrowUrl: "http://192.168.2.99:9999/borrow" }
            if (command === "start_borrow_lan_server") return { running: true, tokenReady: false, bindHost: "192.168.2.99", port: 9999 }
            if (command === "stop_borrow_lan_server") return { running: false, tokenReady: false, bindHost: "192.168.2.99", port: 9999 }
            if (command === "issue_borrow_lan_token") return "fresh-session-token"
            return undefined
        })

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        await act(async () => {
            result.current.handleBorrowLanHostInputChange("192.168.2.99")
            result.current.setBorrowLanPortInput("9999")
        })
        await act(async () => {
            await result.current.handleSaveBorrowLanSettings()
        })

        expect(result.current.borrowLanRestartRequired).toBe(true)
        expect(result.current.borrowLanQrUrl).toBe("")

        await act(async () => {
            await result.current.handleRestartBorrowLanServer()
        })

        expect(result.current.lanServerStatus).toMatchObject({
            running: true,
            bindHost: "192.168.2.99",
            port: 9999,
            tokenReady: true,
        })
        expect(result.current.borrowLanRestartRequired).toBe(false)
        expect(result.current.borrowLanQrUrl).toBe("http://192.168.2.99:9999/borrow#t=fresh-session-token")
        expect(result.current.borrowLanQrUrl).not.toContain("192.168.2.1")
    })

    it("does not retain a QR token across Stop and Start", async () => {
        setSession({ sessionToken: "lan-token", expiresAt: "2099-01-01T00:00:00Z" })
        let tokenCount = 0
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_borrow_lan_settings") return { enabled: true, host: "127.0.0.1", port: 8787, borrowUrl: "http://127.0.0.1:8787/borrow" }
            if (command === "get_borrow_lan_token_status") return { ready: false }
            if (command === "get_borrow_lan_status") return { running: false, tokenReady: false, bindHost: "127.0.0.1", port: 8787 }
            if (command === "start_borrow_lan_server") return { running: true, tokenReady: false, bindHost: "127.0.0.1", port: 8787 }
            if (command === "stop_borrow_lan_server") return { running: false, tokenReady: false, bindHost: "127.0.0.1", port: 8787 }
            if (command === "issue_borrow_lan_token") {
                tokenCount += 1
                return tokenCount > 1 ? "second-token" : "first-token"
            }
            return undefined
        })

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
            await result.current.ensureBorrowLanReady()
        })
        expect(result.current.borrowLanQrUrl).toBe("http://127.0.0.1:8787/borrow#t=first-token")

        await act(async () => {
            await result.current.handleStopBorrowLanServer()
        })
        expect(result.current.borrowLanQrUrl).toBe("")

        await act(async () => {
            await result.current.ensureBorrowLanReady()
        })
        expect(result.current.borrowLanQrUrl).toBe("http://127.0.0.1:8787/borrow#t=second-token")
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

describe("Borrow / Return saved Handle with Care policy", () => {
    const savedPolicy = {
        version: 7,
        textEn: "Saved English policy.",
        textVi: "Chính sách đã lưu.",
        createdAt: "2026-08-10T00:00:00Z",
    }

    async function settlePolicyLoad() {
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
    }

    it("keeps unsaved textarea edits local without changing the saved current policy", async () => {
        setSession({ sessionToken: "policy-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_borrow_policy") return savedPolicy
            return undefined
        })

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await settlePolicyLoad()

        await act(async () => {
            result.current.setBorrowPolicyEnglishInput("Unsaved English draft.")
            result.current.setBorrowPolicyVietnameseInput("Bản nháp chưa lưu.")
        })

        expect(result.current.borrowPolicy).toEqual(savedPolicy)
        expect(result.current.borrowPolicy?.textEn).toBe(savedPolicy.textEn)
        expect(result.current.borrowPolicy?.textVi).toBe(savedPolicy.textVi)
        expect(result.current.borrowPolicyEnglishInput).toBe("Unsaved English draft.")
        expect(result.current.borrowPolicyVietnameseInput).toBe("Bản nháp chưa lưu.")
    })

    it("updates the saved current policy only after a successful save", async () => {
        setSession({ sessionToken: "policy-token", expiresAt: "2099-01-01T00:00:00Z" })
        const updatedPolicy = { ...savedPolicy, version: 8, textEn: "Updated English policy.", textVi: "Chính sách mới." }
        invokeMock.mockImplementation(async (command: string, args?: { payload?: { textEn?: string; textVi?: string } }) => {
            if (command === "get_borrow_policy") return savedPolicy
            if (command === "save_borrow_policy") return { ...updatedPolicy, textEn: args?.payload?.textEn, textVi: args?.payload?.textVi }
            return undefined
        })

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await settlePolicyLoad()
        await act(async () => {
            result.current.setBorrowPolicyEnglishInput(updatedPolicy.textEn)
            result.current.setBorrowPolicyVietnameseInput(updatedPolicy.textVi)
        })

        expect(result.current.borrowPolicy).toEqual(savedPolicy)
        await act(async () => {
            await result.current.saveBorrowPolicy()
        })

        expect(result.current.borrowPolicy?.version).toBe(8)
        expect(result.current.borrowPolicy?.textEn).toBe(updatedPolicy.textEn)
        expect(result.current.borrowPolicy?.textVi).toBe(updatedPolicy.textVi)
        expect(result.current.borrowPolicyEnglishInput).toBe(updatedPolicy.textEn)
        expect(result.current.borrowPolicyVietnameseInput).toBe(updatedPolicy.textVi)
    })

    it("keeps the draft and saved policy unchanged when Save fails", async () => {
        setSession({ sessionToken: "policy-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_borrow_policy") return savedPolicy
            if (command === "save_borrow_policy") throw new Error("temporary save failure")
            return undefined
        })

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await settlePolicyLoad()
        await act(async () => {
            result.current.setBorrowPolicyEnglishInput("Failed English draft.")
            result.current.setBorrowPolicyVietnameseInput("Bản nháp lỗi.")
        })
        await act(async () => {
            await result.current.saveBorrowPolicy()
        })

        expect(result.current.borrowPolicy).toEqual(savedPolicy)
        expect(result.current.borrowPolicyEnglishInput).toBe("Failed English draft.")
        expect(result.current.borrowPolicyVietnameseInput).toBe("Bản nháp lỗi.")
        expect(result.current.borrowPolicyMessage).not.toBe("Saved")
    })

    it("does not invoke Save when the draft is unchanged", async () => {
        setSession({ sessionToken: "policy-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_borrow_policy") return savedPolicy
            return undefined
        })

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await settlePolicyLoad()
        await act(async () => {
            await result.current.saveBorrowPolicy()
        })

        expect(invokeMock.mock.calls.filter(([command]) => command === "save_borrow_policy")).toHaveLength(0)
        expect(result.current.borrowPolicy).toEqual(savedPolicy)
    })

    it("does not overwrite an active draft on a plain rerender", async () => {
        setSession({ sessionToken: "policy-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_borrow_policy") return savedPolicy
            return undefined
        })

        const { result, rerender } = renderHook(({ reloadToken }: { reloadToken: number }) =>
            useSettingsState({ ...baseOptions, reloadToken }),
            { initialProps: { reloadToken: 0 } },
        )
        await settlePolicyLoad()
        await act(async () => {
            result.current.setBorrowPolicyEnglishInput("Active draft.")
        })
        rerender({ reloadToken: 0 })

        expect(result.current.borrowPolicyEnglishInput).toBe("Active draft.")
        expect(result.current.borrowPolicy?.textEn).toBe(savedPolicy.textEn)
    })

    it("reloads the saved policy over an unsaved draft", async () => {
        setSession({ sessionToken: "policy-token", expiresAt: "2099-01-01T00:00:00Z" })
        invokeMock.mockImplementation(async (command: string) => {
            if (command === "get_borrow_policy") return savedPolicy
            return undefined
        })

        const { result } = renderHook(() => useSettingsState(baseOptions))
        await settlePolicyLoad()
        await act(async () => {
            result.current.setBorrowPolicyEnglishInput("Draft that must be discarded.")
        })
        expect(result.current.borrowPolicyEnglishInput).toBe("Draft that must be discarded.")

        await act(async () => {
            await result.current.loadBorrowPolicy()
        })

        expect(result.current.borrowPolicy).toEqual(savedPolicy)
        expect(result.current.borrowPolicyEnglishInput).toBe(savedPolicy.textEn)
        expect(result.current.borrowPolicyVietnameseInput).toBe(savedPolicy.textVi)
    })
})
