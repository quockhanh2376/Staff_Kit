import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { ImportDetectionResult } from '../../types/staff'
import type { DataImportState } from './useDataImportState'
import { DataImportPanel } from './DataImportPanel'

function state(overrides: Partial<DataImportState> = {}): DataImportState {
    return {
        isOpen: true,
        isDetecting: false,
        selectedFilePath: 'C:\\imports\\Onboarding.xlsx',
        selectedFileName: 'Onboarding.xlsx',
        detection: null,
        routeChoice: null,
        candidateRoutes: [],
        actionableMessage: null,
        canContinue: false,
        open: vi.fn(),
        close: vi.fn(),
        chooseFile: vi.fn(),
        selectRoute: vi.fn(),
        continueToPreview: vi.fn(),
        ...overrides,
    }
}

const onboarding: ImportDetectionResult = {
    kind: 'employee',
    subtype: 'onboarding',
    confidence: 0.95,
    sheetName: 'Onboarding',
    headerRow: 1,
    rowCount: 1,
    evidenceHeaders: ['EE.ID', 'Full Name'],
    reason: 'Employee match-key and name headers were detected.',
    warnings: [],
    candidateTypes: ['employee'],
}

describe('DataImportPanel', () => {
    it('shows the detected result and delegates only after continuing', () => {
        const dataImport = state({ detection: onboarding, routeChoice: 'onboarding', canContinue: true })
        render(<DataImportPanel canImportData={true} dataImport={dataImport} />)

        expect(screen.getByRole('heading', { name: 'Data Import' })).toBeTruthy()
        expect(screen.getByText('Onboarding Employees')).toBeTruthy()
        expect(screen.getByText('95%')).toBeTruthy()
        expect(screen.getByText('Onboarding')).toBeTruthy()
        expect(screen.getByText('EE.ID')).toBeTruthy()

        fireEvent.click(screen.getByRole('button', { name: 'Continue to Preview' }))
        expect(dataImport.continueToPreview).toHaveBeenCalledTimes(1)
    })

    it('requires a manual route for ambiguous results', () => {
        const dataImport = state({
            detection: {
                ...onboarding,
                kind: 'ambiguous',
                subtype: null,
                confidence: 0,
                reason: 'Choose the destination explicitly.',
                candidateTypes: ['employee', 'asset:serialized'],
            },
            candidateRoutes: ['onboarding', 'asset:serialized'],
            routeChoice: null,
            canContinue: false,
        })
        render(<DataImportPanel canImportData={true} dataImport={dataImport} />)

        expect(screen.getByText('Choose an import destination before continuing.')).toBeTruthy()
        const continueButton = screen.getByRole('button', { name: 'Continue to Preview' })
        expect(continueButton.hasAttribute('disabled')).toBe(true)
        fireEvent.click(screen.getByRole('button', { name: 'Serialized Assets' }))
        expect(dataImport.selectRoute).toHaveBeenCalledWith('asset:serialized')
    })

    it('shows an actionable unknown-workbook message and disables routing', () => {
        const dataImport = state({
            detection: {
                ...onboarding,
                kind: 'unknown',
                subtype: null,
                confidence: 0,
                reason: 'Add Staff ID and Full Name, or Asset Tag and Asset Name headers.',
                candidateTypes: [],
            },
            actionableMessage: 'Add Staff ID and Full Name, or Asset Tag and Asset Name headers.',
            routeChoice: null,
            canContinue: false,
        })
        render(<DataImportPanel canImportData={true} dataImport={dataImport} />)

        expect(screen.getByRole('alert').textContent).toContain('Add Staff ID and Full Name, or Asset Tag and Asset Name headers.')
        expect(screen.getByRole('button', { name: 'Continue to Preview' }).hasAttribute('disabled')).toBe(true)
    })
})
