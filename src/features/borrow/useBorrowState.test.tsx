import { act, renderHook, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { useBorrowState } from "./useBorrowState"

const mocks = vi.hoisted(() => ({
  listPendingBorrowRequests: vi.fn(),
  getBorrowRequestDetail: vi.fn(),
  cancelBorrowRequest: vi.fn(),
}))
const setGlobalError = vi.fn()

vi.mock("../../services/staff-api", () => ({
  staffApi: {
    listPendingBorrowRequests: mocks.listPendingBorrowRequests,
    getBorrowRequestDetail: mocks.getBorrowRequestDetail,
    cancelBorrowRequest: mocks.cancelBorrowRequest,
    approveBorrowRequest: vi.fn(),
    rejectBorrowRequest: vi.fn(),
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
    mocks.cancelBorrowRequest.mockReset()
    mocks.listPendingBorrowRequests.mockResolvedValue([pending])
    mocks.getBorrowRequestDetail.mockResolvedValue(pending)
    mocks.cancelBorrowRequest.mockResolvedValue({ ...pending, status: "cancelled" })
  })

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
})
