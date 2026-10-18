import { describe, expect, it } from "vitest"
import {
  getTeamPillClasses,
  isRecognizedTeam,
  NEUTRAL_TEAM_STYLE,
  normalizeTeamKey,
} from "./teamColors"

describe("teamColors", () => {
  it("normalizes team keys predictably", () => {
    expect(normalizeTeamKey("  Engineering  ")).toBe("engineering")
    expect(normalizeTeamKey("DESIGN")).toBe("design")
    expect(normalizeTeamKey("")).toBe("")
    expect(normalizeTeamKey(null)).toBe("")
    expect(normalizeTeamKey(undefined)).toBe("")
  })

  it("identifies recognized vs unrecognized teams", () => {
    expect(isRecognizedTeam("Engineering")).toBe(true)
    expect(isRecognizedTeam("design")).toBe(true)
    expect(isRecognizedTeam("Marketing")).toBe(true)
    expect(isRecognizedTeam("HR")).toBe(true)
    expect(isRecognizedTeam("IT")).toBe(true)
    expect(isRecognizedTeam("Sales")).toBe(true)
    expect(isRecognizedTeam("Examworks")).toBe(true)
    expect(isRecognizedTeam("ZZ_Floating")).toBe(true)

    expect(isRecognizedTeam("Unknown Corp")).toBe(false)
    expect(isRecognizedTeam("Random Project Team")).toBe(false)
    expect(isRecognizedTeam("")).toBe(false)
    expect(isRecognizedTeam(null)).toBe(false)
    expect(isRecognizedTeam(undefined)).toBe(false)
  })

  it("assigns stable deterministic distinct color classes to recognized teams", () => {
    const engineering = getTeamPillClasses("Engineering")
    const design = getTeamPillClasses("Design")
    const marketing = getTeamPillClasses("Marketing")
    const hr = getTeamPillClasses("HR")
    const itTeam = getTeamPillClasses("IT")
    const sales = getTeamPillClasses("Sales")

    expect(engineering).toContain("emerald")
    expect(design).toContain("purple")
    expect(marketing).toContain("amber")
    expect(hr).toContain("rose")
    expect(itTeam).toContain("cyan")
    expect(sales).toContain("blue")

    // Stable regardless of case or whitespace
    expect(getTeamPillClasses("  ENGINEERING  ")).toBe(engineering)
    expect(getTeamPillClasses("marketing")).toBe(marketing)
  })

  it("assigns neutral color classes to unrecognized or empty teams", () => {
    expect(getTeamPillClasses("NonExistentTeam")).toBe(NEUTRAL_TEAM_STYLE)
    expect(getTeamPillClasses("Client XYZ")).toBe(NEUTRAL_TEAM_STYLE)
    expect(getTeamPillClasses("")).toBe(NEUTRAL_TEAM_STYLE)
    expect(getTeamPillClasses(null)).toBe(NEUTRAL_TEAM_STYLE)
    expect(getTeamPillClasses(undefined)).toBe(NEUTRAL_TEAM_STYLE)
  })

  it("provides accessible contrast classes for both light and dark themes", () => {
    const engineering = getTeamPillClasses("Engineering")
    // Light mode text should be dark (e.g. text-emerald-800)
    expect(engineering).toMatch(/text-[a-z]+-(800|900)/)
    // Dark mode text should be light (e.g. dark:text-emerald-300)
    expect(engineering).toMatch(/dark:text-[a-z]+-300/)

    // Neutral style should also have light/dark contrast
    expect(NEUTRAL_TEAM_STYLE).toContain("text-slate-700")
    expect(NEUTRAL_TEAM_STYLE).toContain("dark:text-slate-300")
  })
})
