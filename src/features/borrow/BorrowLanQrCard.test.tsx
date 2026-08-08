import { render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { SettingsState } from "../settings/useSettingsState"
import { BorrowLanQrCard } from "./BorrowLanQrCard"

function settings(overrides: Partial<SettingsState> = {}) {
  return {
    borrowLanQrUrl: "http://192.168.1.10:8787/borrow#t=token",
    borrowLanSettings: { enabled: true, host: "192.168.1.10", port: 8787, borrowUrl: "http://192.168.1.10:8787/borrow" },
    lanServerAlive: true,
    lanServerStatus: { running: true, tokenReady: true, bindHost: "192.168.1.10", port: 8787 },
    lanAutoStartState: "ready",
    lanAutoStartError: null,
    ensureBorrowLanReady: vi.fn(),
    ...overrides,
  } as unknown as SettingsState
}

describe("BorrowLanQrCard operational UX", () => {
  it("does not render the operational card for a standard user", () => {
    const { container } = render(<BorrowLanQrCard settings={settings()} isAdmin={false} />)
    expect(container.textContent).toBe("")
  })

  it("shows readiness, reachable address, and QR without lifecycle controls", () => {
    render(<BorrowLanQrCard settings={settings()} isAdmin />)

    expect(screen.getByText("Ready at 192.168.1.10:8787")).toBeTruthy()
    expect(screen.getByText("192.168.1.10:8787")).toBeTruthy()
    expect(screen.getByText("QR code")).toBeTruthy()
    for (const label of [
      "Regenerate QR token",
      "Start LAN server",
      "Stop LAN server",
      "Refresh status",
      "Revoke QR token",
      "Refresh LAN IP",
      "Save LAN settings",
      "Enable LAN server on app startup",
    ]) {
      expect(screen.queryByText(label)).toBeNull()
    }
  })

  it("does not render a usable QR before the server is ready", () => {
    render(
      <BorrowLanQrCard
        settings={settings({
          lanAutoStartState: "starting",
          lanServerAlive: false,
          lanServerStatus: { running: false, tokenReady: false, bindHost: "192.168.1.10", port: 8787 },
          borrowLanQrUrl: "http://192.168.1.10:8787/borrow#t=token",
        })}
        isAdmin
      />,
    )

    expect(screen.getByText("Starting LAN server…")).toBeTruthy()
    expect(screen.getByText("QR appears when the LAN server is ready.")).toBeTruthy()
    expect(screen.queryByText("Ready at 192.168.1.10:8787")).toBeNull()
  })

  it("shows a sanitized retry action after auto-start failure", () => {
    const retry = vi.fn()
    render(
      <BorrowLanQrCard
        settings={settings({
          lanAutoStartState: "error",
          lanAutoStartError: "LAN server could not start. Check the LAN host and port in Settings, then retry.",
          ensureBorrowLanReady: retry,
        })}
        isAdmin
      />,
    )

    screen.getByRole("button", { name: "Retry" }).click()
    expect(retry).toHaveBeenCalledTimes(1)
    expect(screen.queryByText(/127\.0\.0\.1|stack|Error:/i)).toBeNull()
  })
})
