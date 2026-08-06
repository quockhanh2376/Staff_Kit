import { render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { SettingsState } from "../settings/useSettingsState"
import { BorrowLanQrCard } from "./BorrowLanQrCard"

function settings(overrides: Partial<SettingsState> = {}) {
  return {
    borrowLanQrUrl: "http://192.168.1.10:8787/borrow#t=token",
    borrowLanSettings: { enabled: true, host: "192.168.1.10", port: 8787, borrowUrl: "http://192.168.1.10:8787/borrow" },
    borrowLanHostInput: "192.168.1.10",
    borrowLanPortInput: "8787",
    borrowLanMessage: "",
    borrowLanDetectionNote: "",
    lanServerAlive: true,
    lanServerStatus: { running: true, tokenReady: true, bindHost: "192.168.1.10", port: 8787 },
    lanTokenReady: true,
    isManagingLanToken: false,
    isSavingBorrowLanSettings: false,
    isDetectingBorrowLanHost: false,
    handleBorrowLanEnabledChange: vi.fn(),
    handleBorrowLanHostInputChange: vi.fn(),
    handleRefreshBorrowLanHost: vi.fn(),
    handleSaveBorrowLanSettings: vi.fn(),
    handleIssueBorrowLanToken: vi.fn(),
    handleRevokeBorrowLanToken: vi.fn(),
    refreshBorrowLanStatus: vi.fn(),
    handleStartBorrowLanServer: vi.fn(),
    handleStopBorrowLanServer: vi.fn(),
    ...overrides,
  } as unknown as SettingsState
}

describe("BorrowLanQrCard lifecycle controls", () => {
  it("does not render LAN admin controls for a standard user", () => {
    const { container } = render(<BorrowLanQrCard settings={settings()} isAdmin={false} />)
    expect(container.textContent).toBe("")
  })

  it("shows running/token readiness separately for an admin", () => {
    render(<BorrowLanQrCard settings={settings()} isAdmin />)
    expect(screen.getByText(/LAN server active/i)).toBeTruthy()
    expect((screen.getByRole("button", { name: "Start LAN Server" }) as HTMLButtonElement).disabled).toBe(true)
    expect((screen.getByRole("button", { name: "Stop LAN Server" }) as HTMLButtonElement).disabled).toBe(false)
    expect((screen.getByRole("button", { name: "Regenerate QR Token" }) as HTMLButtonElement).disabled).toBe(false)
  })
})
