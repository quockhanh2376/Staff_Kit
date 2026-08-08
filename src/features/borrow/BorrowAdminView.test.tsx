import { render } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { ComponentProps } from "react"
import { BorrowAdminView } from "./BorrowAdminView"

vi.mock("./BorrowLanQrCard", () => ({ BorrowLanQrCard: () => null }))
vi.mock("./BorrowPendingQueue", () => ({ BorrowPendingQueue: () => null }))
vi.mock("./BorrowRequestDetail", () => ({ BorrowRequestDetail: () => null }))

describe("Borrow / Return LAN entry", () => {
  it("ensures the LAN server once when entering the operational view", () => {
    const ensureBorrowLanReady = vi.fn()
    const props = {
      auth: { isAdminAccount: true },
      borrow: { isLoadingQueue: false, refreshQueue: vi.fn(), selectedRequestId: null },
      settings: { lanServerAlive: false, ensureBorrowLanReady },
    } as unknown as ComponentProps<typeof BorrowAdminView>

    const { rerender } = render(<BorrowAdminView {...props} />)
    rerender(<BorrowAdminView {...props} />)

    expect(ensureBorrowLanReady).toHaveBeenCalledTimes(1)
  })
})
