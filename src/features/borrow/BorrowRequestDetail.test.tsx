import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { BorrowState } from "./useBorrowState"
import { BorrowRequestDetail } from "./BorrowRequestDetail"

function request(status = "pending") {
  return {
    id: 7,
    requestKey: "BR-0007",
    submittedEmployeeId: "EE1001",
    submittedFullName: "Nguyen Van A",
    status,
    requestType: "return",
    assetCodes: ["ASSET-001"],
    submittedAt: "2026-08-06T00:00:00Z",
    decisionNote: null,
  }
}

function borrowState(status = "pending", overrides: Partial<BorrowState> = {}) {
  return {
    pendingRequests: [],
    selectedRequestId: 7,
    selectedRequest: request(status),
    reviewNote: "",
    setReviewNote: vi.fn(),
    queueMessage: "",
    isLoadingQueue: false,
    isLoadingDetail: false,
    isApproving: false,
    isRejecting: false,
    isCancelling: false,
    handleSelectRequest: vi.fn(),
    handleApproveRequest: vi.fn(),
    handleRejectRequest: vi.fn(),
    handleCancelRequest: vi.fn(),
    refreshQueue: vi.fn(),
    ...overrides,
  } as unknown as BorrowState
}

describe("BorrowRequestDetail actions", () => {
  it("shows Cancel only for pending requests and invokes it once", () => {
    const handleCancelRequest = vi.fn()
    render(<BorrowRequestDetail borrow={borrowState("pending", { handleCancelRequest })} />)

    const cancel = screen.getByRole("button", { name: "Cancel Request" })
    fireEvent.click(cancel)

    expect(handleCancelRequest).toHaveBeenCalledTimes(1)
  })

  it("disables review actions while cancellation is loading", () => {
    render(<BorrowRequestDetail borrow={borrowState("pending", { isCancelling: true })} />)

    expect((screen.getByRole("button", { name: "Cancelling..." }) as HTMLButtonElement).disabled).toBe(true)
    expect((screen.getByRole("button", { name: /Approve Return/ }) as HTMLButtonElement).disabled).toBe(true)
    expect((screen.getByRole("button", { name: /Reject Return/ }) as HTMLButtonElement).disabled).toBe(true)
  })

  it("does not show Cancel after a request leaves Pending", () => {
    render(<BorrowRequestDetail borrow={borrowState("approved")} />)

    expect(screen.queryByRole("button", { name: "Cancel Request" })).toBeNull()
  })
})
