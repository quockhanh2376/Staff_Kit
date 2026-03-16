import { useCallback, useEffect, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { BorrowLanSettings, BorrowRequestRecord } from "../../types/staff"
import { getErrorMessage } from "../../lib/utils"

type UseBorrowStateOptions = {
  dbReady: boolean
  isAuthenticated: boolean
  isAdminAccount: boolean
  reloadToken: number
  borrowLanSettings: BorrowLanSettings | null
  setGlobalError: (msg: string | null) => void
  triggerReload: () => void
}

export type BorrowState = ReturnType<typeof useBorrowState>

export function useBorrowState({
  dbReady,
  isAuthenticated,
  isAdminAccount,
  reloadToken,
  borrowLanSettings,
  setGlobalError,
  triggerReload,
}: UseBorrowStateOptions) {
  const [pendingRequests, setPendingRequests] = useState<BorrowRequestRecord[]>([])
  const [selectedRequestId, setSelectedRequestId] = useState<number | null>(null)
  const [selectedRequest, setSelectedRequest] = useState<BorrowRequestRecord | null>(null)
  const [reviewNote, setReviewNote] = useState("")
  const [queueMessage, setQueueMessage] = useState("")
  const [isLoadingQueue, setLoadingQueue] = useState(false)
  const [isLoadingDetail, setLoadingDetail] = useState(false)
  const [isApproving, setApproving] = useState(false)
  const [isRejecting, setRejecting] = useState(false)

  const loadRequestDetail = useCallback(
    async (requestId: number | null) => {
      if (!requestId) {
        setSelectedRequest(null)
        return
      }

      try {
        setLoadingDetail(true)
        const detail = await staffApi.getBorrowRequestDetail(requestId)
        setSelectedRequest(detail)
      } catch (error) {
        setGlobalError(getErrorMessage(error))
      } finally {
        setLoadingDetail(false)
      }
    },
    [setGlobalError],
  )

  const refreshQueue = useCallback(
    async (preferredRequestId?: number | null) => {
      if (!dbReady || !isAuthenticated || !isAdminAccount) {
        setPendingRequests([])
        setSelectedRequestId(null)
        setSelectedRequest(null)
        return
      }

      try {
        setLoadingQueue(true)
        const items = await staffApi.listPendingBorrowRequests()
        setPendingRequests(items)

        const nextSelectedId =
          preferredRequestId && items.some((item) => item.id === preferredRequestId)
            ? preferredRequestId
            : items[0]?.id ?? null

        setSelectedRequestId(nextSelectedId)
        await loadRequestDetail(nextSelectedId)
      } catch (error) {
        setGlobalError(getErrorMessage(error))
      } finally {
        setLoadingQueue(false)
      }
    },
    [dbReady, isAuthenticated, isAdminAccount, loadRequestDetail, setGlobalError],
  )

  useEffect(() => {
    if (!dbReady || !isAuthenticated || !isAdminAccount) {
      setPendingRequests([])
      setSelectedRequestId(null)
      setSelectedRequest(null)
      return
    }

    void refreshQueue(selectedRequestId)
  }, [dbReady, isAuthenticated, isAdminAccount, reloadToken, refreshQueue, selectedRequestId])

  const handleSelectRequest = async (requestId: number) => {
    setSelectedRequestId(requestId)
    await loadRequestDetail(requestId)
  }

  const handleApproveRequest = async () => {
    if (!selectedRequest) return

    try {
      setApproving(true)
      setQueueMessage("")
      const approved = await staffApi.approveBorrowRequest(selectedRequest.id)
      setQueueMessage(`Approved request ${approved.requestKey}. Stock and loan records were updated.`)
      setReviewNote("")
      await refreshQueue(selectedRequest.id)
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setApproving(false)
    }
  }

  const handleRejectRequest = async () => {
    if (!selectedRequest) return

    const note = reviewNote.trim()
    if (!note) {
      setQueueMessage("A rejection note is required.")
      return
    }

    try {
      setRejecting(true)
      setQueueMessage("")
      const rejected = await staffApi.rejectBorrowRequest({
        requestId: selectedRequest.id,
        note,
      })
      setQueueMessage(`Rejected request ${rejected.requestKey}. The employee must resubmit with the correct asset item.`)
      setReviewNote("")
      await refreshQueue(selectedRequest.id)
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setRejecting(false)
    }
  }

  return {
    borrowLanSettings,
    pendingRequests,
    selectedRequestId,
    selectedRequest,
    reviewNote,
    setReviewNote,
    queueMessage,
    isLoadingQueue,
    isLoadingDetail,
    isApproving,
    isRejecting,
    handleSelectRequest,
    handleApproveRequest,
    handleRejectRequest,
    refreshQueue,
  }
}
