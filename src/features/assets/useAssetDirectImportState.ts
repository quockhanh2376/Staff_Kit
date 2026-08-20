import { useCallback, useMemo, useState } from "react"
import { open as openFileDialog } from "@tauri-apps/plugin-dialog"
import { staffApi } from "../../services/staff-api"
import type {
    AssetCategoryRecord,
    AssetDirectImportPreview,
    AssetDirectImportReport,
    AssetImportFileInspection,
    AssetImportMode,
    AssetRecord,
    AssetSeedItemInput,
} from "../../types/staff"
import { getUserErrorMessage } from "../../lib/errorHandling"
import {
    buildManualSerializedAssetCreatedMessage,
    buildManualSerializedAssetRequiredMessage,
} from "./assetImportCopy"

type UseAssetDirectImportStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    reloadToken: number
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
}

type AssetImportPanelMode = "import" | "manual"
type ManualAssetForm = {
    assetCode: string
    assetType: string
    displayName: string
    model: string
    serialNumber: string
    notes: string
}

const DEFAULT_ASSET_IMPORT_MODE: AssetImportMode = "serialized"
const EMPTY_MANUAL_ASSET_FORM: ManualAssetForm = {
    assetCode: "",
    assetType: "",
    displayName: "",
    model: "",
    serialNumber: "",
    notes: "",
}

export type AssetImportState = ReturnType<typeof useAssetDirectImportState>

