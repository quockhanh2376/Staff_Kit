/**
 * Maps team identities to stable, accessible pill color classes.
 * High-contrast in both light and dark themes.
 * Unrecognized teams fall back to neutral styling.
 */

// Canonical recognizable team keys mapped to accessible tailwind color classes.
// Light mode text: 800/900 shade for WCAG AAA contrast against light surfaces.
// Dark mode text: 300 shade for WCAG AAA contrast against dark surfaces.
const KNOWN_TEAM_STYLES: Record<string, string> = {
  engineering:
    "border-emerald-500/30 bg-emerald-500/15 text-emerald-800 dark:border-emerald-500/30 dark:bg-emerald-500/15 dark:text-emerald-300",
  design:
    "border-purple-500/30 bg-purple-500/15 text-purple-800 dark:border-purple-500/30 dark:bg-purple-500/15 dark:text-purple-300",
  marketing:
    "border-amber-500/30 bg-amber-500/15 text-amber-900 dark:border-amber-500/30 dark:bg-amber-500/15 dark:text-amber-300",
  hr:
    "border-rose-500/30 bg-rose-500/15 text-rose-800 dark:border-rose-500/30 dark:bg-rose-500/15 dark:text-rose-300",
  "human resources":
    "border-rose-500/30 bg-rose-500/15 text-rose-800 dark:border-rose-500/30 dark:bg-rose-500/15 dark:text-rose-300",
  it:
    "border-cyan-500/30 bg-cyan-500/15 text-cyan-800 dark:border-cyan-500/30 dark:bg-cyan-500/15 dark:text-cyan-300",
  technology:
    "border-cyan-500/30 bg-cyan-500/15 text-cyan-800 dark:border-cyan-500/30 dark:bg-cyan-500/15 dark:text-cyan-300",
  sales:
    "border-blue-500/30 bg-blue-500/15 text-blue-800 dark:border-blue-500/30 dark:bg-blue-500/15 dark:text-blue-300",
  management:
    "border-indigo-500/30 bg-indigo-500/15 text-indigo-800 dark:border-indigo-500/30 dark:bg-indigo-500/15 dark:text-indigo-300",
  delivery:
    "border-teal-500/30 bg-teal-500/15 text-teal-800 dark:border-teal-500/30 dark:bg-teal-500/15 dark:text-teal-300",
  support:
    "border-lime-500/30 bg-lime-500/15 text-lime-900 dark:border-lime-500/30 dark:bg-lime-500/15 dark:text-lime-300",
  examworks:
    "border-fuchsia-500/30 bg-fuchsia-500/15 text-fuchsia-800 dark:border-fuchsia-500/30 dark:bg-fuchsia-500/15 dark:text-fuchsia-300",
  floating:
    "border-violet-500/30 bg-violet-500/15 text-violet-800 dark:border-violet-500/30 dark:bg-violet-500/15 dark:text-violet-300",
  zz_floating:
    "border-violet-500/30 bg-violet-500/15 text-violet-800 dark:border-violet-500/30 dark:bg-violet-500/15 dark:text-violet-300",
}

export const NEUTRAL_TEAM_STYLE =
  "border-slate-400/40 bg-slate-500/10 text-slate-700 dark:border-slate-600/40 dark:bg-slate-700/25 dark:text-slate-300"

export function normalizeTeamKey(teamName: string | null | undefined): string {
  if (!teamName) return ""
  return teamName.trim().toLowerCase()
}

export function isRecognizedTeam(teamName: string | null | undefined): boolean {
  const key = normalizeTeamKey(teamName)
  return Boolean(key && KNOWN_TEAM_STYLES[key])
}

export function getTeamPillClasses(teamName: string | null | undefined): string {
  const key = normalizeTeamKey(teamName)
  if (!key) return NEUTRAL_TEAM_STYLE
  return KNOWN_TEAM_STYLES[key] ?? NEUTRAL_TEAM_STYLE
}
