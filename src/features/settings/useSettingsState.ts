import { useCallback, useEffect, useState } from "react"
import { open as openFileDialog } from "@tauri-apps/plugin-dialog"
import { staffApi } from "../../services/staff-api"
import type { AssetRecord, AssetSeedItemInput, BackupSettings, BorrowLanSettings, SnapshotInfo } from "../../types/staff"
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

    // Borrow LAN settings
    const [borrowLanSettings, setBorrowLanSettings] = useState<BorrowLanSettings | null>(null)
    const [borrowLanHostInput, setBorrowLanHostInput] = useState("")
    const [borrowLanPortInput, setBorrowLanPortInput] = useState("8787")
    const [borrowLanMessage, setBorrowLanMessage] = useState("")
    const [isSavingBorrowLanSettings, setSavingBorrowLanSettings] = useState(false)

    // Asset seed utility
    const [assetSeedText, setAssetSeedText] = useState("")
    const [seededAssets, setSeededAssets] = useState<AssetRecord[]>([])
    const [assetSeedMessage, setAssetSeedMessage] = useState("")
    const [isSeedingAssets, setSeedingAssets] = useState(false)

    // Load backup settings + current DB custom path
    useEffect(() => {
        if (!dbReady || !isAuthenticated) return

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
                    setDbCustomPathInput(customPathValue ?? "")
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
            setSnapshotMessage("✅ Restore complete. Reloading data...")
            await loadSnapshots()
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
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
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
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
            setBorrowLanMessage("Borrow LAN settings saved.")
        } catch (error) {
            setBorrowLanMessage(getErrorMessage(error))
        } finally {
            setSavingBorrowLanSettings(false)
        }
    }

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
            setAssetSeedMessage(getErrorMessage(error))
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
        setBorrowLanHostInput,
        borrowLanPortInput,
        setBorrowLanPortInput,
        borrowLanMessage,
        isSavingBorrowLanSettings,
        handleSaveBorrowLanSettings,
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
