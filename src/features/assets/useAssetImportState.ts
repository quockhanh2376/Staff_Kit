import { useCallback, useEffect, useMemo, useState } from "react"
import { open as openFileDialog } from "@tauri-apps/plugin-dialog"
import { staffApi } from "../../services/staff-api"
import type {
    AssetCategoryRecord,
    AssetImportBatchDetail,
    AssetImportBatchSummary,
    AssetImportFileInspection,
    AssetImportMode,
    AssetImportRowRecord,
    AssetRecord,
    AssetSeedItemInput,
} from "../../types/staff"
import { getErrorMessage } from "../../lib/utils"
import {
    canStageAssetImportMode,
    convertBackendMappingToWizardMapping,
    detectAssetImportWizardMapping,
    getAssetImportStageBlockReason,
    getRequiredAssetImportMappingKeys,
    mergeAssetImportWizardMappings,
    toBackendAssetImportMapping,
    toBackendRowFieldKey,
    type AssetImportWizardFieldKey,
    type AssetImportWizardMapping,
} from "./assetImportModeConfig"
import {
    buildAssetImportDeleteMessage,
    buildAssetImportSuccessMessage,
} from "./assetImportMessages"
import {
    buildManualSerializedAssetCreatedMessage,
    buildManualSerializedAssetRequiredMessage,
} from "./assetImportCopy"
import { syncAssetImportWizardOnOpen } from "./assetImportOpenBehavior"

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
    const [mappingDraft, setMappingDraft] = useState<AssetImportWizardMapping>({})
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

    useEffect(() => {
        if (activeBatchDetail) {
            return
        }

        if (!inspection) {
            return
        }

        setMappingDraft(detectAssetImportWizardMapping(inspection.headers, selectedImportMode))
    }, [activeBatchDetail, inspection, selectedImportMode])

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
        setSelectedImportMode(DEFAULT_ASSET_IMPORT_MODE)
    }, [])

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
                setMappingDraft(
                    detectAssetImportWizardMapping(
                        nextInspection.headers,
                        selectedImportMode,
                    ),
                )
                setCurrentStep("choose_file")
                setActiveBatchDetail(null)
                setSelectedRowId(null)
            } catch (error) {
                setGlobalError(getErrorMessage(error))
            } finally {
                setInspectingFile(false)
            }
        },
        [selectedImportMode, setGlobalError],
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
        (fieldKey: AssetImportWizardFieldKey, header: string | null) => {
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
            setMappingDraft((prev) =>
                mergeAssetImportWizardMappings(
                    detectAssetImportWizardMapping(
                        detail.headers,
                        detail.summary.importType,
                    ),
                    mergeAssetImportWizardMappings(
                        prev,
                        convertBackendMappingToWizardMapping(
                            detail.summary.importType,
                            detail.mapping,
                        ),
                    ),
                ),
            )
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setRefreshingBatch(false)
        }
    }, [activeBatchDetail, setGlobalError])

    const openImportWizard = useCallback(() => {
        setPanelMode("import")
        setWizardOpen(true)
        setStatusMessage("")
        void syncAssetImportWizardOnOpen({
            hasActiveBatchDetail: Boolean(activeBatchDetail),
            openFreshWizard: () => {
                setCurrentStep("choose_file")
                setSelectedImportMode(DEFAULT_ASSET_IMPORT_MODE)
            },
            resetReviewFilterToAll: () => {
                setReviewFilter("all")
            },
            refreshActiveBatch,
            loadBatchSummaries,
        })
    }, [activeBatchDetail, loadBatchSummaries, refreshActiveBatch])

    const handleStageBatch = useCallback(async () => {
        if (!selectedFilePath) {
            setStatusMessage("Choose a CSV or Excel file before staging a batch.")
            return
        }

        if (!inspection) {
            setStatusMessage("Inspect a file before staging a batch.")
            return
        }

        const stageBlockReason = getAssetImportStageBlockReason(
            selectedImportMode,
            inspection.headers,
            mappingDraft,
        )
        if (stageBlockReason) {
            setStatusMessage(stageBlockReason)
            return
        }

        try {
            setCreatingBatch(true)
            setStatusMessage("")
            const detail = await staffApi.createAssetImportBatch({
                importType: selectedImportMode,
                filePath: selectedFilePath,
                sheetName: selectedSheetName ?? undefined,
                mapping:
                    toBackendAssetImportMapping(
                        selectedImportMode,
                        mappingDraft,
                        inspection.headers,
                    ) ?? undefined,
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
        loadBatchSummaries,
        mappingDraft,
        selectedImportMode,
        inspection,
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
                setMappingDraft(
                    mergeAssetImportWizardMappings(
                        detectAssetImportWizardMapping(
                            detail.headers,
                            detail.summary.importType,
                        ),
                        convertBackendMappingToWizardMapping(
                            detail.summary.importType,
                            detail.mapping,
                        ),
                    ),
                )
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
            fieldKey: AssetImportWizardFieldKey,
            value: string,
        ) => {
            const backendFieldKey = toBackendRowFieldKey(fieldKey)
            if (!backendFieldKey) {
                setStatusMessage("This column becomes editable in the next import runtime slice.")
                return
            }

            try {
                setUpdatingRow(rowId)
                await staffApi.updateAssetImportRow({
                    rowId,
                    fieldKey: backendFieldKey,
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
                buildAssetImportSuccessMessage(activeBatchDetail.summary.importType, result),
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
            buildAssetImportDeleteMessage(
                activeBatchDetail.summary.batchKey,
                activeBatchDetail.summary.importType,
            ),
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

    const currentImportMode = activeBatchDetail?.summary.importType ?? selectedImportMode

    const selectedRow = useMemo(
        () => activeBatchDetail?.rows.find((row) => row.id === selectedRowId) ?? null,
        [activeBatchDetail, selectedRowId],
    )

    const activeBatchSummary = activeBatchDetail?.summary ?? null
    const canStageCurrentMode = inspection
        ? canStageAssetImportMode(currentImportMode, inspection.headers, mappingDraft)
        : false
    const importBlockReason = null
    const canImportCurrentBatch =
        Boolean(activeBatchDetail?.summary.validRows) && !importBlockReason

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
        importBlockReason,
        canImportCurrentBatch,
        selectedFilePath,
        currentImportMode,
        selectedImportMode,
        canStageCurrentMode,
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

export function hasRequiredAssetImportMapping(
    mapping: AssetImportWizardMapping,
    mode: AssetImportMode,
): boolean {
    return getRequiredAssetImportMappingKeys(mode).every((fieldKey) =>
        Boolean(mapping[fieldKey]),
    )
}
