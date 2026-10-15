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

describe("Remote EE Sync settings", () => {
    it("renders compact layout without technical description lines and hides port/auth-type", async () => {
        render(<RemoteEeSyncSettings />)
        expect(await screen.findByLabelText("Server")).toBeTruthy()
        expect(screen.getByLabelText("SQL Username")).toBeTruthy()
        expect(screen.getByLabelText("Password")).toBeTruthy()
        expect(screen.getByRole("button", { name: "Save Settings" })).toBeTruthy()
        expect(screen.getByRole("button", { name: "Test Connection" })).toBeTruthy()

        // Technical description lines removed
        expect(screen.queryByText(/Uses the approved Query B and blocks sync/i)).toBeNull()
        expect(screen.queryByText(/Save settings before testing\. Test Connection uses the saved profile/i)).toBeNull()

        // Approved account change panel removed
        expect(screen.queryByText("Approved SQL account change")).toBeNull()
        expect(screen.queryByLabelText("New approved SQL login")).toBeNull()

        // Authentication type hidden
        expect(screen.queryByLabelText("Authentication type")).toBeNull()

        // Database and Port in collapsed Advanced Settings
        const advanced = screen.getByText("Advanced Settings")
        expect(advanced).toBeTruthy()
        expect(screen.getByLabelText("Database")).toBeTruthy()
        expect(screen.getByLabelText("Port")).toBeTruthy()
    })

    it("never pre-fills the password and explains an active environment override", async () => {
        render(<RemoteEeSyncSettings />)
        const password = await screen.findByLabelText("Password") as HTMLInputElement
        expect(password.value).toBe("")
        expect(screen.getByText(/saved securely in Windows Credential Manager/i)).toBeTruthy()
        expect(screen.getByText(/STAFFKIT_MSSQL_\* environment override is active/i)).toBeTruthy()
    })

    it("sends password only to the save command and displays test PASS/count", async () => {
        render(<RemoteEeSyncSettings />)
        const password = await screen.findByLabelText("Password")
        fireEvent.change(password, { target: { value: "synthetic-test-secret" } })
        fireEvent.click(screen.getByRole("button", { name: "Save Settings" }))
        await waitFor(() => expect(api.saveRemoteEeSyncSettings).toHaveBeenCalledWith(
            expect.objectContaining({ server: "sql.example", database: "AssetManagement", port: 1433, username: "user_reader", approvedUsername: "user_reader" }),
            "synthetic-test-secret",
        ))
        fireEvent.click(screen.getByRole("button", { name: "Test Connection" }))
        const status = await screen.findByRole("status")
        expect(status.textContent).toContain("PASS")
        expect(screen.getByText(/12 source rows/i)).toBeTruthy()
    })

    it("supports admin account rotation directly through Save Settings", async () => {
        render(<RemoteEeSyncSettings isSuperAdmin />)
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

    it("shows a failed reason without sending form credentials to Test Connection", async () => {
        api.testRemoteEeSyncConnection.mockResolvedValue({ status: "FAIL", reason: "Read-only permissions could not be verified.", rowCount: null })
        render(<RemoteEeSyncSettings />)
        await screen.findByLabelText("Password")
        fireEvent.click(screen.getByRole("button", { name: "Test Connection" }))
        expect(await screen.findByText(/FAIL/i)).toBeTruthy()
        expect(screen.getByText(/Read-only permissions could not be verified/i)).toBeTruthy()
        expect(api.testRemoteEeSyncConnection).toHaveBeenCalledWith()
    })

    it("displays error when save is rejected by backend without leaking credentials", async () => {
        api.saveRemoteEeSyncSettings.mockRejectedValueOnce("Only the approved Staff Kit SQL login is allowed.")
        render(<RemoteEeSyncSettings />)
        const usernameInput = await screen.findByLabelText("SQL Username")
        fireEvent.change(usernameInput, { target: { value: "aswhite-plus-webserver" } })
        fireEvent.click(screen.getByRole("button", { name: "Save Settings" }))

        expect(await screen.findByText("Only the approved Staff Kit SQL login is allowed.")).toBeTruthy()
    })
})
