import { useEffect, useId, useRef, type ReactNode } from "react"
import { LoaderCircle, Upload } from "lucide-react"
import type { SharedImportErrorItem, SharedImportStatItem } from "./sharedImportTypes"

type SharedImportShellPreviewProps<Row> = {
    title: string
    summaryItems: SharedImportStatItem[]
    rows: Row[]
    errors: SharedImportErrorItem[]
    renderRow: (row: Row, index: number) => ReactNode
    onCancel: () => void
    onApprove: () => void
    approveDisabled?: boolean
    isApproving: boolean
}

type SharedImportShellReportProps = {
    title: string
    summaryItems: SharedImportStatItem[]
    errors: SharedImportErrorItem[]
}

type SharedImportShellProps<Row> = {
    fileSectionTitle: string
    fileSectionDescription: string
    selectedFiles: string[]
    emptyFilesLabel: string
    chooseButtonLabel?: string
    chooseButtonBusyLabel?: string
    onChooseFiles?: () => void
    isChoosingFiles?: boolean
    primaryActionLabel: string
    primaryActionBusyLabel: string
    onPrimaryAction: () => void
    isPrimaryActionDisabled: boolean
    isPrimaryActionBusy: boolean
    onClose: () => void
    children?: ReactNode
    preview?: SharedImportShellPreviewProps<Row> | null
    report?: SharedImportShellReportProps | null
}

export function SharedImportShell<Row>({
    fileSectionTitle,
    fileSectionDescription,
    selectedFiles,
    emptyFilesLabel,
    chooseButtonLabel,
    chooseButtonBusyLabel,
    onChooseFiles,
    isChoosingFiles,
    primaryActionLabel,
    primaryActionBusyLabel,
    onPrimaryAction,
    isPrimaryActionDisabled,
    isPrimaryActionBusy,
    onClose,
    children,
    preview,
    report,
}: SharedImportShellProps<Row>) {
    const previewDialogId = useId()
    const previewDialogRef = useRef<HTMLDivElement | null>(null)
    const hasChooseAction =
        typeof onChooseFiles === "function" &&
        typeof chooseButtonLabel === "string" &&
        typeof chooseButtonBusyLabel === "string"
    const isChoosing = isChoosingFiles ?? false

    useEffect(() => {
        if (!preview || typeof window === "undefined") {
            return
        }

        const dialog = previewDialogRef.current
        dialog?.focus()

        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                event.preventDefault()
                preview.onCancel()
            }
        }

        window.addEventListener("keydown", handleKeyDown)
        return () => window.removeEventListener("keydown", handleKeyDown)
    }, [preview])

    return (
        <>
            <div className="space-y-5">
                <div className="rounded-[8px] border border-dashed border-[var(--border)] bg-[var(--surface-hover)]/35 p-4">
                    <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                        <Upload size={16} className="text-[var(--primary)]" />
                        {fileSectionTitle}
                    </div>
                    <div className="mt-2 text-xs text-[var(--text-secondary)]">
                        {fileSectionDescription}
                    </div>
                    <div className="mt-3 space-y-2 text-xs text-[var(--text-secondary)]">
                        {selectedFiles.length === 0 ? (
                            <div className="rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-2">
                                {emptyFilesLabel}
                            </div>
                        ) : (
                            selectedFiles.map((file) => (
                                <div
                                    key={file}
                                    className="break-all rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-2"
                                >
                                    {file}
                                </div>
                            ))
                        )}
                    </div>
                </div>

                {children}

                <div className="flex flex-wrap items-center gap-2 border-t border-[var(--border)] pt-4">
                    {hasChooseAction && (
                    <button
                        className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium"
                        onClick={onChooseFiles}
                        type="button"
                        disabled={isChoosing || isPrimaryActionBusy}
                    >
                        {isChoosing ? (
                            <span className="inline-flex items-center gap-2">
                                <LoaderCircle className="animate-spin" size={14} />
                                {chooseButtonBusyLabel}
                            </span>
                        ) : (
                            chooseButtonLabel
                        )}
                    </button>
                    )}
                    <button
                        className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                        onClick={onPrimaryAction}
                        type="button"
                        disabled={isPrimaryActionDisabled || isPrimaryActionBusy || isChoosing}
                    >
                        {isPrimaryActionBusy ? primaryActionBusyLabel : primaryActionLabel}
                    </button>
                    <button
                        className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium"
                        onClick={onClose}
                        type="button"
                    >
                        Close
                    </button>
                </div>

                {report && (
                    <div className="space-y-3 rounded-[8px] border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
                        <div className="font-semibold">{report.title}</div>
                        <div className="grid grid-cols-2 gap-2 text-xs">
                            {report.summaryItems.map((item) => (
                                <div key={item.label}>
                                    {item.label}: {item.value}
                                </div>
                            ))}
                        </div>
                        {report.errors.length > 0 && (
                            <div className="max-h-44 overflow-auto rounded-[6px] border border-emerald-500/30 bg-black/20 p-2 text-xs">
                                {report.errors.slice(0, 12).map((item, index) => (
                                    <div key={`${item.rowNumber}-${index}`}>
                                        Row {item.rowNumber}
                                        {item.entityKey ? ` (${item.entityKey})` : ""}: {item.reason}
                                    </div>
                                ))}
                            </div>
                        )}
                    </div>
                )}
            </div>

            {preview && (
                <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div
                        ref={previewDialogRef}
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby={previewDialogId}
                        tabIndex={-1}
                        className="max-h-[90vh] w-full max-w-4xl overflow-auto rounded-lg border border-[var(--border)] bg-[var(--bg)] p-6 shadow-xl focus:outline-none"
                    >
                        <h2 id={previewDialogId} className="mb-4 text-lg font-semibold">
                            {preview.title}
                        </h2>

                        <div className="mb-6 flex flex-wrap gap-6 rounded-md border border-[var(--border)] bg-[var(--bg-secondary)] p-4">
                            {preview.summaryItems.map((item) => (
                                <div key={item.label}>
                                    <div className="text-sm text-[var(--text-secondary)]">{item.label}</div>
                                    <div className="text-xl font-bold">{item.value}</div>
                                </div>
                            ))}
                        </div>

                        <div className="mb-6 max-h-[50vh] space-y-3 overflow-y-auto">
                            {preview.rows.map((row, index) => preview.renderRow(row, index))}
                        </div>

                        {preview.errors.length > 0 && (
                            <div className="mb-6 rounded border border-red-200 bg-red-50 p-3 dark:border-red-800 dark:bg-red-950">
                                <div className="text-sm font-medium text-red-800 dark:text-red-200">
                                    Errors ({preview.errors.length})
                                </div>
                                {preview.errors.slice(0, 8).map((item, index) => (
                                    <div key={`${item.rowNumber}-${index}`} className="text-xs text-red-700 dark:text-red-300">
                                        Row {item.rowNumber}
                                        {item.entityKey ? ` (${item.entityKey})` : ""}: {item.reason}
                                    </div>
                                ))}
                            </div>
                        )}

                        <div className="flex gap-3 border-t border-[var(--border)] pt-4">
                            <button
                                className="flex-1 rounded-md border border-[var(--border)] px-4 py-2 text-sm font-medium hover:bg-[var(--bg-secondary)]"
                                onClick={preview.onCancel}
                                disabled={preview.isApproving}
                            >
                                Cancel
                            </button>
                            <button
                                className="rounded-md bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:opacity-50"
                                onClick={preview.onApprove}
                                disabled={preview.approveDisabled || preview.isApproving}
                            >
                                {preview.isApproving ? "Importing..." : "Approve Import"}
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </>
    )
}
