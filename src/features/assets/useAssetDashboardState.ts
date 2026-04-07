import { useCallback, useEffect, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type {
  AssetDashboardQuantityRecord,
  AssetDashboardSerializedRecord,
  AssetDashboardSummary,
  StockItemQuantityUpdateInput,
} from "../../types/staff"
import { getErrorMessage } from "../../lib/utils"

type UseAssetDashboardStateOptions = {
  dbReady: boolean
  isAuthenticated: boolean
  reloadToken: number
  setGlobalError: (msg: string | null) => void
}

export type AssetDashboardState = ReturnType<typeof useAssetDashboardState>

export function useAssetDashboardState({
  dbReady,
  isAuthenticated,
  reloadToken,
  setGlobalError,
}: UseAssetDashboardStateOptions) {
  const [summary, setSummary] = useState<AssetDashboardSummary | null>(null)
  const [serializedRows, setSerializedRows] = useState<AssetDashboardSerializedRecord[]>([])
  const [quantityRows, setQuantityRows] = useState<AssetDashboardQuantityRecord[]>([])
  const [isLoadingDashboard, setLoadingDashboard] = useState(false)
  const [isRefreshingDashboard, setRefreshingDashboard] = useState(false)
  const [isUpdatingStockItemId, setUpdatingStockItemId] = useState<number | null>(null)
  const [statusMessage, setStatusMessage] = useState("")

  const loadDashboard = useCallback(
    async ({ refresh = false }: { refresh?: boolean } = {}) => {
      if (!dbReady || !isAuthenticated) {
        setSummary(null)
        setSerializedRows([])
        setQuantityRows([])
        setStatusMessage("")
        return
      }

      try {
        if (refresh) {
          setRefreshingDashboard(true)
        } else {
          setLoadingDashboard(true)
        }

        const [nextSummary, nextSerializedRows, nextQuantityRows] = await Promise.all([
          staffApi.getAssetDashboardSummary(),
          staffApi.listAssetDashboardSerialized(),
          staffApi.listAssetDashboardQuantity(),
        ])

        setSummary(nextSummary)
        setSerializedRows(nextSerializedRows)
        setQuantityRows(nextQuantityRows)
      } catch (error) {
        setGlobalError(getErrorMessage(error))
      } finally {
        setLoadingDashboard(false)
        setRefreshingDashboard(false)
      }
    },
    [dbReady, isAuthenticated, setGlobalError],
  )

  useEffect(() => {
    void loadDashboard()
  }, [loadDashboard, reloadToken])

  const refreshDashboard = useCallback(async () => {
    await loadDashboard({ refresh: true })
  }, [loadDashboard])

  const updateStockItemQuantity = useCallback(
    async (payload: StockItemQuantityUpdateInput) => {
      try {
        setUpdatingStockItemId(payload.stockItemId)
        const updatedRow = await staffApi.updateStockItemQuantity(payload)
        setStatusMessage(`Updated stock quantities for ${updatedRow.itemName}.`)
        await loadDashboard({ refresh: true })
        return updatedRow
      } catch (error) {
        setGlobalError(getErrorMessage(error))
        return null
      } finally {
        setUpdatingStockItemId(null)
      }
    },
    [loadDashboard, setGlobalError],
  )

  return {
    summary,
    serializedRows,
    quantityRows,
    isLoadingDashboard,
    isRefreshingDashboard,
    isUpdatingStockItemId,
    statusMessage,
    refreshDashboard,
    updateStockItemQuantity,
  }
}
