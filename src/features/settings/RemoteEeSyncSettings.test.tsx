import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"

const api = vi.hoisted(() => ({
    getRemoteEeSyncSettings: vi.fn(),
    saveRemoteEeSyncSettings: vi.fn(),
    removeRemoteEeSyncPassword: vi.fn(),
    testRemoteEeSyncConnection: vi.fn(),
}))
vi.mock("../../services/staff-api", () => ({ staffApi: api }))

import { RemoteEeSyncSettings } from "./RemoteEeSyncSettings"

beforeEach(() => {
    vi.clearAllMocks()
    api.getRemoteEeSyncSettings.mockResolvedValue({
        server: "sql.example",
        database: "AssetManagement",
        port: 1433,
        authType: "sql_server",
        username: "user_reader",
        approvedUsername: "user_reader",
        hasSavedPassword: true,
        environmentOverride: true,
    })
    api.saveRemoteEeSyncSettings.mockResolvedValue(undefined)
    api.testRemoteEeSyncConnection.mockResolvedValue({ status: "PASS", reason: "Query B completed.", rowCount: 12 })
})

describe("Config Sync settings collapsible panel", () => {
    it("is collapsed by default with Admin Portal style header and chevron", async () => {
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })
        expect(header).toBeTruthy()
        expect(header.tagName.toLowerCase()).toBe("button")
        expect(header.getAttribute("aria-expanded")).toBe("false")
        expect(header.getAttribute("aria-controls")).toBe("config-sync-panel")
        expect(header.getAttribute("id")).toBe("config-sync-header")
        expect(screen.getByText("Config Sync")).toBeTruthy()
        expect(screen.getByLabelText("Expand Config Sync")).toBeTruthy()

        const panel = document.getElementById("config-sync-panel")
        expect(panel).toBeTruthy()
        expect(panel?.className).toContain("invisible")

        const section = screen.getByLabelText("Config Sync")
        expect(section.className).toContain("rounded-[16px]")
        expect(section.className).toContain("shadow-[0_18px_42px_rgba(0,0,0,0.22)]")

        // Wait for settings to load
        await waitFor(() => expect(api.getRemoteEeSyncSettings).toHaveBeenCalled())
    })

    it("provides focus-visible styling on the header button", async () => {
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })
        expect(header.tagName.toLowerCase()).toBe("button")
        expect(header.className).toContain("focus-visible:ring-2")
        expect(header.className).toContain("focus-visible:ring-emerald-500/70")
        await waitFor(() => expect(api.getRemoteEeSyncSettings).toHaveBeenCalled())
    })

    it("expands and collapses when clicking header or chevron", async () => {
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })
        expect(header.getAttribute("aria-expanded")).toBe("false")

        // Click to expand
        fireEvent.click(header)
        expect(header.getAttribute("aria-expanded")).toBe("true")
        expect(screen.getByLabelText("Collapse Config Sync")).toBeTruthy()

        // Form elements are visible
        expect(await screen.findByLabelText("Server")).toBeTruthy()
        expect(screen.getByLabelText("SQL Username")).toBeTruthy()
        expect(screen.getByLabelText("Password")).toBeTruthy()

        // Click to collapse
        fireEvent.click(header)
        expect(header.getAttribute("aria-expanded")).toBe("false")
        expect(screen.getByLabelText("Expand Config Sync")).toBeTruthy()
    })

    it("toggles collapse/expand via Enter and Space keyboard navigation with ARIA updates", async () => {
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })
        const panel = document.getElementById("config-sync-panel")
        expect(header.getAttribute("aria-expanded")).toBe("false")
        expect(panel?.className).toContain("invisible")

        // Press Enter to expand
        fireEvent.keyDown(header, { key: "Enter" })
        expect(header.getAttribute("aria-expanded")).toBe("true")
        expect(panel?.className).toContain("visible")
        expect(await screen.findByLabelText("Server")).toBeTruthy()

        // Press Enter to collapse
        fireEvent.keyDown(header, { key: "Enter" })
        expect(header.getAttribute("aria-expanded")).toBe("false")
        expect(panel?.className).toContain("invisible")

        // Press Space to expand
        fireEvent.keyDown(header, { key: " " })
        expect(header.getAttribute("aria-expanded")).toBe("true")
        expect(panel?.className).toContain("visible")

        // Press Space to collapse
        fireEvent.keyDown(header, { key: " " })
        expect(header.getAttribute("aria-expanded")).toBe("false")
        expect(panel?.className).toContain("invisible")
    })

    it("preserves form state across collapse and expand cycles", async () => {
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })

        // Expand
        fireEvent.click(header)
        const serverInput = await screen.findByLabelText("Server") as HTMLInputElement
        const usernameInput = screen.getByLabelText("SQL Username") as HTMLInputElement
        const passwordInput = screen.getByLabelText("Password") as HTMLInputElement

        // Edit fields
        fireEvent.change(serverInput, { target: { value: "10.0.0.99" } })
        fireEvent.change(usernameInput, { target: { value: "rotated_user" } })
        fireEvent.change(passwordInput, { target: { value: "draft-secret" } })

        // Collapse
        fireEvent.click(header)
        expect(header.getAttribute("aria-expanded")).toBe("false")

        // Re-expand
        fireEvent.click(header)
        expect(header.getAttribute("aria-expanded")).toBe("true")

        // Form values remain preserved
        expect(serverInput.value).toBe("10.0.0.99")
        expect(usernameInput.value).toBe("rotated_user")
        expect(passwordInput.value).toBe("draft-secret")
    })

    it("supports Save and Test Connection after expanding", async () => {
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })
        fireEvent.click(header)

        const passwordInput = await screen.findByLabelText("Password")
        fireEvent.change(passwordInput, { target: { value: "synthetic-test-secret" } })

        fireEvent.click(screen.getByRole("button", { name: "Save Settings" }))
        await waitFor(() => expect(api.saveRemoteEeSyncSettings).toHaveBeenCalledWith(
            expect.objectContaining({
                server: "sql.example",
                database: "AssetManagement",
                port: 1433,
                username: "user_reader",
                approvedUsername: "user_reader",
            }),
            "synthetic-test-secret",
        ))

        fireEvent.click(screen.getByRole("button", { name: "Test Connection" }))
        const connectedBtn = await screen.findByRole("button", { name: "✓ Connected" })
        expect(connectedBtn).toBeTruthy()
        expect(connectedBtn.className).toContain("bg-emerald-600")
        expect(screen.queryByRole("status")).toBeNull()
    })

    it("supports admin account rotation directly through Save Settings", async () => {
        render(<RemoteEeSyncSettings isSuperAdmin />)
        const header = screen.getByRole("button", { name: /config sync/i })
        fireEvent.click(header)

        const usernameInput = await screen.findByLabelText("SQL Username")
        fireEvent.change(usernameInput, { target: { value: "new_reader" } })
        const passwordInput = screen.getByLabelText("Password")
        fireEvent.change(passwordInput, { target: { value: "new-password" } })

        fireEvent.click(screen.getByRole("button", { name: "Save Settings" }))
        await waitFor(() => expect(api.saveRemoteEeSyncSettings).toHaveBeenCalledWith(
            expect.objectContaining({ username: "new_reader", approvedUsername: "new_reader" }),
            "new-password",
        ))
        expect(await screen.findByText(/Approved SQL login updated and settings saved/i)).toBeTruthy()
    })

    it("shows failed reason without sending form credentials to Test Connection", async () => {
        api.testRemoteEeSyncConnection.mockResolvedValue({
            status: "FAIL",
            reason: "Read-only permissions could not be verified.",
            rowCount: null,
        })
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })
        fireEvent.click(header)

        await screen.findByLabelText("Password")
        fireEvent.click(screen.getByRole("button", { name: "Test Connection" }))
        expect(await screen.findByText(/FAIL/i)).toBeTruthy()
        expect(screen.getByText(/Read-only permissions could not be verified/i)).toBeTruthy()
        expect(api.testRemoteEeSyncConnection).toHaveBeenCalledWith()
    })

    it("displays error when save is rejected by backend without leaking credentials", async () => {
        api.saveRemoteEeSyncSettings.mockRejectedValueOnce("Only the approved Staff Kit SQL login is allowed.")
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })
        fireEvent.click(header)

        const usernameInput = await screen.findByLabelText("SQL Username")
        fireEvent.change(usernameInput, { target: { value: "aswhite-plus-webserver" } })
        fireEvent.click(screen.getByRole("button", { name: "Save Settings" }))

        expect(await screen.findByText("Only the approved Staff Kit SQL login is allowed.")).toBeTruthy()
    })

    it("renders Advanced Settings with Database and Port defaults", async () => {
        render(<RemoteEeSyncSettings />)
        fireEvent.click(screen.getByRole("button", { name: /config sync/i }))

        const advanced = await screen.findByText("Advanced Settings")
        expect(advanced).toBeTruthy()
        expect(screen.getByLabelText("Database")).toBeTruthy()
        expect(screen.getByLabelText("Port")).toBeTruthy()
    })

    it("resets Connected button state to Test Connection when server or credentials change", async () => {
        render(<RemoteEeSyncSettings />)
        const header = screen.getByRole("button", { name: /config sync/i })
        fireEvent.click(header)

        // Test connection successfully
        const testBtn = await screen.findByRole("button", { name: "Test Connection" })
        fireEvent.click(testBtn)
        expect(await screen.findByRole("button", { name: "✓ Connected" })).toBeTruthy()

        // Change server field
        const serverInput = screen.getByLabelText("Server")
        fireEvent.change(serverInput, { target: { value: "sql-updated.example" } })

        // Button should reset to Test Connection
        expect(screen.getByRole("button", { name: "Test Connection" })).toBeTruthy()
        expect(screen.queryByRole("button", { name: "✓ Connected" })).toBeNull()
    })
})
