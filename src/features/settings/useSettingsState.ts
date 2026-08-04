import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { open as openFileDialog } from "@tauri-apps/plugin-dialog"
import { staffApi } from "../../services/staff-api"
import { clearSession } from "../../services/session"
import type { AssetRecord, AssetSeedItemInput, BackupSettings, BorrowLanSettings, SnapshotInfo } from "../../types/staff"
import { getUserErrorMessage } from "../../lib/errorHandling"
import {
    applyDetectedBorrowLanSettings,
    buildBorrowLanUrlPreview,
    chooseBorrowLanHostInput,
} from "./borrowLanAutoFill"

type UseSettingsStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    isAdminAccount: boolean
    reloadToken: number
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
    /**
     * Invoked after a session-ending success (reset_all_data, restore, or
     * database move) to reset app-level state and return to the login screen.
     * The backend invalidates ALL sessions after these super_admin operations,
     * so the in-memory token is cleared proactively rather than waiting for the
     * next IPC call to fail with AUTH_SESSION_EXPIRED.
     */
    onLogout: () => void
}

export type SettingsState = ReturnType<typeof useSettingsState>

export function useSettingsState({
    dbReady,
    isAuthenticated,
    isAdminAccount,
    reloadToken,
    setGlobalError,
    triggerReload,
    onLogout,
}: UseSettingsStateOptions) {
    const [backupSettings, setBackupSettings] = useState<BackupSettings | null>(null)
    const [backupDirectoryInput, setBackupDirectoryInput] = useState("")
    const [backupAutoEnabled, setBackupAutoEnabled] = useState(false)
    const [isSavingBackupSettings, setSavingBackupSettings] = useState(false)
    const [isBackingUpData, setBackingUpData] = useState(false)
    const [backupStatusMessage, setBackupStatusMessage] = useState("")
    const [isResettingData, setResettingData] = useState(false)

    // History snapshots
    const [snapshots, setSnapshots] = useState<SnapshotInfo[]>([])
    const [isLoadingSnapshots, setLoadingSnapshots] = useState(false)
    const [isRestoringSnapshot, setRestoringSnapshot] = useState<string | null>(null)
    const [snapshotMessage, setSnapshotMessage] = useState("")

    // DB path
    const [dbCustomPathInput, setDbCustomPathInput] = useState("")
    const [isMovingDb, setMovingDb] = useState(false)
    const [dbPathMessage, setDbPathMessage] = useState("")
    const [dbMovePending, setDbMovePending] = useState(false)

    // Borrow LAN settings
    const [borrowLanSettings, setBorrowLanSettings] = useState<BorrowLanSettings | null>(null)
    const [borrowLanHostInput, setBorrowLanHostInput] = useState("")
    const [borrowLanPortInput, setBorrowLanPortInput] = useState("8787")
    const [borrowLanMessage, setBorrowLanMessage] = useState("")
    const [borrowLanDetectionNote, setBorrowLanDetectionNote] = useState("")
    const [isDetectingBorrowLanHost, setDetectingBorrowLanHost] = useState(false)
    const [isSavingBorrowLanSettings, setSavingBorrowLanSettings] = useState(false)
    const [lanServerAlive, setLanServerAlive] = useState<boolean | null>(null)
    const borrowLanHostTouchedRef = useRef(false)

    // Asset seed utility
    const [assetSeedText, setAssetSeedText] = useState("")
    const [seededAssets, setSeededAssets] = useState<AssetRecord[]>([])
    const [assetSeedMessage, setAssetSeedMessage] = useState("")
    const [isSeedingAssets, setSeedingAssets] = useState(false)

    const detectBorrowLanHost = useCallback(
        async ({
            savedHost,
            forceReplace = false,
            showMissingMessage = false,
        }: {
            savedHost: string
            forceReplace?: boolean
            showMissingMessage?: boolean
        }) => {
            try {
                setDetectingBorrowLanHost(true)
                if (showMissingMessage) {
                    setBorrowLanDetectionNote("")
                }

                const detectedHost = await staffApi.detectBorrowLanHost()
                const nextHost = chooseBorrowLanHostInput(savedHost, detectedHost)
                const normalizedSavedHost = savedHost.trim()

                if (!detectedHost?.trim()) {
                    if (showMissingMessage) {
                        setBorrowLanDetectionNote(
                            "Could not detect the current LAN IP on this machine. Enter it manually if needed.",
                        )
                    }
                    return null
                }

                if (forceReplace) {
                    borrowLanHostTouchedRef.current = false
                    setBorrowLanHostInput(nextHost)
                    setBorrowLanSettings((current) => applyDetectedBorrowLanSettings(current, nextHost))
                    setBorrowLanDetectionNote(
                        `Detected current LAN IP ${nextHost}. The Borrow URL and QR are updated for this session. Save to keep it for the next launch.`,
                    )
                    return nextHost
                }

                if (borrowLanHostTouchedRef.current) {
                    return nextHost
                }

                setBorrowLanHostInput(nextHost)
                setBorrowLanSettings((current) => applyDetectedBorrowLanSettings(current, nextHost))
                if (nextHost !== normalizedSavedHost) {
                    setBorrowLanDetectionNote(
                        `Auto-detected current LAN IP ${nextHost}. The Borrow URL and QR are updated for this session. Save to keep it for the next launch, or edit it manually if needed.`,
                    )
                }

                return nextHost
            } catch {
                if (showMissingMessage) {
                    setBorrowLanDetectionNote(
                        "Could not detect the current LAN IP on this machine. Enter it manually if needed.",
                    )
                } else {
                    setBorrowLanDetectionNote("")
                }
                return null
            } finally {
                setDetectingBorrowLanHost(false)
            }
        },
        [],
    )

    // Load backup settings + current DB custom path (admin-only commands).
    // Standard users must never trigger these admin-guarded calls.
    useEffect(() => {
        if (!dbReady || !isAuthenticated || !isAdminAccount) return

        let disposed = false

        void (async () => {
            try {
                const [backupSettings, customPathValue, lanSettings] = await Promise.all([
                    staffApi.getBackupSettings(),
                    staffApi.getDbCustomPath(),
                    staffApi.getBorrowLanSettings(),
                ])
                if (!disposed) {
                    setBackupSettings(backupSettings)
                    setBackupDirectoryInput(backupSettings?.backupDirectoryPath ?? "")
                    setBackupAutoEnabled(backupSettings?.autoBackupEnabled ?? false)
                    setBorrowLanSettings(lanSettings)
                    setBorrowLanHostInput(lanSettings.host)
                    setBorrowLanPortInput(String(lanSettings.port))
                    setBorrowLanDetectionNote("")
                    borrowLanHostTouchedRef.current = false
                    setDbCustomPathInput(customPathValue ?? "")
                }

                if (!disposed) {
                    void detectBorrowLanHost({
                        savedHost: lanSettings.host,
                    })
                }
            } catch (error) {
                if (!disposed) setGlobalError(getUserErrorMessage(error))
            }
        })()

        return () => { disposed = true }
    }, [dbReady, detectBorrowLanHost, isAuthenticated, isAdminAccount, reloadToken, setGlobalError])

    // Load history snapshots (admin-only command)
    const loadSnapshots = useCallback(async () => {
        if (!dbReady || !isAdminAccount) return
        try {
            setLoadingSnapshots(true)
            const list = await staffApi.listHistorySnapshots()
            setSnapshots(list)
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setLoadingSnapshots(false)
        }
    }, [dbReady, setGlobalError])

    useEffect(() => {
        if (!dbReady || !isAuthenticated || !isAdminAccount) return
        void loadSnapshots()
    }, [dbReady, isAuthenticated, isAdminAccount, reloadToken, loadSnapshots])

    const handleSaveBackupSettings = async () => {
        const nextPath = backupDirectoryInput.trim()
        if (!nextPath) {
            setGlobalError("Backup path is required.")
            return
        }
        try {
            setSavingBackupSettings(true)
            const settings = await staffApi.updateBackupSettings({
                backupDirectoryPath: nextPath,
                autoBackupEnabled: backupAutoEnabled,
            })
            setBackupSettings(settings)
            setBackupDirectoryInput(settings.backupDirectoryPath)
            setBackupAutoEnabled(settings.autoBackupEnabled)
            setBackupStatusMessage("Backup settings saved.")
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setSavingBackupSettings(false)
        }
    }

    const handleBackupNow = async () => {
        try {
            setBackingUpData(true)
            setBackupStatusMessage("")
            const result = await staffApi.backupDatabaseNow()
            setBackupStatusMessage(`Backup created: ${result.backupFilePath}`)
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setBackingUpData(false)
        }
    }

    const handleCreateSnapshot = async () => {
        try {
            setSnapshotMessage("")
            const snap = await staffApi.createHistorySnapshot("manual")
            setSnapshotMessage(`Snapshot saved: ${snap.timestamp}`)
            await loadSnapshots()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        }
    }

    const handleRestoreSnapshot = async (filename: string) => {
        const confirmed = window.confirm(
            `Restore snapshot "${filename}"?\n\nThe current database state will be saved as a new snapshot first, then restored. App data will reload after restore.`
        )
        if (!confirmed) return

        try {
            setRestoringSnapshot(filename)
            setSnapshotMessage("")
            await staffApi.restoreHistorySnapshot(filename)
            // SEC-001 Phase D3: restore invalidates ALL backend sessions. Clear the
            // in-memory token and return to login instead of waiting for the next
            // IPC call to fail with AUTH_SESSION_EXPIRED.
            clearSession()
            setSnapshotMessage("✅ Restore complete. Reloading data...")
            setGlobalError("Database was reset/restored. Please sign in again.")
            onLogout()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setRestoringSnapshot(null)
        }
    }

    const handleMoveDatabase = async () => {
        const folder = dbCustomPathInput.trim()
        if (!folder) {
            setDbPathMessage("Please enter a folder path.")
            return
        }
        // Step 1 — show inline confirmation
        if (!dbMovePending) {
            setDbMovePending(true)
            setDbPathMessage(`⚠️ Click "Confirm Move" to copy DB to: ${folder}`)
            return
        }
        // Step 2 — actually move/link
        try {
            setMovingDb(true)
            setDbPathMessage("")
            setDbMovePending(false)
            const result = await staffApi.moveDatabaseTo(folder)
            // SEC-001 Phase D3: moving the database invalidates ALL backend
            // sessions. Clear the in-memory token and return to login rather than
            // waiting for the next IPC call to fail.
            clearSession()
            if (result.startsWith("LINKED:")) {
                const path = result.slice("LINKED:".length)
                setDbPathMessage(
                    `✅ Linked to shared database at: ${path}\n` +
                    `Please restart the app to load the shared data.`
                )
            } else {
                const path = result.startsWith("COPIED:") ? result.slice("COPIED:".length) : result
                setDbPathMessage(
                    `✅ Database copied to: ${path}\n` +
                    `Restart app on all machines to switch to the new location.`
                )
            }
            setGlobalError("Database was reset/restored. Please sign in again.")
            onLogout()
        } catch (error) {
            setDbPathMessage(`❌ ${getUserErrorMessage(error)}`)
            setDbMovePending(false)
        } finally {
            setMovingDb(false)
        }
    }

    const handleMoveDatabaseCancel = () => {
        setDbMovePending(false)
        setDbPathMessage("")
    }

    const handleRestoreFromFile = async () => {
        try {
            const selected = await openFileDialog({
                title: "Select a Staff Kit backup database file",
                filters: [{ name: "SQLite Database", extensions: ["sqlite3", "sqlite", "db"] }],
                multiple: false,
            })
            if (!selected) return
            const filePath = typeof selected === "string" ? selected : (selected as { path: string }).path

            const confirmed = window.confirm(
                `Restore database from:\n${filePath}\n\nThe current database will be saved as a snapshot first. App will reload after restore.`
            )
            if (!confirmed) return

            setSnapshotMessage("")
            await staffApi.restoreDatabaseFromFile(filePath)
            // SEC-001 Phase D3: restore invalidates ALL backend sessions. Clear
            // the in-memory token and return to login instead of waiting for the
            // next IPC call to fail.
            clearSession()
            setSnapshotMessage("✅ Restore complete. Reloading data...")
            setGlobalError("Database was reset/restored. Please sign in again.")
            onLogout()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        }
    }

    const handleResetAllData = async () => {
        const firstConfirm = window.confirm(
            "This will delete ALL employees, teams, and imported columns. Are you sure?",
        )
        if (!firstConfirm) return
        const secondConfirm = window.confirm(
            "Final warning: this action is irreversible. Continue?",
        )
        if (!secondConfirm) return

        try {
            setResettingData(true)
            await staffApi.resetAllData()
            // SEC-001 Phase D3: reset_all_data invalidates ALL backend sessions.
            // Clear the in-memory token and return to login instead of waiting
            // for the next IPC call to fail with AUTH_SESSION_EXPIRED.
            clearSession()
            setGlobalError("Database was reset/restored. Please sign in again.")
            onLogout()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setResettingData(false)
        }
    }

    const handleSaveBorrowLanSettings = async () => {
        const host = borrowLanHostInput.trim()
        const port = Number.parseInt(borrowLanPortInput.trim(), 10)
        if (!host) {
            setBorrowLanMessage("LAN host is required.")
            return
        }
        if (!Number.isInteger(port) || port < 1 || port > 65535) {
            setBorrowLanMessage("LAN port must be between 1 and 65535.")
            return
        }

        try {
            setSavingBorrowLanSettings(true)
            setBorrowLanMessage("")
            const updated = await staffApi.updateBorrowLanSettings({ host, port })
            setBorrowLanSettings(updated)
            setBorrowLanHostInput(updated.host)
            setBorrowLanPortInput(String(updated.port))
            borrowLanHostTouchedRef.current = false
            setBorrowLanMessage("Borrow LAN settings saved.")
        } catch (error) {
            setBorrowLanMessage(getUserErrorMessage(error))
        } finally {
            setSavingBorrowLanSettings(false)
        }
    }

    const handleBorrowLanHostInputChange = useCallback((nextValue: string) => {
        borrowLanHostTouchedRef.current = true
        setBorrowLanHostInput(nextValue)
    }, [])

    const handleRefreshBorrowLanHost = useCallback(() => {
        setBorrowLanMessage("")
        void detectBorrowLanHost({
            savedHost: borrowLanHostInput,
            forceReplace: true,
            showMissingMessage: true,
        })
    }, [borrowLanHostInput, detectBorrowLanHost])

    const borrowLanUrlPreview = useMemo(
        () => buildBorrowLanUrlPreview(borrowLanHostInput, borrowLanPortInput),
        [borrowLanHostInput, borrowLanPortInput],
    )

    useEffect(() => {
        const port = borrowLanSettings?.port
        if (!port) {
            setLanServerAlive(null)
            return
        }
        let disposed = false
        setLanServerAlive(null)
        void staffApi
            .probeLanServer(port)
            .then((alive) => {
                if (!disposed) setLanServerAlive(alive)
            })
            .catch(() => {
                if (!disposed) setLanServerAlive(false)
            })
        return () => {
            disposed = true
        }
    }, [borrowLanSettings?.port])

    const handleSeedAssets = async () => {
        try {
            const items = assetSeedText
                .split(/\r?\n/)
                .map((line) => line.trim())
                .filter(Boolean)
                .map(parseAssetSeedLine)

            if (items.length === 0) {
                setAssetSeedMessage("Add at least one asset line before seeding.")
                return
            }

            setSeedingAssets(true)
            setAssetSeedMessage("")
            const result = await staffApi.upsertAssets(items)
            setSeededAssets(result)
            setAssetSeedMessage(`Seeded ${result.length} asset(s).`)
            triggerReload()
        } catch (error) {
            setAssetSeedMessage(getUserErrorMessage(error))
        } finally {
            setSeedingAssets(false)
        }
    }

    return {
        backupSettings,
        backupDirectoryInput,
        setBackupDirectoryInput,
        backupAutoEnabled,
        setBackupAutoEnabled,
        isSavingBackupSettings,
        isBackingUpData,
        backupStatusMessage,
        isResettingData,
        handleSaveBackupSettings,
        handleBackupNow,
        handleResetAllData,
        // History
        snapshots,
        isLoadingSnapshots,
        isRestoringSnapshot,
        snapshotMessage,
        handleCreateSnapshot,
        handleRestoreSnapshot,
        // DB path
        dbCustomPathInput,
        setDbCustomPathInput,
        isMovingDb,
        dbPathMessage,
        dbMovePending,
        handleMoveDatabase,
        handleMoveDatabaseCancel,
        handleRestoreFromFile,
        // Borrow LAN
        borrowLanSettings,
        borrowLanHostInput,
        handleBorrowLanHostInputChange,
        borrowLanPortInput,
        setBorrowLanPortInput,
        borrowLanUrlPreview,
        borrowLanMessage,
        borrowLanDetectionNote,
        isDetectingBorrowLanHost,
        isSavingBorrowLanSettings,
        handleRefreshBorrowLanHost,
        handleSaveBorrowLanSettings,
        lanServerAlive,
        // Asset seed
        assetSeedText,
        setAssetSeedText,
        seededAssets,
        assetSeedMessage,
        isSeedingAssets,
        handleSeedAssets,
    }
}

function parseAssetSeedLine(line: string): AssetSeedItemInput {
    const parts = line.split("|").map((part) => part.trim())
    if (parts.length < 3) {
        throw new Error("Each asset line must include assetCode|assetType|displayName.")
    }

    return {
        assetCode: parts[0],
        assetType: parts[1],
        displayName: parts[2],
        model: parts[3] || null,
        serialNumber: parts[4] || null,
        notes: parts[5] || null,
    }
}
