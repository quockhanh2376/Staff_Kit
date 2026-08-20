import { Drawer } from '../../components/Drawer'
import { SharedImportShell } from './sharedImportShell'
import type { ImportDetectionResult } from '../../types/staff'
import type { DataImportRoute, DataImportState } from './useDataImportState'

type DataImportPanelProps = {
    canImportData: boolean
    dataImport: DataImportState
}

export function DataImportPanel({ canImportData, dataImport }: DataImportPanelProps) {
    const detection = dataImport.detection
    const selectedFiles = dataImport.selectedFileName ? [dataImport.selectedFileName] : []

    return (
        <Drawer
            open={dataImport.isOpen}
            onClose={dataImport.close}
            title='Data Import'
            widthClass='w-[720px]'
        >
            <SharedImportShell
                fileSectionTitle='Choose Excel or CSV file'
                fileSectionDescription='Staff Kit will detect the import destination before opening the existing preview and approval flow.'
                selectedFiles={selectedFiles}
                emptyFilesLabel='No file selected yet.'
                chooseButtonLabel='Choose File'
                chooseButtonBusyLabel='Detecting...'
                onChooseFiles={() => void dataImport.chooseFile()}
                isChoosingFiles={dataImport.isDetecting}
                primaryActionLabel='Continue to Preview'
                primaryActionBusyLabel='Opening Preview...'
                onPrimaryAction={() => void dataImport.continueToPreview()}
                isPrimaryActionDisabled={!canImportData || !dataImport.canContinue}
                isPrimaryActionBusy={false}
                onClose={dataImport.close}
            >
                {!detection ? (
                    <div className='rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-4 text-sm text-[var(--text-secondary)]'>
                        Choose a file to begin read-only detection.
                    </div>
                ) : (
                    <DetectionSummary dataImport={dataImport} detection={detection} />
                )}
            </SharedImportShell>
        </Drawer>
    )
}

function DetectionSummary({
    dataImport,
    detection,
}: {
    dataImport: DataImportState
    detection: ImportDetectionResult
}) {
    const isUnknown = detection.kind === 'unknown'
    const isAmbiguous = detection.kind === 'ambiguous'

    return (
        <div className='space-y-4'>
            <div className={`rounded-[10px] border p-4 ${isUnknown ? 'border-red-500/35 bg-red-500/10' : isAmbiguous ? 'border-amber-500/35 bg-amber-500/10' : 'border-emerald-500/35 bg-emerald-500/10'}`}>
                <div className='text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-secondary)]'>
                    Detection result
                </div>
                <div className='mt-1 text-lg font-semibold text-[var(--text-primary)]'>
                    {getDetectionLabel(detection)}
                </div>
                <div className='mt-3 grid gap-3 sm:grid-cols-2'>
                    <InfoRow label='Confidence' value={`${Math.round(detection.confidence * 100)}%`} />
                    <InfoRow label='Detected sheet' value={detection.sheetName ?? 'Not identified'} />
                    <InfoRow label='Data rows' value={String(detection.rowCount)} />
                    <InfoRow label='Header row' value={detection.headerRow ? String(detection.headerRow) : 'Not identified'} />
                </div>
            </div>

            <div className='rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-4'>
                <div className='text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-secondary)]'>Evidence / reason</div>
                <div className='mt-2 text-sm text-[var(--text-primary)]'>{detection.reason}</div>
                {detection.evidenceHeaders.length > 0 && (
                    <div className='mt-3 flex flex-wrap gap-2'>
                        {detection.evidenceHeaders.map((header) => (
                            <span key={header} className='rounded-[999px] border border-[var(--border)] bg-[var(--surface)] px-2.5 py-1 text-xs text-[var(--text-primary)]'>
                                {header}
                            </span>
                        ))}
                    </div>
                )}
            </div>

            {detection.warnings.length > 0 && (
                <div className='rounded-[10px] border border-amber-500/35 bg-amber-500/10 p-4 text-sm text-amber-100'>
                    <div className='font-semibold'>Warnings</div>
                    <ul className='mt-2 list-disc space-y-1 pl-5'>
                        {detection.warnings.map((warning) => <li key={warning}>{warning}</li>)}
                    </ul>
                </div>
            )}

            {isUnknown && dataImport.actionableMessage && (
                <div role='alert' className='rounded-[10px] border border-red-500/35 bg-red-500/10 p-4 text-sm text-red-100'>
                    {dataImport.actionableMessage}
                </div>
            )}

            {isAmbiguous && (
                <div className='rounded-[10px] border border-amber-500/35 bg-amber-500/10 p-4'>
                    <div className='text-sm font-semibold text-[var(--text-primary)]'>Choose an import destination before continuing.</div>
                    <div className='mt-3 grid gap-2 sm:grid-cols-2'>
                        {dataImport.candidateRoutes.map((route) => (
                            <button
                                key={route}
                                type='button'
                                onClick={() => dataImport.selectRoute(route)}
                                className={`rounded-[8px] border px-3 py-2 text-left text-sm ${dataImport.routeChoice === route ? 'border-[var(--primary)] bg-[var(--primary)]/10 text-[var(--text-primary)]' : 'border-[var(--border)] text-[var(--text-secondary)]'}`}
                            >
                                {getRouteLabel(route)}
                            </button>
                        ))}
                    </div>
                </div>
            )}
        </div>
    )
}

function InfoRow({ label, value }: { label: string; value: string }) {
    return (
        <div>
            <div className='text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]'>{label}</div>
            <div className='mt-1 text-sm text-[var(--text-primary)]'>{value}</div>
        </div>
    )
}

function getDetectionLabel(detection: ImportDetectionResult): string {
    if (detection.kind === 'ambiguous') return 'Ambiguous Import'
    if (detection.kind === 'unknown') return 'Unknown Workbook'
    return detection.subtype ? getRouteLabel(detection.subtype) : 'Import'
}

function getRouteLabel(route: DataImportRoute | string): string {
    switch (route) {
        case 'employee_list': return 'Employee List'
        case 'onboarding': return 'Onboarding Employees'
        case 'offboarding': return 'Offboarding Employees'
        case 'internal_movement': return 'Internal Movement'
        case 'asset:serialized':
        case 'serialized': return 'Serialized Assets'
        case 'asset:quantity':
        case 'quantity': return 'Quantity Stock'
        default: return route
    }
}
