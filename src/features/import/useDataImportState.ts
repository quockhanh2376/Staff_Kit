import { useCallback, useMemo, useState } from 'react'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import { staffApi } from '../../services/staff-api'
import type {
    AssetImportMode,
    ImportDetectionResult,
} from '../../types/staff'
import type { StaffGroupKey } from '../../types/app'
import { getUserErrorMessage } from '../../lib/errorHandling'

export type DataImportRoute =
    | StaffGroupKey
    | 'asset:serialized'
    | 'asset:quantity'

type EmployeeImportBridge = {
    prepareSelectedFile: (input: {
        filePath: string
        sheetName: string | null
        targetStaffGroup: StaffGroupKey
    }) => Promise<void> | void
}

type AssetImportBridge = {
    prepareSelectedFile: (input: {
        filePath: string
        sheetName: string | null
        importType: AssetImportMode
    }) => Promise<void> | void
}

type UseDataImportStateOptions = {
    canImportData: boolean
    employeeImport: EmployeeImportBridge
    assetImport: AssetImportBridge
    setGlobalError: (message: string | null) => void
}

export type DataImportState = ReturnType<typeof useDataImportState>

const EMPLOYEE_ROUTES: StaffGroupKey[] = [
    'employee_list',
    'onboarding',
    'offboarding',
    'internal_movement',
]

const ASSET_ROUTES: DataImportRoute[] = ['asset:serialized', 'asset:quantity']

export function useDataImportState({
    canImportData,
    employeeImport,
    assetImport,
    setGlobalError,
}: UseDataImportStateOptions) {
    const [isOpen, setOpen] = useState(false)
    const [isDetecting, setDetecting] = useState(false)
    const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null)
    const [detection, setDetection] = useState<ImportDetectionResult | null>(null)
    const [routeChoice, setRouteChoice] = useState<DataImportRoute | null>(null)

    const open = useCallback(() => {
        if (canImportData) setOpen(true)
    }, [canImportData])

    const close = useCallback(() => {
        setOpen(false)
    }, [])

    const chooseFile = useCallback(async () => {
        if (!canImportData) {
            setGlobalError('Admin access required for import.')
            return
        }

        try {
            const selected = await openFileDialog({
                multiple: false,
                filters: [{ name: 'Import Files', extensions: ['csv', 'xlsx', 'xls'] }],
            })
            if (!selected) return

            const filePath =
                typeof selected === 'string' ? selected : (selected as { path: string }).path
            setSelectedFilePath(filePath)
            setDetection(null)
            setRouteChoice(null)
            setDetecting(true)
            const result = await staffApi.detectImportFile(filePath)
            setDetection(result)
            setRouteChoice(getAutomaticRoute(result))
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setDetecting(false)
        }
    }, [canImportData, setGlobalError])

    const candidateRoutes = useMemo(
        () => (detection?.type === 'ambiguous' ? getCandidateRoutes(detection) : []),
        [detection],
    )

    const selectRoute = useCallback(
        (route: DataImportRoute) => {
            if (!detection || detection.type !== 'ambiguous') return
            if (getCandidateRoutes(detection).includes(route)) setRouteChoice(route)
        },
        [detection],
    )

    const continueToPreview = useCallback(async () => {
        if (!selectedFilePath || !detection || !routeChoice) return
        const sheetName = detection.sheetName

        if (EMPLOYEE_ROUTES.includes(routeChoice as StaffGroupKey)) {
            await employeeImport.prepareSelectedFile({
                filePath: selectedFilePath,
                sheetName,
                targetStaffGroup: routeChoice as StaffGroupKey,
            })
        } else if (ASSET_ROUTES.includes(routeChoice)) {
            await assetImport.prepareSelectedFile({
                filePath: selectedFilePath,
                sheetName,
                importType: routeChoice.replace('asset:', '') as AssetImportMode,
            })
        } else {
            return
        }

        setOpen(false)
    }, [assetImport, detection, employeeImport, routeChoice, selectedFilePath])

    const actionableMessage = detection?.type === 'unknown' ? detection.reason : null

    return {
        isOpen,
        isDetecting,
        selectedFilePath,
        selectedFileName: selectedFilePath ? getFileName(selectedFilePath) : null,
        detection,
        routeChoice,
        candidateRoutes,
        actionableMessage,
        canContinue: Boolean(selectedFilePath && detection && routeChoice && !isDetecting),
        open,
        close,
        chooseFile,
        selectRoute,
        continueToPreview,
    }
}

function getAutomaticRoute(result: ImportDetectionResult): DataImportRoute | null {
    if (result.type === 'employee' && result.subtype && EMPLOYEE_ROUTES.includes(result.subtype as StaffGroupKey)) {
        return result.subtype as StaffGroupKey
    }
    if (result.type === 'asset' && (result.subtype === 'serialized' || result.subtype === 'quantity')) {
        return 'asset:' + result.subtype as DataImportRoute
    }
    return null
}

function getCandidateRoutes(result: ImportDetectionResult): DataImportRoute[] {
    const routes: DataImportRoute[] = []
    if (result.candidateTypes.includes('employee')) routes.push(...EMPLOYEE_ROUTES)
    if (result.candidateTypes.includes('asset:serialized') || result.candidateTypes.includes('asset:serialized_legacy_manual_confirmation')) {
        routes.push('asset:serialized')
    }
    if (result.candidateTypes.includes('asset:quantity')) routes.push('asset:quantity')
    return routes
}

function getFileName(filePath: string): string {
    return filePath.split(/[\\/]/).pop() ?? filePath
}
