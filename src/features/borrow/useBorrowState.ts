import { useCallback, useEffect, useRef, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { BorrowRequestRecord } from "../../types/staff"
import { getUserErrorMessage } from "../../lib/errorHandling"
import {
  buildBorrowReviewApproveSuccessMessage,
  buildBorrowReviewRejectSuccessMessage,
} from "./borrowReviewCopy"

type UseBorrowStateOptions = {
  dbReady: boolean
  isAuthenticated: boolean
  isAdminAccount: boolean
  reloadToken: number
  setGlobalError: (msg: string | null) => void
  triggerReload: () => void
}

export type BorrowState = ReturnType<typeof useBorrowState>

export function useBorrowState({
  dbReady,
  isAuthenticated,
  isAdminAccount,
  reloadToken,
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
  const [isCancelling, setCancelling] = useState(false)
  const reviewActionInFlightRef = useRef(false)

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
        setGlobalError(getUserErrorMessage(error))
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
        setGlobalError(getUserErrorMessage(error))
      } finally {
        setLoadingQueue(false)
      }
    },
    [dbReady, isAuthenticated, isAdminAccount, loadRequestDetail, setGlobalError],
  )

  const selectedRequestIdRef = useRef(selectedRequestId)
  useEffect(() => {
    selectedRequestIdRef.current = selectedRequestId
  }, [selectedRequestId])

  useEffect(() => {
    if (!dbReady || !isAuthenticated || !isAdminAccount) {
      setPendingRequests([])
      setSelectedRequestId(null)
      setSelectedRequest(null)
      return
    }

    void refreshQueue(selectedRequestIdRef.current)
  }, [dbReady, isAuthenticated, isAdminAccount, reloadToken, refreshQueue])

  const handleSelectRequest = async (requestId: number) => {
    setSelectedRequestId(requestId)
    await loadRequestDetail(requestId)
  }

  const handleApproveRequest = async () => {
    if (!selectedRequest || reviewActionInFlightRef.current) return

    try {
      reviewActionInFlightRef.current = true
      setApproving(true)
      setQueueMessage("")
      const approved = await staffApi.approveBorrowRequest(selectedRequest.id)
      setQueueMessage(
        buildBorrowReviewApproveSuccessMessage(approved.requestType, approved.requestKey),
      )
      setReviewNote("")
      await refreshQueue(selectedRequest.id)
      triggerReload()
    } catch (error) {
      setGlobalError(getUserErrorMessage(error))
    } finally {
      reviewActionInFlightRef.current = false
      setApproving(false)
    }
  }

  const handleRejectRequest = async () => {
    if (!selectedRequest || reviewActionInFlightRef.current) return

    const note = reviewNote.trim()
    if (!note) {
      setQueueMessage("A rejection note is required.")
      return
    }

    try {
      reviewActionInFlightRef.current = true
      setRejecting(true)
      setQueueMessage("")
      const rejected = await staffApi.rejectBorrowRequest({
        requestId: selectedRequest.id,
        note,
      })
      setQueueMessage(
        buildBorrowReviewRejectSuccessMessage(rejected.requestType, rejected.requestKey),
      )
      setReviewNote("")
      await refreshQueue(selectedRequest.id)
      triggerReload()
    } catch (error) {
      setGlobalError(getUserErrorMessage(error))
    } finally {
      reviewActionInFlightRef.current = false
      setRejecting(false)
    }
  }

  const handleCancelRequest = async () => {
    if (!selectedRequest || selectedRequest.status !== "pending" || reviewActionInFlightRef.current) return

    try {
      reviewActionInFlightRef.current = true
      setCancelling(true)
      setQueueMessage("")
      const cancelled = await staffApi.cancelBorrowRequest(selectedRequest.id)
      setQueueMessage(`Request ${cancelled.requestKey} cancelled.`)
      await refreshQueue(selectedRequest.id)
      triggerReload()
    } catch (error) {
      setQueueMessage(getUserErrorMessage(error))
    } finally {
      reviewActionInFlightRef.current = false
      setCancelling(false)
    }
  }

  return {
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
    isCancelling,
    handleSelectRequest,
    handleApproveRequest,
    handleRejectRequest,
    handleCancelRequest,
    refreshQueue,
  }
}
