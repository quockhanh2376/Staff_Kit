import { act, renderHook, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { useBorrowState } from "./useBorrowState"

const mocks = vi.hoisted(() => ({
  listPendingBorrowRequests: vi.fn(),
  getBorrowRequestDetail: vi.fn(),
  approveBorrowRequest: vi.fn(),
  rejectBorrowRequest: vi.fn(),
  cancelBorrowRequest: vi.fn(),
}))
const setGlobalError = vi.fn()

vi.mock("../../services/staff-api", () => ({
  staffApi: {
    listPendingBorrowRequests: mocks.listPendingBorrowRequests,
    getBorrowRequestDetail: mocks.getBorrowRequestDetail,
    cancelBorrowRequest: mocks.cancelBorrowRequest,
    approveBorrowRequest: mocks.approveBorrowRequest,
    rejectBorrowRequest: mocks.rejectBorrowRequest,
  },
}))

const pending = {
  id: 7,
  requestKey: "BR-0007",
  submittedEmployeeId: "EE1001",
  submittedFullName: "Nguyen Van A",
  status: "pending",
  requestType: "borrow",
  assetCodes: ["ASSET-001"],
  submittedAt: "2026-08-06T00:00:00Z",
  decisionNote: null,
}

describe("useBorrowState cancellation", () => {
  beforeEach(() => {
    mocks.listPendingBorrowRequests.mockReset()
    mocks.getBorrowRequestDetail.mockReset()
    mocks.approveBorrowRequest.mockReset()
    mocks.rejectBorrowRequest.mockReset()
    mocks.cancelBorrowRequest.mockReset()
    mocks.listPendingBorrowRequests.mockResolvedValue([pending])
    mocks.getBorrowRequestDetail.mockResolvedValue(pending)
    mocks.approveBorrowRequest.mockResolvedValue({ ...pending, status: "approved" })
    mocks.rejectBorrowRequest.mockResolvedValue({ ...pending, status: "rejected" })
    mocks.cancelBorrowRequest.mockResolvedValue({ ...pending, status: "cancelled" })
  })

  function renderReadyState() {
    return renderHook(() => useBorrowState({
      dbReady: true,
      isAuthenticated: true,
      isAdminAccount: true,
      reloadToken: 0,
      setGlobalError,
      triggerReload: vi.fn(),
    }))
  }

  it("cancels a pending request once and refreshes the queue", async () => {
    const refreshReload = vi.fn()
    const { result } = renderHook(() => useBorrowState({
      dbReady: true,
      isAuthenticated: true,
      isAdminAccount: true,
      reloadToken: 0,
      setGlobalError,
      triggerReload: refreshReload,
    }))

    await waitFor(() => expect(result.current.selectedRequest?.id).toBe(7))
    await act(async () => {
      await result.current.handleCancelRequest()
    })

    expect(mocks.cancelBorrowRequest).toHaveBeenCalledTimes(1)
    expect(mocks.cancelBorrowRequest).toHaveBeenCalledWith(7)
    expect(mocks.listPendingBorrowRequests).toHaveBeenCalledTimes(2)
    expect(refreshReload).toHaveBeenCalledTimes(1)
    expect(result.current.queueMessage).toContain("BR-0007")
  })

  it("guards duplicate cancellation calls while the first request is pending", async () => {
    let resolveCancellation: ((value: typeof pending) => void) | undefined
    mocks.cancelBorrowRequest.mockImplementation(
      () => new Promise((resolve) => { resolveCancellation = resolve }),
    )

    const { result } = renderHook(() => useBorrowState({
      dbReady: true,
      isAuthenticated: true,
      isAdminAccount: true,
      reloadToken: 0,
      setGlobalError,
      triggerReload: vi.fn(),
    }))

    await waitFor(() => expect(result.current.selectedRequest?.id).toBe(7))
    let first: Promise<void>
    let second: Promise<void>
    await act(async () => {
      first = result.current.handleCancelRequest()
      second = result.current.handleCancelRequest()
      await Promise.resolve()
    })

    expect(mocks.cancelBorrowRequest).toHaveBeenCalledTimes(1)
    resolveCancellation?.({ ...pending, status: "cancelled" })
    await act(async () => {
      await first
      await second
    })
  })

  it("blocks duplicate approve calls and allows the next approve after success", async () => {
    let resolveApproval: ((value: typeof pending) => void) | undefined
    mocks.approveBorrowRequest.mockImplementation(
      () => new Promise((resolve) => { resolveApproval = resolve }),
    )
    const { result } = renderReadyState()
    await waitFor(() => expect(result.current.selectedRequest?.id).toBe(7))

    let first: Promise<void>
    let second: Promise<void>
    await act(async () => {
      first = result.current.handleApproveRequest()
      second = result.current.handleApproveRequest()
      await Promise.resolve()
    })
    expect(mocks.approveBorrowRequest).toHaveBeenCalledTimes(1)
    resolveApproval?.({ ...pending, status: "approved" })
    await act(async () => { await first; await second })

    mocks.approveBorrowRequest.mockResolvedValue({ ...pending, status: "approved" })
    await act(async () => { await result.current.handleApproveRequest() })
    expect(mocks.approveBorrowRequest).toHaveBeenCalledTimes(2)
  })

  it("blocks duplicate reject calls and allows the next reject after failure", async () => {
    let rejectFailure: ((error: Error) => void) | undefined
    mocks.rejectBorrowRequest.mockImplementation(
      () => new Promise((_resolve, reject) => { rejectFailure = reject }),
    )
    const { result } = renderReadyState()
    await waitFor(() => expect(result.current.selectedRequest?.id).toBe(7))
    await act(async () => { result.current.setReviewNote("wrong asset") })

    let first: Promise<void>
    let second: Promise<void>
    await act(async () => {
      first = result.current.handleRejectRequest()
      second = result.current.handleRejectRequest()
      await Promise.resolve()
    })
    expect(mocks.rejectBorrowRequest).toHaveBeenCalledTimes(1)
    rejectFailure?.(new Error("temporary failure"))
    await act(async () => { await first; await second })

    mocks.rejectBorrowRequest.mockResolvedValue({ ...pending, status: "rejected" })
    await act(async () => { await result.current.handleRejectRequest() })
    expect(mocks.rejectBorrowRequest).toHaveBeenCalledTimes(2)
  })

  it("blocks approve followed immediately by reject", async () => {
    let resolveApproval: ((value: typeof pending) => void) | undefined
    mocks.approveBorrowRequest.mockImplementation(
      () => new Promise((resolve) => { resolveApproval = resolve }),
    )
    const { result } = renderReadyState()
    await waitFor(() => expect(result.current.selectedRequest?.id).toBe(7))
    await act(async () => { result.current.setReviewNote("wrong asset") })

    let approve: Promise<void>
    await act(async () => {
      approve = result.current.handleApproveRequest()
      await result.current.handleRejectRequest()
    })
    expect(mocks.approveBorrowRequest).toHaveBeenCalledTimes(1)
    expect(mocks.rejectBorrowRequest).not.toHaveBeenCalled()
    resolveApproval?.({ ...pending, status: "approved" })
    await act(async () => { await approve })
  })

  it("blocks approve followed immediately by cancel", async () => {
    let resolveApproval: ((value: typeof pending) => void) | undefined
    mocks.approveBorrowRequest.mockImplementation(
      () => new Promise((resolve) => { resolveApproval = resolve }),
    )
    const { result } = renderReadyState()
    await waitFor(() => expect(result.current.selectedRequest?.id).toBe(7))

    let approve: Promise<void>
    await act(async () => {
      approve = result.current.handleApproveRequest()
      await result.current.handleCancelRequest()
    })
    expect(mocks.approveBorrowRequest).toHaveBeenCalledTimes(1)
    expect(mocks.cancelBorrowRequest).not.toHaveBeenCalled()
    resolveApproval?.({ ...pending, status: "approved" })
    await act(async () => { await approve })
  })

  it("blocks reject followed immediately by cancel", async () => {
    let resolveRejection: ((value: typeof pending) => void) | undefined
    mocks.rejectBorrowRequest.mockImplementation(
      () => new Promise((resolve) => { resolveRejection = resolve }),
    )
    const { result } = renderReadyState()
    await waitFor(() => expect(result.current.selectedRequest?.id).toBe(7))
    await act(async () => { result.current.setReviewNote("wrong asset") })

    let reject: Promise<void>
    await act(async () => {
      reject = result.current.handleRejectRequest()
      await result.current.handleCancelRequest()
    })
    expect(mocks.rejectBorrowRequest).toHaveBeenCalledTimes(1)
    expect(mocks.cancelBorrowRequest).not.toHaveBeenCalled()
    resolveRejection?.({ ...pending, status: "rejected" })
    await act(async () => { await reject })
  })
})
