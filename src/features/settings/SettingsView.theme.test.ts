import { readFileSync } from "node:fs"
import { resolve } from "node:path"
import { describe, expect, it } from "vitest"

const settingsView = readFileSync(resolve(process.cwd(), "src/features/settings/SettingsView.tsx"), "utf8")

describe("SettingsView theme tokens", () => {
    it("uses theme-aware neutral colors instead of dark-only palette classes", () => {
        for (const token of [
            "var(--bg)",
            "var(--surface)",
            "var(--surface-hover)",
            "var(--border)",
            "var(--text-primary)",
            "var(--text-secondary)",
        ]) {
            expect(settingsView).toContain(token)
        }

        for (const darkOnlyClass of [
            "bg-[#0d1117]",
            "bg-[#0f141b]",
            "bg-[#161b22]",
            "bg-[#1c2128]",
            "border-slate-700",
            "border-slate-800",
            "text-slate-100",
            "text-slate-300",
            "text-slate-400",
            "text-slate-500",
        ]) {
            expect(settingsView).not.toContain(darkOnlyClass)
        }
    })
})
