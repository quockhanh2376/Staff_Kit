import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const columnsDrawer = readFileSync("src/features/columns/ColumnsDrawer.tsx", "utf8")
const teamView = readFileSync("src/features/teams/TeamView.tsx", "utf8")
const settingsView = readFileSync("src/features/settings/SettingsView.tsx", "utf8")

assert.match(columnsDrawer, /className="action-icon-button"/)
assert.match(columnsDrawer, /className="action-icon-button action-icon-button-danger"/)
assert.match(teamView, /className="action-icon-button"/)
assert.match(teamView, /PencilLine size=\{15\}/)
assert.match(teamView, /Trash2 size=\{15\}/)
assert.doesNotMatch(teamView, />\s*Rename\s*</)
assert.match(settingsView, /className="action-icon-button"/)
assert.match(settingsView, /UserRoundCheck size=\{15\}/)
assert.match(settingsView, /KeyRound size=\{15\}/)
assert.match(settingsView, /Trash2 size=\{15\}/)

console.log("action-icon-ui tests passed")
