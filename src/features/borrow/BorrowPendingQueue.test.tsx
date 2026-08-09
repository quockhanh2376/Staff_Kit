import { fireEvent, render, screen } from "@testing-library/react"
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
  it("renders one Pending Requests accordion card with compact summaries", () => {
    render(<BorrowPendingQueue borrow={borrowState()} />)

    expect(screen.getByText("Pending Requests (2)")).toBeTruthy()
    expect(screen.queryByText("Request Detail")).toBeNull()
    expect(screen.getByTestId("pending-request-summary-1").textContent).toContain("ASSET-001")
    expect(screen.getByTestId("pending-request-summary-2").textContent).toContain("2 assets")
    expect(screen.getByTestId("pending-request-summary-1").textContent).not.toContain("ASSET-002")
  })

  it("expands only the selected request and keeps staff identity in the summary only", () => {
    const borrow = borrowState({
      selectedRequestId: 1,
      selectedRequest: {
        ...borrowState().pendingRequests[0],
      },
    })
    render(<BorrowPendingQueue borrow={borrow} />)

    expect(screen.getByTestId("pending-request-details-1")).toBeTruthy()
    expect(screen.queryByTestId("pending-request-details-2")).toBeNull()
    expect(screen.getAllByText("EE1001")).toHaveLength(1)
    expect(screen.getAllByText("Borrower")).toHaveLength(1)
    expect(screen.getByText("Request ID")).toBeTruthy()
    expect(screen.getByLabelText("Rejection note")).toBeTruthy()
  })

  it("selects a request when its compact summary row is clicked", () => {
    const handleSelectRequest = vi.fn()
    render(<BorrowPendingQueue borrow={borrowState({ handleSelectRequest })} />)

    fireEvent.click(screen.getByTestId("pending-request-summary-2"))

    expect(handleSelectRequest).toHaveBeenCalledWith(2)
  })

  it("uses a responsive two-column request grid and keeps expansion independent from selection", () => {
    const borrow = borrowState({
      selectedRequestId: 1,
      selectedRequest: { ...borrowState().pendingRequests[0] },
    })
    render(<BorrowPendingQueue borrow={borrow} />)

    const grid = screen.getByTestId("pending-request-grid")
    expect(grid.className).toContain("grid")
    expect(grid.className).toContain("xl:grid-cols-2")
    expect(grid.className).toContain("grid-cols-1")

    fireEvent.click(screen.getByTestId("pending-request-toggle-2"))

    expect(screen.getByTestId("pending-request-details-1")).toBeTruthy()
    expect(screen.getByTestId("pending-request-details-2")).toBeTruthy()
    expect(screen.getByTestId("pending-request-row-1").getAttribute("data-selected")).toBe("true")
    expect(screen.getByTestId("pending-request-row-2").getAttribute("data-selected")).toBe("false")
  })

  it("uses the expanded textarea for missing rejection-note validation", () => {
    const borrow = borrowState({
      selectedRequestId: 1,
      selectedRequest: { ...borrowState().pendingRequests[0] },
      queueMessage: "A rejection note is required.",
    })
    render(<BorrowPendingQueue borrow={borrow} />)

    expect(document.activeElement).toBe(screen.getByLabelText("Rejection note"))
    expect(screen.getByText("A rejection note is required.")).toBeTruthy()
  })

  it("displays pending Borrow and Return requests with their asset counts", () => {
    render(<BorrowPendingQueue borrow={borrowState()} />)

    expect(screen.getByText("Pending Requests (2)")).toBeTruthy()
    expect(screen.getAllByText("Borrow").length).toBeGreaterThan(0)
    expect(screen.getAllByText("Return").length).toBeGreaterThan(0)
    expect(screen.getByText("2 assets")).toBeTruthy()
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
