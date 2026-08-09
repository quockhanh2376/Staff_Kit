import { readFileSync } from "node:fs"
import { resolve } from "node:path"
import { describe, expect, it } from "vitest"

const settingsView = readFileSync(resolve(process.cwd(), "src/features/settings/SettingsView.tsx"), "utf8")

describe("SettingsView LAN controls", () => {
    it("does not render Borrow / Return LAN controls in Settings", () => {
        expect(settingsView).not.toContain('data-testid="settings-lan-controls"')
        for (const label of [
            "LAN host / IP",
            "Refresh LAN IP",
            "Save LAN settings",
            "Enable LAN server on app startup",
            "Advanced lifecycle and token controls",
            "Start LAN server",
            "Stop LAN server",
            "Regenerate QR token",
            "Revoke QR token",
        ]) {
            expect(settingsView).not.toContain(label)
        }
    })
})
