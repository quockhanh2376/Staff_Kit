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
assert.doesNotMatch(settingsView, />\s*Import Excel\s*<\/div>/)
assert.doesNotMatch(
    settingsView,
    /Select one or multiple Excel files, then choose the columns before importing into app data\./,
)
assert.doesNotMatch(settingsView, /SQLite status:/)
assert.doesNotMatch(settingsView, /SQLite version:/)
assert.doesNotMatch(settingsView, /Manage local accounts in this app\. Column layout is saved per account profile\./)
assert.doesNotMatch(settingsView, /Point to a SharePoint \/ OneDrive synced folder to share data across the team\./)

console.log("action-icon-ui tests passed")
