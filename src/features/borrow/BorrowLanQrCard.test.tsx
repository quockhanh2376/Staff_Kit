import { act, fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { SettingsState } from "../settings/useSettingsState"
import { BorrowLanQrCard } from "./BorrowLanQrCard"

function settings(overrides: Partial<SettingsState> = {}) {
  return {
    borrowLanQrUrl: "http://192.168.1.10:8787/borrow#t=token",
    borrowLanSettings: { enabled: true, host: "192.168.1.10", port: 8787, borrowUrl: "http://192.168.1.10:8787/borrow" },
    borrowLanHostInput: "192.168.1.10",
    borrowLanPortInput: "8787",
    setBorrowLanPortInput: vi.fn(),
    handleBorrowLanHostInputChange: vi.fn(),
    handleRefreshBorrowLanHost: vi.fn(),
    handleSaveBorrowLanSettings: vi.fn(),
    isDetectingBorrowLanHost: false,
    isSavingBorrowLanSettings: false,
    borrowLanDetectionNote: "",
    borrowLanMessage: "",
    lanServerAlive: true,
    lanServerStatus: { running: true, tokenReady: true, bindHost: "192.168.1.10", port: 8787 },
    lanAutoStartState: "ready",
    lanAutoStartError: null,
    isManagingLanToken: false,
    ensureBorrowLanReady: vi.fn(),
    handleStartBorrowLanServer: vi.fn(),
    handleStopBorrowLanServer: vi.fn(),
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
    expect(screen.getByText("QR Code")).toBeTruthy()
    expect(screen.getByText("LAN Address")).toBeTruthy()
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

    expect(screen.getAllByText("Starting…").length).toBe(2)
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
        onStartLan={retry}
      />,
    )

    screen.getByRole("button", { name: "Retry" }).click()
    expect(retry).toHaveBeenCalledTimes(1)
    expect(screen.queryByText(/127\.0\.0\.1|stack|Error:/i)).toBeNull()
  })

  it("keeps manual queue refresh inside the LAN readiness card", () => {
    const refreshQueue = vi.fn()
    render(<BorrowLanQrCard settings={settings()} isAdmin onRefreshQueue={refreshQueue} />)

    screen.getByRole("button", { name: "Refresh Queue" }).click()
    expect(refreshQueue).toHaveBeenCalledTimes(1)
    expect(screen.queryByText("Refresh Queue")).toBeNull()
  })

  it("renders QR Code, LAN Address, then Handle with Care as the top cards", () => {
    render(<BorrowLanQrCard settings={settings()} isAdmin onRefreshQueue={vi.fn()} />)

    const topCards = screen.getByTestId("lan-top-cards")
    expect(topCards.children).toHaveLength(3)
    expect(topCards.children[0]).toBe(screen.getByTestId("qr-code-card"))
    expect(topCards.children[1]).toBe(screen.getByTestId("lan-address-card"))
    expect(topCards.children[2]).toBe(screen.getByTestId("handle-with-care-card"))
    expect(screen.queryByText("Reachable address")).toBeNull()
    expect(screen.queryByText("Use the same Wi-Fi/LAN as the Staff Kit machine when scanning this QR.")).toBeNull()
    expect(screen.queryByText("LAN configuration and lifecycle controls are available in Settings.")).toBeNull()
  })

  it("uses a square QR card and responsive wide LAN address layout", () => {
    render(<BorrowLanQrCard settings={settings()} isAdmin />)

    const topCards = screen.getByTestId("lan-top-cards")
    const qrCard = screen.getByTestId("qr-code-card")
    const qrSurface = screen.getByTestId("qr-code-surface")
    const lanCard = screen.getByTestId("lan-address-card")

    expect(topCards.className).toContain("xl:grid-cols-[minmax(280px,320px)_minmax(280px,320px)_minmax(0,1fr)]")
    expect(topCards.className).toContain("xl:items-stretch")
    expect(qrCard.className).toContain("aspect-square")
    expect(qrSurface.className).toContain("aspect-square")
    expect(qrSurface.className).toContain("flex")
    expect(qrSurface.className).toContain("items-center")
    expect(qrSurface.className).toContain("justify-center")
    expect(qrSurface.className).toContain("max-w-[250px]")
    expect(lanCard.className).toContain("min-w-0")
    expect(lanCard.className).toContain("xl:aspect-square")
  })

  it("exposes Wi-Fi guidance as a keyboard-focusable address tooltip", () => {
    render(<BorrowLanQrCard settings={settings()} isAdmin />)

    const address = screen.getByLabelText("LAN address and Wi-Fi guidance")
    expect(address.getAttribute("title")).toBe("Use the same Wi-Fi/LAN as the Staff Kit machine when scanning this QR.")
    expect(address.getAttribute("tabindex")).toBe("0")
  })

  it("keeps LAN editing, detection, and saving inside the LAN Address card", () => {
    const detect = vi.fn()
    const save = vi.fn()
    const start = vi.fn()
    const stop = vi.fn()
    render(
      <BorrowLanQrCard
        settings={settings({ handleRefreshBorrowLanHost: detect, handleSaveBorrowLanSettings: save })}
        isAdmin
        onStartLan={start}
        onStopLan={stop}
      />,
    )

    expect(screen.getByLabelText("LAN Host / IP")).toBeTruthy()
    expect(screen.getByLabelText("Port")).toBeTruthy()
    fireEvent.click(screen.getByRole("button", { name: "Detect LAN IP" }))
    fireEvent.click(screen.getByRole("button", { name: "Save LAN settings" }))
    expect(detect).toHaveBeenCalledTimes(1)
    expect(save).toHaveBeenCalledTimes(1)
    expect(start).not.toHaveBeenCalled()
    expect(stop).not.toHaveBeenCalled()
  })

  it("renders a disabled stopped card with a start action", async () => {
    const start = vi.fn()
    render(
      <BorrowLanQrCard
        settings={settings({
          lanAutoStartState: "idle",
          lanServerAlive: false,
          lanServerStatus: { running: false, tokenReady: false, bindHost: "192.168.1.10", port: 8787 },
          borrowLanQrUrl: "",
        })}
        isAdmin
        onRefreshQueue={vi.fn()}
        onStartLan={start}
      />,
    )

    const card = screen.getByRole("button", { name: "Start Borrow/Return" })
    expect(card.getAttribute("title")).toBe("Start Borrow/Return")
    expect(screen.getByText("Start Borrow/Return")).toBeTruthy()
    expect(screen.queryByRole("img")).toBeNull()
    await act(async () => {
      fireEvent.click(card)
      fireEvent.click(card)
      await Promise.resolve()
    })
    expect(start).toHaveBeenCalledTimes(1)
  })

  it("stops from the active card and keeps Refresh isolated", async () => {
    const stop = vi.fn()
    const refresh = vi.fn()
    render(
      <BorrowLanQrCard
        settings={settings()}
        isAdmin
        onRefreshQueue={refresh}
        onStopLan={stop}
      />,
    )

    const card = screen.getByRole("button", { name: "Stop LAN" })
    expect(card.getAttribute("title")).toBe("Stop LAN")
    fireEvent.click(screen.getByRole("button", { name: "Refresh Queue" }))
    fireEvent.click(screen.getByRole("button", { name: "Refresh Queue" }))
    expect(refresh).toHaveBeenCalledTimes(2)
    await act(async () => {
      fireEvent.click(card)
      fireEvent.click(card)
      await Promise.resolve()
    })
    expect(stop).toHaveBeenCalledTimes(1)
  })

  it("shows starting and stopping states without a usable QR", () => {
    render(
      <BorrowLanQrCard
        settings={settings({
          lanAutoStartState: "starting",
          lanServerAlive: false,
          lanServerStatus: { running: false, tokenReady: false, bindHost: "192.168.1.10", port: 8787 },
          borrowLanQrUrl: "",
        })}
        isAdmin
        onStartLan={vi.fn()}
      />,
    )
    const card = screen.getByRole("button", { name: "Starting LAN" })
    expect(card.getAttribute("aria-disabled")).toBe("true")
    expect(screen.queryByRole("img")).toBeNull()
  })
})
