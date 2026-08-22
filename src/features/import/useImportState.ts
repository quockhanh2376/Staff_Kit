import { useCallback, useState } from "react"
import { open as openFileDialog } from "@tauri-apps/plugin-dialog"
import { staffApi } from "../../services/staff-api"
import type { ImportColumnsPreview, ImportPreviewResult, ImportReport } from "../../types/staff"
import type { StaffGroupKey } from "../../types/app"
import { getUserErrorMessage } from "../../lib/errorHandling"

type UseImportStateOptions = {
    staffGroupFilter: StaffGroupKey
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
}

export type ImportState = ReturnType<typeof useImportState>

type ImportColumnOption = {
    key: string
    label: string
    source: "core" | "dynamic" | "required"
    required: boolean
}

export function useImportState({
    setGlobalError,
    triggerReload,
}: UseImportStateOptions) {
    const [isImportDrawerOpen, setImportDrawerOpen] = useState(false)
    const [isImporting, setImporting] = useState(false)
    const [importReport, setImportReport] = useState<ImportReport | null>(null)
    const [importSelectedFiles, setImportSelectedFiles] = useState<string[]>([])
    const [importSelectedSheetName, setImportSelectedSheetName] = useState<string | null>(null)
    const [importColumnOptions, setImportColumnOptions] = useState<ImportColumnOption[]>([])
    const [importSelectedColumnKeys, setImportSelectedColumnKeys] = useState<string[]>([])
    const [importTargetGroup, setImportTargetGroup] = useState<StaffGroupKey>("employee_list")
    const [importPreviewResult, setImportPreviewResult] = useState<ImportPreviewResult | null>(null)
    const [showImportPreviewModal, setShowImportPreviewModal] = useState(false)

    const importTargetGroupLabel = (() => {
        switch (importTargetGroup) {
            case "employee_list": return "Employee list"
            case "onboarding": return "Onboarding"
            case "offboarding": return "Offboarding"
            case "internal_movement": return "Internal Movement"
        }
    })()

    const effectiveImportColumnKeySet = new Set([
        ...importColumnOptions.filter((c) => c.required).map((c) => c.key),
        ...importSelectedColumnKeys,
    ])

    const prepareSelectedFile = useCallback(async (input: {
        filePath: string
        sheetName: string | null
        targetStaffGroup: StaffGroupKey
    }) => {
        const { filePath, sheetName, targetStaffGroup } = input
        try {
            setImporting(true)
            setImportReport(null)
            const result: ImportColumnsPreview = await staffApi.inspectImportColumns({
                filePaths: [filePath],
                sheetName,
            })
            setImportSelectedFiles(result.sourceFiles.length > 0 ? result.sourceFiles : [filePath])
            setImportSelectedSheetName(sheetName)
            setImportColumnOptions(
                result.detectedColumns.map((column) => ({
                    ...column,
                    required: column.source === 'core',
                })),
            )
            setImportSelectedColumnKeys(
                result.detectedColumns.filter((column) => column.source !== 'core').map((column) => column.key),
            )
            setImportTargetGroup(targetStaffGroup)
            setImportDrawerOpen(true)
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setImporting(false)
        }
    }, [setGlobalError])

    const handlePickImportFiles = async () => {
        try {
            // Step 1: open native Tauri file picker
            const selected = await openFileDialog({
                multiple: true,
                filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
            })

            if (!selected) return
            const filePaths = Array.isArray(selected) ? selected : [selected]
            if (filePaths.length === 0) return

            setImporting(true)
            setImportReport(null)

            // Step 2: inspect columns from selected files
            const result: ImportColumnsPreview = await staffApi.inspectImportColumns({ filePaths })
            setImportSelectedFiles(result.sourceFiles)
            setImportSelectedSheetName(null)
            setImportColumnOptions(
                result.detectedColumns.map((column) => ({
                    ...column,
                    required: column.source === "core",
                })),
            )
            setImportSelectedColumnKeys(
                result.detectedColumns.filter((c) => c.source !== "core").map((c) => c.key),
            )
            setImportDrawerOpen(true)
        } catch (error) {
            const msg = getUserErrorMessage(error)
            if (!msg.toLowerCase().includes("cancel")) {
                setGlobalError(msg)
            }
        } finally {
            setImporting(false)
        }
    }

    const toggleImportColumn = (column: ImportColumnOption) => {
        if (column.required) return
        setImportSelectedColumnKeys((prev) => {
            const included = prev.includes(column.key)
            return included ? prev.filter((k) => k !== column.key) : [...prev, column.key]
        })
    }

    const selectAllOptionalImportColumns = () => {
        setImportSelectedColumnKeys(
            importColumnOptions.filter((c) => !c.required).map((c) => c.key),
        )
    }

    const clearOptionalImportColumns = () => {
        setImportSelectedColumnKeys([])
    }

    const handleImportSelectedColumns = async () => {
        if (importSelectedFiles.length === 0) return

        try {
            setImporting(true)
            const preview = await staffApi.previewImportExcel({
                filePaths: importSelectedFiles,
                sheetName: importSelectedSheetName,
                selectedColumnKeys: [...effectiveImportColumnKeySet],
                targetStaffGroup: importTargetGroup,
            })
            setImportPreviewResult(preview)
            setShowImportPreviewModal(true)
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setImporting(false)
        }
    }

    const handleApprovePreviewRows = async () => {
        if (!importPreviewResult) return

        try {
            setImporting(true)
            const report = await staffApi.importExcel({
                filePaths: importSelectedFiles,
                sheetName: importSelectedSheetName,
                selectedColumnKeys: [...effectiveImportColumnKeySet],
                targetStaffGroup: importTargetGroup,
            })
            setImportReport(report)
            setImportPreviewResult(null)
            setShowImportPreviewModal(false)
            triggerReload()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setImporting(false)
        }
    }

    const handleRejectPreviewRows = useCallback(() => {
        setImportPreviewResult(null)
        setShowImportPreviewModal(false)
    }, [])

    const handleClose = useCallback(() => {
        setImportDrawerOpen(false)
    }, [])

    const handleOpenMssqlImport = useCallback(() => {
        setImportDrawerOpen(true)
    }, [])

    return {
        isImportDrawerOpen,
        setImportDrawerOpen,
        isImporting,
        importReport,
        importSelectedFiles,
        importSelectedSheetName,
        importColumnOptions,
        importSelectedColumnKeys,
        importTargetGroup,
        setImportTargetGroup,
        importTargetGroupLabel,
        effectiveImportColumnKeySet,
        importPreviewResult,
        showImportPreviewModal,
        handlePickImportFiles,
        prepareSelectedFile,
        toggleImportColumn,
        selectAllOptionalImportColumns,
        clearOptionalImportColumns,
        handleImportSelectedColumns,
        handleApprovePreviewRows,
        handleRejectPreviewRows,
        handleClose,
        handleOpenMssqlImport,
        setGlobalError,
        triggerReload,
    }
}