export function useAssetDirectImportState({
    dbReady,
    isAuthenticated,
    setGlobalError,
    triggerReload,
}: UseAssetDirectImportStateOptions) {
    const [isWizardOpen, setWizardOpen] = useState(false)
    const [panelMode, setPanelMode] = useState<AssetImportPanelMode>("import")
    const [selectedImportMode, setSelectedImportMode] =
        useState<AssetImportMode>(DEFAULT_ASSET_IMPORT_MODE)
    const [isLoadingCategories, setLoadingCategories] = useState(false)
    const [isInspectingFile, setInspectingFile] = useState(false)
    const [isPreviewingImport, setPreviewingImport] = useState(false)
    const [isApprovingImport, setApprovingImport] = useState(false)
    const [statusMessage, setStatusMessage] = useState("")
    const [assetCategories, setAssetCategories] = useState<AssetCategoryRecord[]>([])
    const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null)
    const [inspection, setInspection] = useState<AssetImportFileInspection | null>(null)
    const [preview, setPreview] = useState<AssetDirectImportPreview | null>(null)
    const [report, setReport] = useState<AssetDirectImportReport | null>(null)
    const [manualAssetForm, setManualAssetForm] =
        useState<ManualAssetForm>(EMPTY_MANUAL_ASSET_FORM)
    const [manualAssetResult, setManualAssetResult] = useState<AssetRecord | null>(null)
    const [manualAssetMessage, setManualAssetMessage] = useState("")
    const [isCreatingManualAsset, setCreatingManualAsset] = useState(false)

    const loadAssetCategories = useCallback(async () => {
        if (!dbReady || !isAuthenticated) return

        try {
            setLoadingCategories(true)
            const categories = await staffApi.listAssetCategories()
            setAssetCategories(categories)
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setLoadingCategories(false)
        }
    }, [dbReady, isAuthenticated, setGlobalError])

    const resetImportFlow = useCallback(() => {
        setSelectedImportMode(DEFAULT_ASSET_IMPORT_MODE)
        setSelectedFilePath(null)
        setInspection(null)
        setPreview(null)
        setReport(null)
        setStatusMessage("")
    }, [])

    const openImportWizard = useCallback(() => {
        setPanelMode("import")
        setWizardOpen(true)
        resetImportFlow()
    }, [resetImportFlow])

    const openManualAssetPanel = useCallback(() => {
        setPanelMode("manual")
        setWizardOpen(true)
        setManualAssetMessage("")
        setManualAssetResult(null)
        void loadAssetCategories()
    }, [loadAssetCategories])

    const closeWizard = useCallback(() => {
        setWizardOpen(false)
    }, [])

    const inspectFile = useCallback(
        async (filePath: string, sheetName?: string | null) => {
            setInspectingFile(true)
            setStatusMessage("")

            try {
                const nextInspection = await staffApi.inspectAssetImportFile({
                    filePath,
                    sheetName: sheetName ?? undefined,
                })

                setSelectedFilePath(filePath)
                setInspection(nextInspection)
                setPreview(null)
                setReport(null)
            } catch (error) {
                setGlobalError(getUserErrorMessage(error))
            } finally {
                setInspectingFile(false)
            }
        },
        [setGlobalError],
    )

    const handlePickImportFile = useCallback(async () => {
        try {
            const selected = await openFileDialog({
                multiple: false,
                filters: [{ name: "Asset Import Files", extensions: ["csv", "xlsx", "xls"] }],
            })

            if (!selected) return

            const filePath =
                typeof selected === "string" ? selected : (selected as { path: string }).path

            await inspectFile(filePath)
            setWizardOpen(true)
            setPanelMode("import")
        } catch (error) {
            const message = getUserErrorMessage(error)
            if (!message.toLowerCase().includes("cancel")) {
                setGlobalError(message)
            }
        }
    }, [inspectFile, setGlobalError])

    const prepareSelectedFile = useCallback(
        async (input: {
            filePath: string
            sheetName: string | null
            importType: AssetImportMode
        }) => {
            resetImportFlow()
            setSelectedImportMode(input.importType)
            await inspectFile(input.filePath, input.sheetName)
            setWizardOpen(true)
            setPanelMode('import')
        },
        [inspectFile, resetImportFlow],
    )

    const handleChangeSelectedSheet = useCallback(
        async (sheetName: string) => {
            if (!selectedFilePath) return
            await inspectFile(selectedFilePath, sheetName)
        },
        [inspectFile, selectedFilePath],
    )

    const handlePreviewImport = useCallback(async () => {
        if (!selectedFilePath) {
            setStatusMessage("Choose a CSV or Excel file before previewing the import.")
            return
        }

        try {
            setPreviewingImport(true)
            setStatusMessage("")
            const nextPreview = await staffApi.previewAssetImportFile({
                importType: selectedImportMode,
                filePath: selectedFilePath,
                sheetName: inspection?.selectedSheetName ?? undefined,
            })
            setPreview(nextPreview)
            setReport(null)
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setPreviewingImport(false)
        }
    }, [inspection?.selectedSheetName, selectedFilePath, selectedImportMode, setGlobalError])

    const handleApproveImport = useCallback(async () => {
        if (!selectedFilePath || !preview) return

        try {
            setApprovingImport(true)
            const nextReport = await staffApi.importAssetImportFile({
                importType: selectedImportMode,
                filePath: selectedFilePath,
                sheetName: inspection?.selectedSheetName ?? undefined,
            })
            setPreview(null)
            setReport(nextReport)
            setStatusMessage(
                `Imported ${nextReport.imported} valid row(s). Skipped ${nextReport.skipped} invalid row(s).`,
            )
            triggerReload()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setApprovingImport(false)
        }
    }, [
        inspection?.selectedSheetName,
        preview,
        selectedFilePath,
        selectedImportMode,
        setGlobalError,
        triggerReload,
    ])

    const handleCancelPreview = useCallback(() => {
        setPreview(null)
    }, [])

    const handleManualAssetFieldChange = useCallback(
        (fieldKey: keyof ManualAssetForm, value: string) => {
            setManualAssetForm((prev) => ({
                ...prev,
                [fieldKey]: value,
            }))
        },
        [],
    )

    const handleCreateManualAsset = useCallback(async () => {
        const payload: AssetSeedItemInput = {
            assetCode: manualAssetForm.assetCode.trim(),
            assetType: manualAssetForm.assetType.trim(),
            displayName: manualAssetForm.displayName.trim(),
            model: normalizeOptionalField(manualAssetForm.model),
            serialNumber: normalizeOptionalField(manualAssetForm.serialNumber),
            notes: normalizeOptionalField(manualAssetForm.notes),
        }

        if (!payload.assetCode || !payload.assetType || !payload.displayName) {
            setManualAssetMessage(buildManualSerializedAssetRequiredMessage())
            return
        }

        try {
            setCreatingManualAsset(true)
            const record = await staffApi.createAssetManually(payload)
            setManualAssetResult(record)
            setManualAssetMessage(buildManualSerializedAssetCreatedMessage(record.assetCode))
            setManualAssetForm(EMPTY_MANUAL_ASSET_FORM)
            triggerReload()
        } catch (error) {
            setManualAssetMessage(getUserErrorMessage(error))
        } finally {
            setCreatingManualAsset(false)
        }
    }, [manualAssetForm, triggerReload])

    const canPreviewCurrentFile = Boolean(selectedFilePath) && !isInspectingFile
    const previewApproveDisabled = (preview?.validRows ?? 0) === 0
    const selectedFiles = useMemo(
        () => (selectedFilePath ? [selectedFilePath] : []),
        [selectedFilePath],
    )

    return {
        isWizardOpen,
        panelMode,
        selectedImportMode,
        isLoadingCategories,
        isInspectingFile,
        isPreviewingImport,
        isApprovingImport,
        isCreatingManualAsset,
        statusMessage,
        assetCategories,
        selectedFilePath,
        selectedFiles,
        inspection,
        preview,
        report,
        canPreviewCurrentFile,
        previewApproveDisabled,
        manualAssetForm,
        manualAssetResult,
        manualAssetMessage,
        setSelectedImportMode,
        openImportWizard,
        openManualAssetPanel,
        closeWizard,
        handlePickImportFile,
        prepareSelectedFile,
        handleChangeSelectedSheet,
        handlePreviewImport,
        handleApproveImport,
        handleCancelPreview,
        handleManualAssetFieldChange,
        handleCreateManualAsset,
    }
}

function normalizeOptionalField(value: string): string | null {
    const next = value.trim()
    return next.length > 0 ? next : null
}
