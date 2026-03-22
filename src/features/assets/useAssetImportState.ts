import { useCallback, useEffect, useMemo, useState } from "react"
import { open as openFileDialog } from "@tauri-apps/plugin-dialog"
import { staffApi } from "../../services/staff-api"
import type {
    AssetCategoryRecord,
    AssetImportBatchDetail,
    AssetImportBatchSummary,
    AssetImportFieldMapping,
    AssetImportFileInspection,
    AssetImportMode,
    AssetImportRowRecord,
    AssetRecord,
    AssetSeedItemInput,
} from "../../types/staff"
import { getErrorMessage } from "../../lib/utils"

type UseAssetImportStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    reloadToken: number
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
}

type AssetImportStep = "choose_file" | "map_columns" | "review_batch"
type AssetImportPanelMode = "import" | "manual"
type AssetImportReviewFilter = "all" | "errors" | "pending"
type ManualAssetForm = {
    assetCode: string
    assetType: string
    displayName: string
    model: string
    serialNumber: string
    notes: string
}

const REQUIRED_MAPPING_KEYS = ["assetCode", "assetType", "displayName"] as const
const OPTIONAL_MAPPING_KEYS = ["model", "serialNumber", "notes"] as const
const EDITABLE_ROW_FIELD_KEYS = [
    "assetCode",
    "assetType",
    "displayName",
    "model",
    "serialNumber",
    "notes",
] as const

const EMPTY_MANUAL_ASSET_FORM: ManualAssetForm = {
    assetCode: "",
    assetType: "",
    displayName: "",
    model: "",
    serialNumber: "",
    notes: "",
}

const DEFAULT_ASSET_IMPORT_MODE: AssetImportMode = "serialized"

export type AssetImportState = ReturnType<typeof useAssetImportState>

