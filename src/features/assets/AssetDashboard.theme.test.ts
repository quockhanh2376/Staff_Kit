import { describe, expect, it } from "vitest"
import { readFileSync } from "node:fs"
import { resolve } from "node:path"

describe("AssetDashboard theme and light mode compatibility contract", () => {
    const dashboardSource = readFileSync(
        resolve(process.cwd(), "src/features/assets/AssetDashboard.tsx"),
        "utf8",
    )
    const drawerSource = readFileSync(
        resolve(process.cwd(), "src/features/assets/AssetDashboardColumnsDrawer.tsx"),
        "utf8",
    )

    it("does not use hardcoded dark hex colors for backgrounds, borders, or text in AssetDashboard", () => {
        // Hardcoded dark backgrounds from previous bug
        expect(dashboardSource).not.toContain("#151921")
        expect(dashboardSource).not.toContain("#1c2128")
        expect(dashboardSource).not.toContain("#0d1117")
        expect(dashboardSource).not.toContain("#0b0f15")
        expect(dashboardSource).not.toContain("#11161f")
        expect(dashboardSource).not.toContain("#171c24")
        expect(dashboardSource).not.toContain("#1a202b")
        expect(dashboardSource).not.toContain("#202737")
        expect(dashboardSource).not.toContain("#222938")
        expect(dashboardSource).not.toContain("#252d3b")
        expect(dashboardSource).not.toContain("#273041")
        expect(dashboardSource).not.toContain("#283140")
        expect(dashboardSource).not.toContain("#31394a")
        expect(dashboardSource).not.toContain("#8f98a8")
        expect(dashboardSource).not.toContain("#8a93a4")
    })

    it("uses global CSS theme variables across shells, tables, cards, inputs and buttons", () => {
        expect(dashboardSource).toContain("var(--border)")
        expect(dashboardSource).toContain("var(--surface)")
        expect(dashboardSource).toContain("var(--surface-hover)")
        expect(dashboardSource).toContain("var(--bg)")
        expect(dashboardSource).toContain("var(--text-primary)")
        expect(dashboardSource).toContain("var(--text-secondary)")
        expect(dashboardSource).toContain("var(--table-header-bg)")
    })

    it("does not use hardcoded dark hex colors or slate classes in ColumnsDrawer", () => {
        expect(drawerSource).not.toContain("#283140")
        expect(drawerSource).not.toContain("#111722")
        expect(drawerSource).not.toContain("#151b26")
        expect(drawerSource).not.toContain("text-slate-100")
        expect(drawerSource).not.toContain("text-slate-400")
        expect(drawerSource).toContain("var(--border)")
        expect(drawerSource).toContain("var(--surface)")
        expect(drawerSource).toContain("var(--text-primary)")
        expect(drawerSource).toContain("var(--text-secondary)")
    })
})
