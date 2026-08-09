import { render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { BorrowState } from "./useBorrowState"
import { BorrowPendingQueue } from "./BorrowPendingQueue"

function borrowState(overrides: Partial<BorrowState> = {}) {
  return {
    pendingRequests: [
      {
        id: 1,
        requestKey: "BR-0001",
        submittedEmployeeId: "EE1001",
        submittedFullName: "Borrower",
        status: "pending",
        requestType: "borrow",
        assetCodes: ["ASSET-001"],
        submittedAt: "2026-08-06T00:00:00Z",
        decisionNote: null,
      },
      {
        id: 2,
        requestKey: "BR-0002",
        submittedEmployeeId: "EE1002",
        submittedFullName: "Returner",
        status: "pending",
        requestType: "return",
        assetCodes: ["ASSET-002", "ASSET-003"],
        submittedAt: "2026-08-06T00:01:00Z",
        decisionNote: null,
      },
    ],
    selectedRequestId: null,
    selectedRequest: null,
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

describe("BorrowPendingQueue", () => {
  it("displays pending Borrow and Return requests with their asset counts", () => {
    render(<BorrowPendingQueue borrow={borrowState()} />)

    expect(screen.getByText("2 request(s) waiting for IT review.")).toBeTruthy()
    expect(screen.getAllByText("Borrow").length).toBeGreaterThan(0)
    expect(screen.getAllByText("Return").length).toBeGreaterThan(0)
    expect(screen.getByText("2 asset")).toBeTruthy()
  })

  it("shows compact review actions in the queue header and disables them without a selection", () => {
    const borrow = borrowState()
    render(<BorrowPendingQueue borrow={borrow} />)

    for (const label of ["Approve", "Cancel", "Reject"]) {
      expect((screen.getByRole("button", { name: label }) as HTMLButtonElement).disabled).toBe(true)
    }
    expect(screen.queryByText("Refresh Queue")).toBeNull()
  })

  it("invokes the selected request review action from the queue header", () => {
    const handleApproveRequest = vi.fn()
    const handleCancelRequest = vi.fn()
    const handleRejectRequest = vi.fn()
    render(
      <BorrowPendingQueue
        borrow={borrowState({
          selectedRequestId: 1,
          selectedRequest: {
            id: 1,
            requestKey: "BR-0001",
            submittedEmployeeId: "EE1001",
            submittedFullName: "Borrower",
            status: "pending",
            requestType: "borrow",
            assetCodes: ["ASSET-001"],
            submittedAt: "2026-08-06T00:00:00Z",
            decisionNote: null,
          },
          handleApproveRequest,
          handleCancelRequest,
          handleRejectRequest,
        })}
      />,
    )

    screen.getByRole("button", { name: "Approve" }).click()
    screen.getByRole("button", { name: "Cancel" }).click()
    screen.getByRole("button", { name: "Reject" }).click()
    expect(handleApproveRequest).toHaveBeenCalledTimes(1)
    expect(handleCancelRequest).toHaveBeenCalledTimes(1)
    expect(handleRejectRequest).toHaveBeenCalledTimes(1)
  })
})