export function useAssetImportState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
    triggerReload,
}: UseAssetImportStateOptions) {
    const [isWizardOpen, setWizardOpen] = useState(false)
    const [panelMode, setPanelMode] = useState<AssetImportPanelMode>("import")
    const [currentStep, setCurrentStep] = useState<AssetImportStep>("choose_file")
    const [isLoadingBatches, setLoadingBatches] = useState(false)
    const [isLoadingCategories, setLoadingCategories] = useState(false)
    const [isInspectingFile, setInspectingFile] = useState(false)
    const [isCreatingBatch, setCreatingBatch] = useState(false)
    const [isRefreshingBatch, setRefreshingBatch] = useState(false)
    const [isUpdatingRow, setUpdatingRow] = useState<number | null>(null)
    const [isImportingRows, setImportingRows] = useState(false)
    const [isDeletingBatch, setDeletingBatch] = useState(false)
    const [isCreatingManualAsset, setCreatingManualAsset] = useState(false)
    const [statusMessage, setStatusMessage] = useState("")
    const [assetCategories, setAssetCategories] = useState<AssetCategoryRecord[]>([])
    const [batchSummaries, setBatchSummaries] = useState<AssetImportBatchSummary[]>([])
    const [activeBatchDetail, setActiveBatchDetail] = useState<AssetImportBatchDetail | null>(null)
    const [selectedRowId, setSelectedRowId] = useState<number | null>(null)
    const [reviewFilter, setReviewFilter] = useState<AssetImportReviewFilter>("all")
    const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null)
    const [selectedImportMode, setSelectedImportMode] =
        useState<AssetImportMode>(DEFAULT_ASSET_IMPORT_MODE)
    const [inspection, setInspection] = useState<AssetImportFileInspection | null>(null)
    const [selectedSheetName, setSelectedSheetName] = useState<string | null>(null)
    const [mappingDraft, setMappingDraft] = useState<AssetImportFieldMapping>({})
    const [manualAssetForm, setManualAssetForm] = useState<ManualAssetForm>(EMPTY_MANUAL_ASSET_FORM)
    const [manualAssetResult, setManualAssetResult] = useState<AssetRecord | null>(null)
    const [manualAssetMessage, setManualAssetMessage] = useState("")

    const loadBatchSummaries = useCallback(async () => {
        if (!dbReady || !isAuthenticated) return

        try {
            setLoadingBatches(true)
            const summaries = await staffApi.listAssetImportBatches()
            setBatchSummaries(summaries)
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setLoadingBatches(false)
        }
    }, [dbReady, isAuthenticated, setGlobalError])

    const loadAssetCategories = useCallback(async () => {
        if (!dbReady || !isAuthenticated) return

        try {
            setLoadingCategories(true)
            const categories = await staffApi.listAssetCategories()
            setAssetCategories(categories)
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setLoadingCategories(false)
        }
    }, [dbReady, isAuthenticated, setGlobalError])

    useEffect(() => {
        if (!dbReady || !isAuthenticated) return
        void loadBatchSummaries()
    }, [dbReady, isAuthenticated, loadBatchSummaries, reloadToken])

    useEffect(() => {
        if (!dbReady || !isAuthenticated) return
        void loadAssetCategories()
    }, [dbReady, isAuthenticated, loadAssetCategories, reloadToken])

    useEffect(() => {
        if (activeBatchDetail?.rows.length && !activeBatchDetail.rows.some((row) => row.id === selectedRowId)) {
            setSelectedRowId(activeBatchDetail.rows[0]?.id ?? null)
        }
    }, [activeBatchDetail, selectedRowId])

    const resetImportComposer = useCallback(() => {
        setCurrentStep("choose_file")
        setStatusMessage("")
        setSelectedFilePath(null)
        setInspection(null)
        setSelectedSheetName(null)
        setMappingDraft({})
        setActiveBatchDetail(null)
        setSelectedRowId(null)
        setReviewFilter("all")
    }, [])

    const openImportWizard = useCallback(() => {
        setPanelMode("import")
        setWizardOpen(true)
        setStatusMessage("")
        if (!activeBatchDetail) {
            setCurrentStep("choose_file")
        }
        void loadBatchSummaries()
    }, [activeBatchDetail, loadBatchSummaries])

    const openManualAssetPanel = useCallback(() => {
        setPanelMode("manual")
        setWizardOpen(true)
        setManualAssetMessage("")
        setManualAssetResult(null)
    }, [])

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
                setSelectedSheetName(nextInspection.selectedSheetName)
                setMappingDraft(nextInspection.mapping)
                setCurrentStep("choose_file")
                setActiveBatchDetail(null)
                setSelectedRowId(null)
            } catch (error) {
                setGlobalError(getErrorMessage(error))
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
            const message = getErrorMessage(error)
            if (!message.toLowerCase().includes("cancel")) {
                setGlobalError(message)
            }
        }
    }, [inspectFile, setGlobalError])

    const handleChangeSelectedSheet = useCallback(
        async (sheetName: string) => {
            if (!selectedFilePath) return
            await inspectFile(selectedFilePath, sheetName)
        },
        [inspectFile, selectedFilePath],
    )

    const updateMappingField = useCallback(
        (fieldKey: keyof AssetImportFieldMapping, header: string | null) => {
            setMappingDraft((prev) => ({
                ...prev,
                [fieldKey]: header || null,
            }))
        },
        [],
    )

    const refreshActiveBatch = useCallback(async () => {
        if (!activeBatchDetail) return

        try {
            setRefreshingBatch(true)
            const detail = await staffApi.getAssetImportBatchDetail(activeBatchDetail.summary.id)
            setActiveBatchDetail(detail)
            setSelectedImportMode(detail.summary.importType)
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setRefreshingBatch(false)
        }
    }, [activeBatchDetail, setGlobalError])

    const handleStageBatch = useCallback(async () => {
        if (!selectedFilePath) {
            setStatusMessage("Choose a CSV or Excel file before staging a batch.")
            return
        }

        try {
            setCreatingBatch(true)
            setStatusMessage("")
            const detail = await staffApi.createAssetImportBatch({
                importType: selectedImportMode,
                filePath: selectedFilePath,
                sheetName: selectedSheetName ?? undefined,
                mapping: inspection?.requiresManualMapping ? mappingDraft : undefined,
            })

            setActiveBatchDetail(detail)
            setSelectedRowId(detail.rows[0]?.id ?? null)
            setCurrentStep("review_batch")
            setReviewFilter("all")
            setStatusMessage(`Staged batch ${detail.summary.batchKey} with ${detail.summary.totalRows} row(s).`)
            await loadBatchSummaries()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setCreatingBatch(false)
        }
    }, [
        inspection?.requiresManualMapping,
        loadBatchSummaries,
        mappingDraft,
        selectedImportMode,
        selectedFilePath,
        selectedSheetName,
        setGlobalError,
    ])

    const openBatchDetail = useCallback(
        async (batchId: number) => {
            try {
                setRefreshingBatch(true)
                const detail = await staffApi.getAssetImportBatchDetail(batchId)
                setActiveBatchDetail(detail)
                setSelectedImportMode(detail.summary.importType)
                setSelectedRowId(detail.rows[0]?.id ?? null)
                setCurrentStep("review_batch")
                setPanelMode("import")
                setWizardOpen(true)
                setStatusMessage("")
            } catch (error) {
                setGlobalError(getErrorMessage(error))
            } finally {
                setRefreshingBatch(false)
            }
        },
        [setGlobalError],
    )

    const handleUpdateRowField = useCallback(
        async (
            rowId: number,
            fieldKey: (typeof EDITABLE_ROW_FIELD_KEYS)[number],
            value: string,
        ) => {
            try {
                setUpdatingRow(rowId)
                await staffApi.updateAssetImportRow({
                    rowId,
                    fieldKey,
                    value,
                })
                await refreshActiveBatch()
                await loadBatchSummaries()
            } catch (error) {
                setGlobalError(getErrorMessage(error))
            } finally {
                setUpdatingRow(null)
            }
        },
        [loadBatchSummaries, refreshActiveBatch, setGlobalError],
    )

    const handleToggleRowSkipped = useCallback(
        async (row: AssetImportRowRecord) => {
            try {
                setUpdatingRow(row.id)
                await staffApi.setAssetImportRowSkipped({
                    rowId: row.id,
                    skipped: row.status !== "skipped",
                })
                await refreshActiveBatch()
                await loadBatchSummaries()
            } catch (error) {
                setGlobalError(getErrorMessage(error))
            } finally {
                setUpdatingRow(null)
            }
        },
        [loadBatchSummaries, refreshActiveBatch, setGlobalError],
    )

    const handleImportValidRows = useCallback(async () => {
        if (!activeBatchDetail) return

        try {
            setImportingRows(true)
            const result = await staffApi.importAssetImportBatchValidRows(activeBatchDetail.summary.id)
            setStatusMessage(
                `Imported ${result.importedCount} valid row(s). ${result.remainingErrorRows} row(s) still need review.`,
            )
            await refreshActiveBatch()
            await loadBatchSummaries()
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setImportingRows(false)
        }
    }, [activeBatchDetail, loadBatchSummaries, refreshActiveBatch, setGlobalError, triggerReload])

    const handleDeleteActiveBatch = useCallback(async () => {
        if (!activeBatchDetail) return

        const confirmed = window.confirm(
            `Delete staged batch ${activeBatchDetail.summary.batchKey}? Imported rows already committed into assets will be kept.`,
        )
        if (!confirmed) return

        try {
            setDeletingBatch(true)
            const deleted = await staffApi.deleteAssetImportBatch(activeBatchDetail.summary.id)
            if (deleted) {
                resetImportComposer()
                await loadBatchSummaries()
                setStatusMessage("Deleted staged batch.")
            }
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setDeletingBatch(false)
        }
    }, [activeBatchDetail, loadBatchSummaries, resetImportComposer, setGlobalError])

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
            setManualAssetMessage("Asset code, asset type, and display name are required.")
            return
        }

        try {
            setCreatingManualAsset(true)
            const record = await staffApi.createAssetManually(payload)
            setManualAssetResult(record)
            setManualAssetMessage(`Created asset ${record.assetCode} in main assets table.`)
            setManualAssetForm(EMPTY_MANUAL_ASSET_FORM)
            triggerReload()
        } catch (error) {
            setManualAssetMessage(getErrorMessage(error))
        } finally {
            setCreatingManualAsset(false)
        }
    }, [manualAssetForm, triggerReload])

    const filteredRows = useMemo(() => {
        if (!activeBatchDetail) return []

        switch (reviewFilter) {
            case "errors":
                return activeBatchDetail.rows.filter((row) => row.status === "error")
            case "pending":
                return activeBatchDetail.rows.filter(
                    (row) => row.status === "valid" || row.status === "error",
                )
            default:
                return activeBatchDetail.rows
        }
    }, [activeBatchDetail, reviewFilter])

    const selectedRow = useMemo(
        () => activeBatchDetail?.rows.find((row) => row.id === selectedRowId) ?? null,
        [activeBatchDetail, selectedRowId],
    )

    const activeBatchSummary = activeBatchDetail?.summary ?? null

    return {
        isWizardOpen,
        panelMode,
        currentStep,
        isLoadingBatches,
        isLoadingCategories,
        isInspectingFile,
        isCreatingBatch,
        isRefreshingBatch,
        isUpdatingRow,
        isImportingRows,
        isDeletingBatch,
        isCreatingManualAsset,
        statusMessage,
        assetCategories,
        batchSummaries,
        activeBatchDetail,
        activeBatchSummary,
        selectedRowId,
        selectedRow,
        reviewFilter,
        filteredRows,
        selectedFilePath,
        selectedImportMode,
        inspection,
        selectedSheetName,
        mappingDraft,
        manualAssetForm,
        manualAssetResult,
        manualAssetMessage,
        openImportWizard,
        openManualAssetPanel,
        closeWizard,
        handlePickImportFile,
        handleChangeSelectedSheet,
        updateMappingField,
        handleStageBatch,
        openBatchDetail,
        refreshActiveBatch,
        handleUpdateRowField,
        handleToggleRowSkipped,
        handleImportValidRows,
        handleDeleteActiveBatch,
        setSelectedRowId,
        setReviewFilter,
        setSelectedImportMode,
        setCurrentStep,
        resetImportComposer,
        handleManualAssetFieldChange,
        handleCreateManualAsset,
    }
}

function normalizeOptionalField(value: string): string | null {
    const next = value.trim()
    return next.length > 0 ? next : null
}

export function hasRequiredAssetImportMapping(mapping: AssetImportFieldMapping): boolean {
    return REQUIRED_MAPPING_KEYS.every((fieldKey) => Boolean(mapping[fieldKey]))
}

export const assetImportRequiredMappingKeys = REQUIRED_MAPPING_KEYS
export const assetImportOptionalMappingKeys = OPTIONAL_MAPPING_KEYS
export const assetImportEditableRowFieldKeys = EDITABLE_ROW_FIELD_KEYS
