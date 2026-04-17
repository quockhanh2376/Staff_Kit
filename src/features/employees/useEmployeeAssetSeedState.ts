import { useCallback, useMemo, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type {
    EmployeeAssetSeedInput,
    EmployeeAssetSeedPreview,
    EmployeeAssetSeedReport,
} from "../../types/staff"
import type { StaffGroupKey } from "../../types/app"
import { getErrorMessage } from "../../lib/utils"
import { ALL_TEAMS_OPTION } from "../../lib/constants"

type UseEmployeeAssetSeedStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    canImportData: boolean
    staffGroupFilter: StaffGroupKey
    searchTerm: string
    teamFilter: string
    startDateFrom: string
    startDateTo: string
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
}

export type EmployeeAssetSeedState = ReturnType<typeof useEmployeeAssetSeedState>

export function useEmployeeAssetSeedState({
    dbReady,
    isAuthenticated,
    canImportData,
    staffGroupFilter,
    searchTerm,
    teamFilter,
    startDateFrom,
    startDateTo,
    setGlobalError,
    triggerReload,
}: UseEmployeeAssetSeedStateOptions) {
    const [isDrawerOpen, setDrawerOpen] = useState(false)
    const [isPreviewing, setPreviewing] = useState(false)
    const [isImporting, setImporting] = useState(false)
    const [preview, setPreview] = useState<EmployeeAssetSeedPreview | null>(null)
    const [report, setReport] = useState<EmployeeAssetSeedReport | null>(null)
    const [statusMessage, setStatusMessage] = useState("")
    const [approvedPayload, setApprovedPayload] = useState<EmployeeAssetSeedInput | null>(null)

    const payload = useMemo<EmployeeAssetSeedInput>(
        () => ({
            query: searchTerm.trim() || null,
            teamName: teamFilter !== ALL_TEAMS_OPTION ? teamFilter : null,
            staffGroup: staffGroupFilter,
            startDateFrom: startDateFrom || null,
            startDateTo: startDateTo || null,
        }),
        [searchTerm, teamFilter, staffGroupFilter, startDateFrom, startDateTo],
    )

    const sourceSummaryLines = useMemo(() => {
        return [
            "Source: Employee List filters",
            payload.query ? `Search: ${payload.query}` : "Search: All employees",
            payload.teamName ? `Team: ${payload.teamName}` : "Team: All teams",
            payload.startDateFrom ? `Start from: ${payload.startDateFrom}` : "Start from: Any date",
            payload.startDateTo ? `Start to: ${payload.startDateTo}` : "Start to: Any date",
        ]
    }, [payload])

    const openDrawer = useCallback(() => {
        setDrawerOpen(true)
        setPreview(null)
        setReport(null)
        setStatusMessage("")
        setApprovedPayload(null)
    }, [])

    const closeDrawer = useCallback(() => {
        setDrawerOpen(false)
        setPreview(null)
    }, [])

    const canOpenDrawer =
        dbReady && isAuthenticated && canImportData && staffGroupFilter === "employee_list"

    const handlePreview = useCallback(async () => {
        if (!canOpenDrawer) {
            return
        }

        try {
            setPreviewing(true)
            setStatusMessage("")
            const nextPreview = await staffApi.previewEmployeeAssetSeed(payload)
            setPreview(nextPreview)
            setReport(null)
            setApprovedPayload(payload)
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setPreviewing(false)
        }
    }, [canOpenDrawer, payload, setGlobalError])

    const handleApprove = useCallback(async () => {
        const importPayload = approvedPayload ?? payload
        if (!canOpenDrawer) {
            return
        }

        try {
            setImporting(true)
            const nextReport = await staffApi.importEmployeeAssetSeed(importPayload)
            setPreview(null)
            setReport(nextReport)
            setStatusMessage(
                `Imported ${nextReport.imported} asset(s). Skipped ${nextReport.skipped} row(s).`,
            )
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setImporting(false)
        }
    }, [approvedPayload, canOpenDrawer, payload, setGlobalError, triggerReload])

    const handleCancelPreview = useCallback(() => {
        setPreview(null)
    }, [])

    return {
        isDrawerOpen,
        isPreviewing,
        isImporting,
        preview,
        report,
        statusMessage,
        sourceSummaryLines,
        canOpenDrawer,
        previewApproveDisabled: (preview?.validRows ?? 0) === 0,
        openDrawer,
        closeDrawer,
        handlePreview,
        handleApprove,
        handleCancelPreview,
    }
}
