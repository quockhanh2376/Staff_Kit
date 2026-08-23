import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import { staffApi } from '../../services/staff-api'
import type { ImportDetectionResult } from '../../types/staff'
import { useDataImportState } from './useDataImportState'

vi.mock('@tauri-apps/plugin-dialog', () => ({
    open: vi.fn(),
}))

vi.mock('../../services/staff-api', () => ({
    staffApi: {
        detectImportFile: vi.fn(),
    },
}))

const detector = vi.mocked(staffApi.detectImportFile)
const chooseFile = vi.mocked(openFileDialog)

const employeeImport = () => ({
    prepareSelectedFile: vi.fn(),
    commit: vi.fn(),
})

const assetImport = () => ({
    prepareSelectedFile: vi.fn(),
    commit: vi.fn(),
})

const detection = (overrides: Partial<ImportDetectionResult> = {}): ImportDetectionResult => ({
    type: 'employee',
    subtype: 'onboarding',
    confidence: 0.95,
    sheetName: 'Onboarding',
    headerRow: 1,
    rowCount: 1,
    evidenceHeaders: ['EE.ID', 'Full Name'],
    reason: 'Employee match-key and name headers were detected.',
    warnings: [],
    candidateTypes: ['employee'],
    ...overrides,
})

function renderDataImport() {
    const employee = employeeImport()
    const asset = assetImport()
    const setGlobalError = vi.fn()
    const hook = renderHook(() =>
        useDataImportState({
            canImportData: true,
            employeeImport: employee,
            assetImport: asset,
            setGlobalError,
        }),
    )

    return { ...hook, employee, asset, setGlobalError }
}

describe('useDataImportState', () => {
    beforeEach(() => {
        detector.mockReset()
        chooseFile.mockReset()
        chooseFile.mockResolvedValue('C:\\imports\\source.xlsx')
    })

    it.each([
        ['onboarding', 'onboarding', 'onboarding'],
        ['employee list', 'employee_list', 'employee_list'],
        ['offboarding', 'offboarding', 'offboarding'],
        ['internal movement', 'internal_movement', 'internal_movement'],
    ])('routes %s to the Employee preview entry point', async (_label, subtype, expectedRoute) => {
        detector.mockResolvedValue(detection({ subtype: subtype as ImportDetectionResult['subtype'] }))
        const { result, employee, asset } = renderDataImport()

        act(() => result.current.open())
        await act(async () => result.current.chooseFile())
        await act(async () => result.current.continueToPreview())

        expect(employee.prepareSelectedFile).toHaveBeenCalledWith({
            filePath: 'C:\\imports\\source.xlsx',
            sheetName: 'Onboarding',
            targetStaffGroup: expectedRoute,
        })
        expect(asset.prepareSelectedFile).not.toHaveBeenCalled()
        expect(employee.commit).not.toHaveBeenCalled()
    })

    it.each([
        ['serialized', 'serialized'],
        ['quantity', 'quantity'],
    ])('routes %s to the Asset preview entry point', async (_label, subtype) => {
        detector.mockResolvedValue(
            detection({
                type: 'asset',
                subtype: subtype as ImportDetectionResult['subtype'],
                sheetName: 'Assets',
                evidenceHeaders: ['Asset Tag', 'Asset Name', 'Category'],
                candidateTypes: [`asset:${subtype}`],
            }),
        )
        const { result, employee, asset } = renderDataImport()

        act(() => result.current.open())
        await act(async () => result.current.chooseFile())
        await act(async () => result.current.continueToPreview())

        expect(asset.prepareSelectedFile).toHaveBeenCalledWith({
            filePath: 'C:\\imports\\source.xlsx',
            sheetName: 'Assets',
            importType: subtype,
        })
        expect(employee.prepareSelectedFile).not.toHaveBeenCalled()
        expect(asset.commit).not.toHaveBeenCalled()
    })

    it('routes the backend type field for serialized assets and preserves the selected sheet', async () => {
        detector.mockResolvedValue({
            type: 'asset',
            subtype: 'serialized',
            confidence: 0.98,
            sheetName: 'Asset',
            headerRow: 1,
            rowCount: 6,
            evidenceHeaders: ['Asset Tag', 'Asset Name', 'Category', 'Computer Name'],
            reason: 'Canonical serialized asset headers were detected.',
            warnings: [],
            candidateTypes: ['asset:serialized'],
        })
        const { result, employee, asset } = renderDataImport()

        act(() => result.current.open())
        await act(async () => result.current.chooseFile())

        expect(result.current.canContinue).toBe(true)
        await act(async () => result.current.continueToPreview())

        expect(asset.prepareSelectedFile).toHaveBeenCalledWith({
            filePath: 'C:\\imports\\source.xlsx',
            sheetName: 'Asset',
            importType: 'serialized',
        })
        expect(employee.prepareSelectedFile).not.toHaveBeenCalled()
    })

    it('requires a manual route for ambiguous detection', async () => {
        detector.mockResolvedValue(
            detection({
                type: 'ambiguous',
                subtype: null,
                confidence: 0,
                candidateTypes: ['employee', 'asset:serialized'],
                warnings: ['No import route was selected automatically.'],
            }),
        )
        const { result, employee, asset } = renderDataImport()

        act(() => result.current.open())
        await act(async () => result.current.chooseFile())
        expect(result.current.routeChoice).toBeNull()

        await act(async () => result.current.continueToPreview())
        expect(employee.prepareSelectedFile).not.toHaveBeenCalled()
        expect(asset.prepareSelectedFile).not.toHaveBeenCalled()

        act(() => result.current.selectRoute('asset:serialized'))
        await act(async () => result.current.continueToPreview())
        expect(asset.prepareSelectedFile).toHaveBeenCalledTimes(1)
    })

    it('keeps unknown workbooks actionable and unrouted', async () => {
        detector.mockResolvedValue(
            detection({
                type: 'unknown',
                subtype: null,
                confidence: 0,
                reason: 'No recognizable Employee or Asset import columns were found.',
                candidateTypes: [],
            }),
        )
        const { result, employee, asset } = renderDataImport()

        act(() => result.current.open())
        await act(async () => result.current.chooseFile())

        expect(result.current.actionableMessage).toContain('No recognizable Employee or Asset')
        await act(async () => result.current.continueToPreview())
        expect(employee.prepareSelectedFile).not.toHaveBeenCalled()
        expect(asset.prepareSelectedFile).not.toHaveBeenCalled()
    })

    it('reports detector failures without invoking either importer', async () => {
        detector.mockRejectedValue(new Error('file could not be read'))
        const { result, employee, asset, setGlobalError } = renderDataImport()

        act(() => result.current.open())
        await act(async () => result.current.chooseFile())

        expect(setGlobalError).toHaveBeenCalledWith(expect.any(String))
        expect(employee.prepareSelectedFile).not.toHaveBeenCalled()
        expect(asset.prepareSelectedFile).not.toHaveBeenCalled()
    })
})
