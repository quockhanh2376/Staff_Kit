import { render, screen } from "@testing-library/react"
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

describe("BorrowRequestDetail", () => {
  it("does not render review action buttons in the request detail", () => {
    render(<BorrowRequestDetail borrow={borrowState("pending")} />)

    expect(screen.queryByRole("button", { name: /Approve|Cancel|Reject/ })).toBeNull()
  })
})
