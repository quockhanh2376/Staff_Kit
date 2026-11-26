import { useCallback, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { AssetImportBatchDetail, AssetImportBatchSummary, AssetImportRowUpdateInput } from "../../types/staff"
import { getUserErrorMessage } from "../../lib/errorHandling"

type Options = {
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
}

export type AssetImportStagingState = ReturnType<typeof useAssetImportStaging>

const isPendingSerialized = (batch: AssetImportBatchSummary) =>
    batch.status === "pending_review" && batch.importType !== "quantity"

/** Serialized asset import staged in asset_import_batches: load, edit, skip, then approve or discard. */
export function useAssetImportStaging({ setGlobalError, triggerReload }: Options) {
    const [batch, setBatch] = useState<AssetImportBatchDetail | null>(null)
    const [pendingBatch, setPendingBatch] = useState<AssetImportBatchSummary | null>(null)
    const [isOpen, setOpen] = useState(false)
    const [isBusy, setBusy] = useState(false)
    const [error, setError] = useState<string | null>(null)

    // Errors from row edits stay in the drawer; load errors go to the app banner.
    const run = useCallback(async <T,>(work: () => Promise<T>, toDrawer = true): Promise<T | null> => {
        setBusy(true)
        setError(null)
        try {
            return await work()
        } catch (err) {
            const message = getUserErrorMessage(err)
            if (toDrawer) setError(message)
            else setGlobalError(message)
            return null
        } finally {
            setBusy(false)
        }
    }, [setGlobalError])

    const checkPendingBatch = useCallback(async () => {
        try {
            const batches = await staffApi.listAssetImportBatches()
            setPendingBatch(batches.filter(isPendingSerialized).at(-1) ?? null)
        } catch {
            setPendingBatch(null) // the wizard still works without the banner
        }
    }, [])

    // One open batch at a time: loading a file discards any older pending batch.
    const loadStaging = useCallback(
        async (filePath: string, sheetName: string | null) => {
            const detail = await run(async () => {
                const batches = await staffApi.listAssetImportBatches()
                for (const old of batches.filter((item) => item.status === "pending_review")) {
                    await staffApi.deleteAssetImportBatch(old.id)
                }
                return staffApi.createAssetImportBatch({ importType: "auto", filePath, sheetName })
            }, false)
            if (!detail) return false
            setBatch(detail)
            setPendingBatch(null)
            setOpen(true)
            return true
        },
        [run],
    )

    const continueStaging = useCallback(async () => {
        if (!pendingBatch) return false
        const detail = await run(() => staffApi.getAssetImportBatchDetail(pendingBatch.id), false)
        if (!detail) return false
        setBatch(detail)
        setOpen(true)
        return true
    }, [pendingBatch, run])

    // One edit can change other rows (duplicates), so the whole batch is reloaded.
    const reload = useCallback(async (batchId: number) => {
        setBatch(await staffApi.getAssetImportBatchDetail(batchId))
    }, [])

    const updateRow = useCallback(
        async (rowId: number, fieldKey: AssetImportRowUpdateInput["fieldKey"], value: string | null) => {
            if (!batch) return
            await run(async () => {
                await staffApi.updateAssetImportRow({ rowId, fieldKey, value })
                await reload(batch.id)
            })
        },
        [batch, reload, run],
    )

    const setRowSkipped = useCallback(
        async (rowId: number, skipped: boolean) => {
            if (!batch) return
            await run(async () => {
                await staffApi.setAssetImportRowSkipped({ rowId, skipped })
                await reload(batch.id)
            })
        },
        [batch, reload, run],
    )

    const approve = useCallback(async () => {
        if (!batch) return null
        const result = await run(() => staffApi.importAssetImportBatchValidRows(batch.id))
        if (!result) {
            // The backend refused (e.g. a row became an error); show the current state.
            await reload(batch.id).catch(() => undefined)
            return null
        }
        setBatch(null)
        setOpen(false)
        triggerReload()
        return result
    }, [batch, reload, run, triggerReload])

    const discard = useCallback(async () => {
        const target = batch?.id ?? pendingBatch?.id
        if (target === undefined) return false
        const done = await run(() => staffApi.deleteAssetImportBatch(target), !!batch)
        if (done === null) return false
        setBatch(null)
        setPendingBatch(null)
        setOpen(false)
        return true
    }, [batch, pendingBatch, run])

    return {
        batch,
        pendingBatch,
        isOpen,
        isBusy,
        error,
        close: () => setOpen(false),
        checkPendingBatch,
        loadStaging,
        continueStaging,
        updateRow,
        setRowSkipped,
        approve,
        discard,
    }
}
