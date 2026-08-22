import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import { useImportState } from './useImportState'

const mocks = vi.hoisted(() => ({
    inspectImportColumns: vi.fn(),
    previewImportExcel: vi.fn(),
    importExcel: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('../../services/staff-api', () => ({
    staffApi: {
        inspectImportColumns: mocks.inspectImportColumns,
        previewImportExcel: mocks.previewImportExcel,
        importExcel: mocks.importExcel,
    },
}))

describe('useImportState selected-file bridge', () => {
    beforeEach(() => {
        mocks.inspectImportColumns.mockReset()
        mocks.previewImportExcel.mockReset()
        mocks.importExcel.mockReset()
        vi.mocked(openFileDialog).mockReset()
        mocks.inspectImportColumns.mockResolvedValue({
            sourceFiles: ['C:\\imports\\employees.xlsx'],
            detectedColumns: [
                { key: 'email', label: 'Email', source: 'core', required: true },
                { key: 'team', label: 'Team', source: 'dynamic', required: false },
            ],
        })
        mocks.previewImportExcel.mockResolvedValue({
            sourceFiles: ['C:\\imports\\employees.xlsx'],
            sheetName: 'Onboarding',
            previewRows: [{
                rowNumber: 2,
                employeeId: 'ASWVN001',
                fullName: 'A',
                isUpdate: false,
                changes: [],
            }],
            totalChanges: 0,
            totalNew: 1,
            totalUpdates: 0,
            errors: [],
        })
    })

    it('opens the existing Employee preview flow for a detected file', async () => {
        const { result } = renderHook(() => useImportState({
            staffGroupFilter: 'employee_list',
            setGlobalError: vi.fn(),
            triggerReload: vi.fn(),
        }))

        await act(async () => result.current.prepareSelectedFile({
            filePath: 'C:\\imports\\employees.xlsx',
            sheetName: 'Onboarding',
            targetStaffGroup: 'onboarding',
        }))

        expect(mocks.inspectImportColumns).toHaveBeenCalledWith({
            filePaths: ['C:\\imports\\employees.xlsx'],
            sheetName: 'Onboarding',
        })
        expect(result.current.isImportDrawerOpen).toBe(true)
        expect(result.current.importSelectedFiles).toEqual(['C:\\imports\\employees.xlsx'])
        expect(result.current.importTargetGroup).toBe('onboarding')

        await act(async () => result.current.handleImportSelectedColumns())

        expect(mocks.previewImportExcel).toHaveBeenCalledWith({
            filePaths: ['C:\\imports\\employees.xlsx'],
            selectedColumnKeys: ['email', 'team'],
            targetStaffGroup: 'onboarding',
            sheetName: 'Onboarding',
        })
        expect(mocks.importExcel).not.toHaveBeenCalled()

        await act(async () => result.current.handleApprovePreviewRows())
        expect(mocks.importExcel).toHaveBeenCalledWith({
            filePaths: ['C:\\imports\\employees.xlsx'],
            selectedColumnKeys: ['email', 'team'],
            targetStaffGroup: 'onboarding',
            sheetName: 'Onboarding',
        })
    })

    it('keeps the existing Employee launcher using its native file picker', async () => {
        vi.mocked(openFileDialog).mockResolvedValue('C:\\imports\\legacy.xlsx')
        const { result } = renderHook(() => useImportState({
            staffGroupFilter: 'employee_list',
            setGlobalError: vi.fn(),
            triggerReload: vi.fn(),
        }))

        await act(async () => result.current.handlePickImportFiles())

        expect(vi.mocked(openFileDialog)).toHaveBeenCalledTimes(1)
        expect(result.current.isImportDrawerOpen).toBe(true)
    })
})
