import { useCallback, useEffect, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { BackupSettings, SnapshotInfo } from "../../types/staff"
import { getErrorMessage } from "../../lib/utils"

type UseSettingsStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    reloadToken: number
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
}

export type SettingsState = ReturnType<typeof useSettingsState>

export function useSettingsState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
    triggerReload,
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

    // Load backup settings + current DB custom path
    useEffect(() => {
        if (!dbReady || !isAuthenticated) return

        let disposed = false

        void (async () => {
            try {
                const [settings, customPath] = await Promise.all([
                    staffApi.getBackupSettings(),
                    staffApi.getDbCustomPath(),
                ])
                if (!disposed) {
                    setBackupSettings(settings)
                    setBackupDirectoryInput(settings?.backupDirectoryPath ?? "")
                    setBackupAutoEnabled(settings?.autoBackupEnabled ?? false)
                    setDbCustomPathInput(customPath ?? "")
                }
            } catch (error) {
                if (!disposed) setGlobalError(getErrorMessage(error))
            }
        })()

        return () => { disposed = true }
    }, [dbReady, isAuthenticated, reloadToken, setGlobalError])

    // Load history snapshots
    const loadSnapshots = useCallback(async () => {
        if (!dbReady) return
        try {
            setLoadingSnapshots(true)
            const list = await staffApi.listHistorySnapshots()
            setSnapshots(list)
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setLoadingSnapshots(false)
        }
    }, [dbReady, setGlobalError])

    useEffect(() => {
        if (!dbReady || !isAuthenticated) return
        void loadSnapshots()
    }, [dbReady, isAuthenticated, reloadToken, loadSnapshots])

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
            setGlobalError(getErrorMessage(error))
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
            setGlobalError(getErrorMessage(error))
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
            setGlobalError(getErrorMessage(error))
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
            setSnapshotMessage("✅ Restore complete. Reloading data...")
            await loadSnapshots()
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
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
        } catch (error) {
            setDbPathMessage(`❌ ${getErrorMessage(error)}`)
            setDbMovePending(false)
        } finally {
            setMovingDb(false)
        }
    }

    const handleMoveDatabaseCancel = () => {
        setDbMovePending(false)
        setDbPathMessage("")
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
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setResettingData(false)
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
    }
}
