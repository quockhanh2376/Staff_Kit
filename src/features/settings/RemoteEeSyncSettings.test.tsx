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
        username: "reader",
        hasSavedPassword: true,
        environmentOverride: true,
    })
    api.saveRemoteEeSyncSettings.mockResolvedValue(undefined)
    api.testRemoteEeSyncConnection.mockResolvedValue({ status: "PASS", reason: "Query B completed.", rowCount: 12 })
})

describe("Remote EE Sync settings", () => {
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
            expect.objectContaining({ server: "sql.example", database: "AssetManagement", username: "reader" }),
            "synthetic-test-secret",
        ))
        fireEvent.click(screen.getByRole("button", { name: "Test Connection" }))
        const status = await screen.findByRole("status")
        expect(status.textContent).toContain("PASS")
        expect(screen.getByText(/12 source rows/i)).toBeTruthy()
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
})
