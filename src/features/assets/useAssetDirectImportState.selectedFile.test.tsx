import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAssetDirectImportState } from './useAssetDirectImportState'

const inspectAssetImportFile = vi.hoisted(() => vi.fn())

vi.mock('../../services/staff-api', () => ({
    staffApi: {
        inspectAssetImportFile,
    },
}))

describe('useAssetDirectImportState selected-file bridge', () => {
    beforeEach(() => {
        inspectAssetImportFile.mockReset()
        inspectAssetImportFile.mockResolvedValue({
            fileName: 'assets.xlsx',
            fileType: 'xlsx',
            selectedSheetName: 'Assets',
            availableSheets: ['Assets'],
            headerRow: 1,
            headers: ['Asset Tag', 'Asset Name', 'Category'],
            mapping: {
                assetCode: 'Asset Tag',
                assetType: 'Category',
                displayName: 'Asset Name',
                displayNameShort: null,
                computerName: null,
                brand: null,
                model: null,
                serialNumber: null,
                adapterNumber: null,
                usageLocation: null,
                quantity: null,
                warehouse: null,
                notes: null,
                submittedStaffId: null,
                submittedFullName: null,
                submittedTeam: null,
                submittedPhoneNumber: null,
            },
            requiresManualMapping: false,
        })
    })

    it('opens the existing Asset preview flow with the detector-selected mode', async () => {
        const { result } = renderHook(() => useAssetDirectImportState({
            dbReady: true,
            isAuthenticated: true,
            reloadToken: 0,
            setGlobalError: vi.fn(),
            triggerReload: vi.fn(),
        }))

        await act(async () => result.current.prepareSelectedFile({
            filePath: 'C:\\imports\\assets.xlsx',
            sheetName: 'Assets',
            importType: 'quantity',
        }))

        expect(inspectAssetImportFile).toHaveBeenCalledWith({
            filePath: 'C:\\imports\\assets.xlsx',
            sheetName: 'Assets',
        })
        expect(result.current.isWizardOpen).toBe(true)
        expect(result.current.selectedImportMode).toBe('quantity')
        expect(result.current.selectedFilePath).toBe('C:\\imports\\assets.xlsx')
        expect(result.current.inspection?.selectedSheetName).toBe('Assets')
    })
})
