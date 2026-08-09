import { act, render, screen } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"
import type { ComponentProps } from "react"
import { BorrowAdminView } from "./BorrowAdminView"
import { BORROW_REVIEW_DESCRIPTION_TYPOGRAPHY } from "./borrowReviewCopy"

vi.mock("./BorrowLanQrCard", () => ({ BorrowLanQrCard: (props: { onRefreshQueue?: () => void }) => (
  <button type="button" aria-label="Refresh pending queue" onClick={props.onRefreshQueue} />
) }))
vi.mock("./BorrowPendingQueue", () => ({ BorrowPendingQueue: () => null }))
vi.mock("./BorrowRequestDetail", () => ({ BorrowRequestDetail: () => null }))

describe("Borrow / Return LAN entry", () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it("does not auto-start the LAN server when entering the operational view", () => {
    const ensureBorrowLanReady = vi.fn()
    const props = {
      auth: { isAdminAccount: true },
      borrow: { isLoadingQueue: false, refreshQueue: vi.fn(), selectedRequestId: null },
      settings: { lanServerAlive: false, ensureBorrowLanReady },
    } as unknown as ComponentProps<typeof BorrowAdminView>

    const { rerender } = render(<BorrowAdminView {...props} />)
    rerender(<BorrowAdminView {...props} />)

    expect(ensureBorrowLanReady).not.toHaveBeenCalled()
  })

  it("shares the page-description typography contract with policy textareas", () => {
    const props = {
      auth: { isAdminAccount: true },
      borrow: { isLoadingQueue: false, refreshQueue: vi.fn(), selectedRequestId: null },
      settings: { lanServerAlive: false, ensureBorrowLanReady: vi.fn() },
    } as unknown as ComponentProps<typeof BorrowAdminView>

    render(<BorrowAdminView {...props} />)

    expect(screen.getByText("Employees scan the fixed LAN QR on their phone, submit a pending borrow or return request, then IT reviews the exact asset items here.").className)
      .toContain(BORROW_REVIEW_DESCRIPTION_TYPOGRAPHY)
  })

  it("fetches immediately, polls while mounted, and cleans up on unmount", async () => {
    vi.useFakeTimers()
    const refreshQueue = vi.fn().mockResolvedValue(false)
    const props = {
      auth: { isAdminAccount: true },
      borrow: { isLoadingQueue: false, refreshQueue, selectedRequestId: null },
      settings: {
        lanServerAlive: true,
        lanAutoStartState: "ready",
        lanServerStatus: { running: true },
        borrowLanQrUrl: "http://192.168.1.10:8787/borrow#t=token",
        ensureBorrowLanReady: vi.fn(),
      },
    } as unknown as ComponentProps<typeof BorrowAdminView>

    const view = render(<BorrowAdminView {...props} />)
    expect(refreshQueue).toHaveBeenCalledTimes(1)

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2999)
    })
    expect(refreshQueue).toHaveBeenCalledTimes(1)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(refreshQueue).toHaveBeenCalledTimes(2)

    view.unmount()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000)
    })
    expect(refreshQueue).toHaveBeenCalledTimes(2)
  })

  it("backs off to ten seconds after two quiet minutes and returns to three seconds on change", async () => {
    vi.useFakeTimers()
    const refreshQueue = vi.fn().mockResolvedValue(false)
    const props = {
      auth: { isAdminAccount: true },
      borrow: { isLoadingQueue: false, refreshQueue, selectedRequestId: null },
      settings: {
        lanServerAlive: true,
        lanAutoStartState: "ready",
        lanServerStatus: { running: true },
        borrowLanQrUrl: "http://192.168.1.10:8787/borrow#t=token",
        ensureBorrowLanReady: vi.fn(),
      },
    } as unknown as ComponentProps<typeof BorrowAdminView>

    render(<BorrowAdminView {...props} />)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(120000)
    })
    const quietPollCount = refreshQueue.mock.calls.length
    await act(async () => {
      await vi.advanceTimersByTimeAsync(9999)
    })
    expect(refreshQueue).toHaveBeenCalledTimes(quietPollCount)
    refreshQueue.mockResolvedValueOnce(true)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    const changedPollCount = refreshQueue.mock.calls.length
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2999)
    })
    expect(refreshQueue).toHaveBeenCalledTimes(changedPollCount)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(refreshQueue).toHaveBeenCalledTimes(changedPollCount + 1)
  })

  it("polls the desktop queue even while the LAN server is stopped", async () => {
    vi.useFakeTimers()
    const refreshQueue = vi.fn().mockResolvedValue(false)
    const props = {
      auth: { isAdminAccount: true },
      borrow: { isLoadingQueue: false, refreshQueue, selectedRequestId: null },
      settings: {
        lanServerAlive: false,
        lanAutoStartState: "starting",
        lanServerStatus: { running: false, tokenReady: false, bindHost: "", port: 8787 },
        ensureBorrowLanReady: vi.fn(),
      },
    } as unknown as ComponentProps<typeof BorrowAdminView>

    const view = render(<BorrowAdminView {...props} />)
    expect(refreshQueue).toHaveBeenCalledTimes(1)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000)
    })
    expect(refreshQueue).toHaveBeenCalledTimes(2)
    view.unmount()
  })

  it("places manual queue refresh in the LAN readiness card", () => {
    const refreshQueue = vi.fn().mockResolvedValue(undefined)
    const props = {
      auth: { isAdminAccount: true },
      borrow: { isLoadingQueue: false, refreshQueue, selectedRequestId: null },
      settings: { lanServerAlive: false, ensureBorrowLanReady: vi.fn() },
    } as unknown as ComponentProps<typeof BorrowAdminView>

    render(<BorrowAdminView {...props} />)
    screen.getByRole("button", { name: "Refresh pending queue" }).click()
    expect(refreshQueue).toHaveBeenCalledTimes(2)
  })
})
