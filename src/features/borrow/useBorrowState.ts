import { useCallback, useEffect, useRef, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { BorrowLanSettings, BorrowRequestRecord } from "../../types/staff"
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
  const [lanServerAlive, setLanServerAlive] = useState<boolean | null>(null)

  useEffect(() => {
    const port = borrowLanSettings?.port
    if (!port) {
      setLanServerAlive(null)
      return
    }
    let disposed = false
    setLanServerAlive(null)
    void staffApi
      .probeLanServer(port)
      .then((alive) => {
        if (!disposed) setLanServerAlive(alive)
      })
      .catch(() => {
        if (!disposed) setLanServerAlive(false)
      })
    return () => {
      disposed = true
    }
  }, [borrowLanSettings?.port])

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
    if (!selectedRequest) return

    try {
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
      setQueueMessage(
        buildBorrowReviewRejectSuccessMessage(rejected.requestType, rejected.requestKey),
      )
      setReviewNote("")
      await refreshQueue(selectedRequest.id)
      triggerReload()
    } catch (error) {
      setGlobalError(getUserErrorMessage(error))
    } finally {
      setRejecting(false)
    }
  }

  return {
    borrowLanSettings,
    lanServerAlive,
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
