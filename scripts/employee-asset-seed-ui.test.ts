import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const libSource = readFileSync("src-tauri/src/lib.rs", "utf8")
const apiSource = readFileSync("src/services/staff-api.ts", "utf8")
const appSource = readFileSync("src/App.tsx", "utf8")
const employeeViewSource = readFileSync("src/features/employees/EmployeeView.tsx", "utf8")
const seedStateSource = readFileSync(
    "src/features/employees/useEmployeeAssetSeedState.ts",
    "utf8",
)
const seedDrawerSource = readFileSync(
    "src/features/employees/EmployeeAssetSeedDrawer.tsx",
    "utf8",
)
const sharedShellSource = readFileSync("src/features/import/sharedImportShell.tsx", "utf8")

assert.match(libSource, /fn preview_employee_asset_seed\(/)
assert.match(libSource, /fn import_employee_asset_seed\(/)
assert.match(libSource, /preview_employee_asset_seed,/)
assert.match(libSource, /import_employee_asset_seed,/)

assert.match(apiSource, /previewEmployeeAssetSeed:/)
assert.match(apiSource, /importEmployeeAssetSeed:/)
assert.match(apiSource, /"preview_employee_asset_seed"/)
assert.match(apiSource, /"import_employee_asset_seed"/)
assert.match(apiSource, /snapshotId: payload\.snapshotId \?\? null/)

assert.doesNotMatch(appSource, /useEmployeeAssetSeedState/)
assert.doesNotMatch(appSource, /EmployeeAssetSeedDrawer/)
assert.doesNotMatch(appSource, /canSeedEmployeeAssets=\{/)
assert.doesNotMatch(appSource, /onOpenEmployeeAssetSeedDrawer=\{/)

assert.doesNotMatch(employeeViewSource, />\s*Seed Assets\s*</)
assert.doesNotMatch(employeeViewSource, /canSeedEmployeeAssets/)
assert.doesNotMatch(employeeViewSource, /onOpenEmployeeAssetSeedDrawer/)
assert.doesNotMatch(employeeViewSource, /Upload size=\{14\}/)

assert.match(seedStateSource, /staffGroupFilter === "employee_list"/)
assert.match(seedStateSource, /previewEmployeeAssetSeed/)
assert.match(seedStateSource, /importEmployeeAssetSeed/)
assert.match(seedStateSource, /snapshotId: nextPreview\.snapshotId/)
assert.match(seedStateSource, /reviewed snapshot/)

assert.match(seedDrawerSource, /Create Assets from Employee List/)
assert.match(seedDrawerSource, /Preview Employee Asset Import/)
assert.match(seedDrawerSource, /Stored Employee Computer Name/)
assert.match(seedDrawerSource, /Approve imports the reviewed snapshot, not a refreshed filter query\./)
assert.match(seedDrawerSource, /Excluded Rows/)

assert.match(sharedShellSource, /chooseButtonLabel\?: string/)
assert.match(sharedShellSource, /onChooseFiles\?: \(\) => void/)
assert.match(sharedShellSource, /const hasChooseAction =/)

console.log("employee-asset-seed UI tests passed")
