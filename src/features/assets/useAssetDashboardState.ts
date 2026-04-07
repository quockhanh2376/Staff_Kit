import { useCallback, useEffect, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type {
  AssetCategoryDetailRecord,
  AssetCategoryUpsertInput,
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
  const [categoryDetails, setCategoryDetails] = useState<AssetCategoryDetailRecord[]>([])
  const [isLoadingDashboard, setLoadingDashboard] = useState(false)
  const [isRefreshingDashboard, setRefreshingDashboard] = useState(false)
  const [isUpdatingStockItemId, setUpdatingStockItemId] = useState<number | null>(null)
  const [isLoadingCategories, setLoadingCategories] = useState(false)
  const [isSavingCategory, setSavingCategory] = useState(false)
  const [isDeactivatingCategoryId, setDeactivatingCategoryId] = useState<number | null>(null)
  const [statusMessage, setStatusMessage] = useState("")

  const loadDashboard = useCallback(
    async ({ refresh = false }: { refresh?: boolean } = {}) => {
      if (!dbReady || !isAuthenticated) {
        setSummary(null)
        setSerializedRows([])
        setQuantityRows([])
        setCategoryDetails([])
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

  const loadCategoryDetails = useCallback(async () => {
    if (!dbReady || !isAuthenticated) {
      setCategoryDetails([])
      return
    }

    try {
      setLoadingCategories(true)
      const nextCategoryDetails = await staffApi.listAssetCategoryDetails()
      setCategoryDetails(nextCategoryDetails)
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setLoadingCategories(false)
    }
  }, [dbReady, isAuthenticated, setGlobalError])

  useEffect(() => {
    void loadDashboard()
  }, [loadDashboard, reloadToken])

  useEffect(() => {
    void loadCategoryDetails()
  }, [loadCategoryDetails, reloadToken])

  const refreshDashboard = useCallback(async () => {
    await Promise.all([
      loadDashboard({ refresh: true }),
      loadCategoryDetails(),
    ])
  }, [loadCategoryDetails, loadDashboard])

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

  const saveAssetCategory = useCallback(
    async (payload: AssetCategoryUpsertInput) => {
      try {
        setSavingCategory(true)
        const savedCategory = payload.id
          ? await staffApi.updateAssetCategory(payload)
          : await staffApi.createAssetCategory(payload)
        setStatusMessage(`Saved asset category ${savedCategory.categoryName}.`)
        await Promise.all([
          loadDashboard({ refresh: true }),
          loadCategoryDetails(),
        ])
        return savedCategory
      } catch (error) {
        setGlobalError(getErrorMessage(error))
        return null
      } finally {
        setSavingCategory(false)
      }
    },
    [loadCategoryDetails, loadDashboard, setGlobalError],
  )

  const deactivateAssetCategory = useCallback(
    async (categoryId: number) => {
      try {
        setDeactivatingCategoryId(categoryId)
        const updatedCategory = await staffApi.deactivateAssetCategory(categoryId)
        setStatusMessage(`Deactivated asset category ${updatedCategory.categoryName}.`)
        await Promise.all([
          loadDashboard({ refresh: true }),
          loadCategoryDetails(),
        ])
        return updatedCategory
      } catch (error) {
        setGlobalError(getErrorMessage(error))
        return null
      } finally {
        setDeactivatingCategoryId(null)
      }
    },
    [loadCategoryDetails, loadDashboard, setGlobalError],
  )

  return {
    summary,
    serializedRows,
    quantityRows,
    categoryDetails,
    isLoadingDashboard,
    isRefreshingDashboard,
    isUpdatingStockItemId,
    isLoadingCategories,
    isSavingCategory,
    isDeactivatingCategoryId,
    statusMessage,
    refreshDashboard,
    updateStockItemQuantity,
    saveAssetCategory,
    deactivateAssetCategory,
  }
}
