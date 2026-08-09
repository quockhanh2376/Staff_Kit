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

type RefreshQueueOptions = {
  silent?: boolean
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
  const queueRefreshInFlightRef = useRef<Promise<boolean> | null>(null)
  const pendingRequestsRef = useRef(pendingRequests)
  const selectedRequestIdRef = useRef(selectedRequestId)
  const selectedRequestRef = useRef(selectedRequest)

  useEffect(() => {
    selectedRequestIdRef.current = selectedRequestId
  }, [selectedRequestId])

  useEffect(() => {
    selectedRequestRef.current = selectedRequest
  }, [selectedRequest])

  const loadRequestDetail = useCallback(
    async (requestId: number | null) => {
      if (!requestId) {
        selectedRequestRef.current = null
        setSelectedRequest(null)
        return
      }

      try {
        setLoadingDetail(true)
        const detail = await staffApi.getBorrowRequestDetail(requestId)
        selectedRequestRef.current = detail
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
    async (preferredRequestId?: number | null, options: RefreshQueueOptions = {}) => {
      if (queueRefreshInFlightRef.current) return queueRefreshInFlightRef.current

      const refresh = async () => {
        if (!dbReady || !isAuthenticated || !isAdminAccount) {
          selectedRequestIdRef.current = null
          selectedRequestRef.current = null
          setPendingRequests([])
          setSelectedRequestId(null)
          setSelectedRequest(null)
          pendingRequestsRef.current = []
          return false
        }

        try {
          if (!options.silent) setLoadingQueue(true)
          const items = await staffApi.listPendingBorrowRequests()
          const previousQueue = JSON.stringify(pendingRequestsRef.current)
          const nextQueue = JSON.stringify(items)
          const changed = previousQueue !== nextQueue
          if (changed) {
            pendingRequestsRef.current = items
            setPendingRequests(items)
          }

          const currentSelectedId = selectedRequestIdRef.current
          const nextSelectedId =
            preferredRequestId && items.some((item) => item.id === preferredRequestId)
              ? preferredRequestId
              : currentSelectedId && items.some((item) => item.id === currentSelectedId)
                ? currentSelectedId
                : items[0]?.id ?? null

          if (nextSelectedId !== currentSelectedId) {
            selectedRequestIdRef.current = nextSelectedId
            setSelectedRequestId(nextSelectedId)
          }
          if (!nextSelectedId) {
            selectedRequestRef.current = null
            setSelectedRequest(null)
          } else if (nextSelectedId !== currentSelectedId || selectedRequestRef.current?.id !== nextSelectedId) {
            await loadRequestDetail(nextSelectedId)
          }
          return changed
        } catch (error) {
          if (!options.silent) {
            setGlobalError(getUserErrorMessage(error))
          }
        } finally {
          if (!options.silent) setLoadingQueue(false)
        }
        return false
      }

      const request = refresh()
      queueRefreshInFlightRef.current = request
      try {
        return await request
      } finally {
        if (queueRefreshInFlightRef.current === request) {
          queueRefreshInFlightRef.current = null
        }
      }
    },
    [dbReady, isAuthenticated, isAdminAccount, loadRequestDetail, setGlobalError],
  )

  useEffect(() => {
    if (!dbReady || !isAuthenticated || !isAdminAccount) {
      pendingRequestsRef.current = []
      setPendingRequests([])
      setSelectedRequestId(null)
      setSelectedRequest(null)
      return
    }

    void refreshQueue(selectedRequestIdRef.current)
  }, [dbReady, isAuthenticated, isAdminAccount, reloadToken, refreshQueue])

  const handleSelectRequest = async (requestId: number) => {
    selectedRequestIdRef.current = requestId
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
