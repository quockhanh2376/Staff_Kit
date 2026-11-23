# Asset Condition + Shared Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Serialized assets get a condition (New / Spare / Broken) that IT controls, Broken can never be handed out, imports carry and highlight it, and admins export asset reports through one shared export module that the employee export also uses.

**Architecture:**
- **Backend:** a new `assets.condition` column. Transitions are centralized in `set_asset_status_tx`, hand-out guards sit at the 3 borrow points, and there is a new `update_asset_condition` command. Import gets a condition field plus review notes.
- **Frontend:** a shared `src/lib/tableExport.ts`. On top of the dashboard rows: a condition column, filter, cards and row selector, and an export dialog.
- **Delivery:** two PRs. PR 1 is Tasks 1–5 (backend and import). PR 2 is Tasks 6–8 (frontend) and is branched from main after PR 1 merges.

**Tech Stack:** Tauri v2, Rust (rusqlite/SQLCipher), React 19, TypeScript, vitest and Testing Library, `write-excel-file`.

**Spec:** `docs/superpowers/specs/2026-11-23-asset-condition-and-export-design.md`

## Global Constraints

- Condition values are stored as `new` | `spare` | `broken` and displayed as `New` | `Spare` | `Broken`. Old databases default to `spare`.
- Transition rules apply only to the targets `assigned` and `in_stock`:
  - on `assigned`: `new` → `spare`;
  - on `in_stock`: `spare` unless the asset is `broken`.
- Every hand-out path rejects broken assets with `Cannot assign broken asset <code>`. LAN search hides them.
- Who can change condition: admin and super admin (`require_admin`). Every change is audited with `asset.update_condition` (`changes.condition.before/after`, plus `source`: `manual` | `import`). No change means no write and no audit.
- Errors must start with `Invalid` / `Not found` / `Cannot` so the UI shows them.
- Import:
  - Status aliases: `status`, `condition`, `tinhtrang`, `trangthai`. Values are case-insensitive and trimmed.
  - Blank or unknown value → `spare`, plus a review note.
  - An existing asset gets **only its condition** updated, and only when the cell is non-blank.
- Export:
  - admin + super admin;
  - full data set;
  - file name `StaffKit_Assets_<Scope>_<YYYY-MM-DD>.<xlsx|csv>`;
  - Excel has the sheets `Summary` and `Assets`; CSV has Assets only.
- CSV formula guard: prefix `'` when a cell starts with `=`, `+`, `-`, `@`, `\t` or `\r`, except cells that match `^[+-]?[0-9][0-9 ().-]*$`.
- Line endings: many files are CRLF or mixed. Edit at byte level and keep each line's existing ending.
- Build environment: Strawberry Perl must come first on PATH for cargo (`export PATH="/c/Strawberry/perl/bin:$PATH"`). Rust tests: `node ./scripts/run-with-shared-cargo-target.mjs cargo test --manifest-path src-tauri/Cargo.toml <filter>`. A `dist/` folder must exist.
- Pre-existing failures that are not ours to fix:
  - `backup::test_move_database_internal_disk_full_fails_closed` is flaky under parallel runs;
  - `scripts/action-icon-ui.test.ts` already fails on main.

## Review Focus

- Re-importing the same inventory file twice → the second run changes nothing: no condition updates and no audit rows. (Task 5 test `reimport_same_file_is_noop_for_condition`.)
- An asset marked Broken while assigned, then returned through borrow → stays Broken. (Task 2 test.)
- `Broken`, `BROKEN ` or `hỏng`-style typos in the file → `Broken` is parsed; anything unknown → spare with a review note, never an error that blocks the whole import. (Task 5 test.)
- An export with a team filter and an in-stock scope → produces an empty Assets sheet with a valid Summary, no crash. (Task 8 test.)
- An employee CSV with phone `+84 912 345 678` and a name `=HYPERLINK(...)` → the phone is unchanged and the name gets a `'` prefix. (Task 6 test.)

---

## PR 1 — branch `feat/asset-condition` (Tasks 1–5)

### Task 1: Condition column, dashboard row fields

**Files:**
- Modify: `src-tauri/src/db/schema.rs`. Add `condition TEXT NOT NULL DEFAULT 'spare' CHECK (condition IN ('new','spare','broken'))` to `CREATE TABLE assets` (around L274).
- Modify: `src-tauri/src/db/mod.rs`. Append `("condition", "TEXT NOT NULL DEFAULT 'spare' CHECK (condition IN ('new','spare','broken'))")` to the `ALTER TABLE assets ADD COLUMN` list in `ensure_asset_model_tables` (around L798). Bump `SCHEMA_USER_VERSION` from 1 to 2 (L540).
- Modify: `src-tauri/src/db/asset.rs`:
  - `AssetDashboardSerializedRecord` (around L123) gains `pub condition: String` and `pub holder_team_name: Option<String>`. Fill them in the serialized dashboard SELECT with `a.condition` and the holder's `teams.name`, appended at the end of the column list.
  - `AssetRecord` gains `pub condition: String`, and `load_asset_record_by_id_conn` selects it.
- Test: `src-tauri/src/db/mod.rs` tests and `asset.rs` tests.

**Interfaces:**
- Produces: column `assets.condition`, `AssetDashboardSerializedRecord.condition`, `AssetDashboardSerializedRecord.holder_team_name`, `AssetRecord.condition`, and the Rust constants in `asset.rs`: `pub(crate) const ASSET_CONDITION_NEW: &str = "new"; ..._SPARE = "spare"; ..._BROKEN = "broken";`.

- [ ] **Step 1: Write failing tests.**
  - `apply_migrations_adds_spare_condition_to_existing_assets`: build a v1 DB (`assets` without `condition`, `PRAGMA user_version = 1`, one asset row), run `apply_migrations`, then assert `SELECT condition` == `"spare"` and `user_version` == 2.
  - `apply_migrations_fresh_db_has_condition_column` runs on an in-memory DB with no error.
  - `serialized_dashboard_rows_include_condition_and_holder_team`: seed an asset with an active loan to an employee in team "IT". Expect `condition == "spare"` and `holder_team_name == Some("IT")`.
- [ ] **Step 2: Run them.** Expected: FAIL (no such column, or missing field).
- [ ] **Step 3: Implement** the files above.
- [ ] **Step 4: Run** `cargo test` filtered to `apply_migrations` and `serialized_dashboard`. Expected: PASS. Then run the full Rust suite: all pass except the known flaky test.
- [ ] **Step 5: Commit** `feat(assets): add condition column (new/spare/broken)`.

### Task 2: Automatic transitions

**Files:**
- Modify: `src-tauri/src/db/asset.rs`, `set_asset_status_tx` (L262).
- Modify: `src-tauri/src/db/historical_assignment_import.rs`, L929 and L1212 (the direct `status = 'assigned'` updates).

**Interfaces:**
- Consumes: the Task 1 constants.
- Produces: unchanged signature `set_asset_status_tx(tx, asset_id, status)`. It now also updates `condition`, but only when `status` is `assigned` or `in_stock`.

- [ ] **Step 1: Failing tests** in `borrow.rs` tests (approve and return through the real flow):
  - `approving_borrow_of_new_asset_makes_it_spare`
  - `returning_spare_asset_keeps_spare`
  - `returning_broken_asset_keeps_broken` (set `condition='broken'` by SQL while it is assigned, then approve the return, then assert `broken`)
  - `retiring_asset_leaves_condition` (status `retired`, condition unchanged)

  In `historical_assignment_import.rs` tests:
  - `historical_assignment_marks_new_asset_spare`
- [ ] **Step 2: Run.** Expected: FAIL on the condition asserts.
- [ ] **Step 3: Implement.** `set_asset_status_tx` uses one UPDATE with `condition = CASE ...` per the Global Constraints. The two historical statements get the same `CASE`.
- [ ] **Step 4: Run** the borrow and historical tests, then the full suite. Expected: PASS.
- [ ] **Step 5: Commit** `feat(assets): condition follows hand-out and return`.

### Task 3: Hand-out guards

**Files:**
- Modify: `src-tauri/src/db/asset.rs`:
  - `search_in_stock_assets_conn` (L929): add `AND a.condition <> 'broken'`.
  - `AssetLookupRecord` and `load_asset_by_code_tx` / `load_asset_by_id_tx` gain `condition: String`.
- Modify: `src-tauri/src/db/borrow.rs`: the borrow-type check in submit (around L671) and in approval (around L1297).

**Interfaces:**
- Produces: `AssetLookupRecord.condition`. The error text is exactly `Cannot assign broken asset {asset_code}`.

- [ ] **Step 1: Failing tests:**
  - `lan_search_hides_broken_assets` (asset.rs)
  - `lan_borrow_submit_rejects_broken_asset`
  - `approve_borrow_rejects_broken_asset` (borrow.rs; the asset was broken after the request was submitted)
  - `return_of_broken_assigned_asset_is_allowed`
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** The guard runs only for `REQUEST_TYPE_BORROW`.
- [ ] **Step 4: Run** the filtered tests and the full suite. Expected: PASS.
- [ ] **Step 5: Commit** `feat(borrow): broken assets cannot be borrowed`.

### Task 4: `update_asset_condition` command, manual create condition

**Files:**
- Modify: `src-tauri/src/db/asset.rs`. Add the new fn after `update_asset_details_conn`.
- Modify: `src-tauri/src/db/asset_import.rs`. Add a wrapper next to `update_asset_details`. `create_asset_manually` honours the input condition.
- Modify: `src-tauri/src/lib.rs`:
  - add the command `update_asset_condition` with `require_admin`;
  - register it in `invoke_handler`;
  - add it to `ADMIN_COMMANDS`.
- Modify: `AssetUpsertInput` gains `#[serde(default)] pub condition: Option<String>` (manual create only; `upsert_assets` ignores it).

**Interfaces:**
- Produces:
  - `pub(crate) fn set_asset_condition_conn(conn: &mut Connection, asset_id: i64, condition: &str, source: &str, actor_ref: Option<&str>) -> Result<AssetRecord, String>` (source is `"manual"` or `"import"`);
  - `pub(crate) fn parse_asset_condition(raw: &str) -> Option<&'static str>` (case-insensitive and trimmed; returns `None` for blank or unknown);
  - `pub fn db::update_asset_condition(app, asset_id: i64, condition: String) -> Result<AssetRecord, String>`;
  - the Tauri command `update_asset_condition(session_token, asset_id: i64, condition: String) -> AssetRecord`.

- [ ] **Step 1: Failing tests** (asset.rs):
  - `set_asset_condition_audits_change` (spare→broken; one `asset.update_condition` row with `changes.condition.before == "spare"`, `after == "broken"`, `source == "manual"`)
  - `set_asset_condition_same_value_writes_nothing`
  - `set_asset_condition_rejects_invalid` (`"repair"` → Err starting `Invalid condition`)
  - `set_asset_condition_missing_asset_not_found`
  - `set_asset_condition_cannot_set_new_after_first_handout` (asset `spare` → `new` → Err starting `Cannot`; `new` → `new` is a no-op)
  - `parse_asset_condition_accepts_case_and_spaces` (`" BROKEN "` → broken, `"New"` → new, `""` → None, `"repair"` → None)
  - `create_asset_manually_conn_uses_input_condition` (`Some("new")` → new; `None` → spare)

  The lib.rs command classification test must still pass once the command is listed.
- [ ] **Step 2: Run.** Expected: FAIL (undefined).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the filtered and full suites. Expected: PASS.
- [ ] **Step 5: Commit** `feat(assets): admin condition update command`.

### Task 5: Import carries condition and review notes

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`:
  - add `CONDITION_ALIASES: &[&str] = &["status","condition","tinhtrang","trangthai"]`;
  - add `AssetImportFieldMapping.condition: Option<String>`;
  - add `AssetImportRowRecord`, `AssetImportRowState` and `AssetDirectImportPreviewRow` fields `condition: Option<String>` (parsed value) and `review_notes: Vec<String>`;
  - classification `Existing` keeps the row `skipped` for asset fields, but the import step applies the condition (below);
  - add `AssetDirectImportReport.condition_updated_rows: i64`.
- Modify: `src-tauri/src/db/mod.rs`. Add `("condition", "TEXT")` and `("review_notes_json", "TEXT NOT NULL DEFAULT '[]'")` to the `asset_import_rows` column list (around L910), and to `schema.rs` `CREATE TABLE asset_import_rows`.
- Modify: `src-tauri/src/db/historical_assignment_import.rs`. `HistoricalAssignmentItem` gains `#[serde(default)] pub current_condition: Option<String>` and `#[serde(default)] pub review_note: Option<String>`.

**Interfaces:**
- Consumes: `parse_asset_condition`, `set_asset_condition_conn` (with `source = "import"`), and the transition rules from Task 2.
- Produces the review note texts (the UI shows them verbatim):
  - `Condition will change: Spare → Broken` (format `Condition will change: {Old} → {New}`)
  - `Unknown status "{raw}" — imported as Spare`
  - `Blank status — imported as Spare` (new assets only)
  - `Assigned on import while Broken — check with IT`
  - `New with an owner — becomes Spare on hand-out`
  - historical: `Asset is Broken — check before assigning`

- [ ] **Step 1: Failing tests** (asset_import.rs, through `preview_asset_import_seed_conn` / `import_asset_import_seed_conn`):
  - `direct_import_new_rows_take_file_condition` (rows New/Spare/Broken)
  - `direct_import_unknown_and_blank_status_become_spare_with_note`
  - `direct_import_existing_asset_updates_only_condition` (existing asset name "Old"; the file has name "Other", status Broken → name stays "Old", condition broken, one audit with `source == "import"`, `condition_updated_rows == 1`)
  - `direct_import_existing_blank_status_keeps_condition`
  - `reimport_same_file_is_noop_for_condition`
  - `owner_row_broken_is_imported_with_note`
  - `owner_row_new_becomes_spare_with_note`
  - `preview_existing_condition_change_has_note`

  In historical_assignment_import.rs:
  - `historical_preview_flags_broken_asset` (`current_condition == Some("broken")`, `review_note` set, row still importable)
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the filtered and full suites. Expected: PASS.
- [ ] **Step 5: Commit** `feat(import): read asset condition and flag rows for IT review`, then push and open PR 1.

---

## PR 2 — branch `feat/asset-condition-ui` from main after PR 1 merges (Tasks 6–8)

### Task 6: Shared export module

**Files:**
- Create: `src/lib/tableExport.ts`, `src/lib/tableExport.test.ts`.
- Modify: `src/features/employees/employeeListExport.ts`. Keep `buildExportRows`, `buildCsv` and `buildXlsxSheetData` with their current signatures, built on `tableExport`, and remove the duplicated save and escape code. `EmployeeView.tsx`'s `handleExport` shows `Saved to {path}` through its existing status/notice mechanism (or `setGlobalError`-style banner if there is none; the builder picks, following the file).

**Interfaces:**
- Produces:
  - `export type ExportCell = string | number | null`
  - `export type ExportSheet = { name: string; headers: string[]; rows: ExportCell[][] }`
  - `export function escapeCsvCell(value: string): string` (quote rule plus the formula guard)
  - `export function buildCsv(sheet: ExportSheet): string` (BOM + CRLF, as today)
  - `export async function buildXlsxBlob(sheets: ExportSheet[]): Promise<Blob>` (numbers are written as numbers, a bold header row, one sheet per entry)
  - `export async function saveExport(blob: Blob, filename: string, ext: "csv" | "xlsx"): Promise<string | null>` (returns the saved path, or `null` if the user cancelled)

- [ ] **Step 1: Failing tests:**
  - `escapeCsvCell guards formulas but not phone numbers`: `=1+1` → `'=1+1`; `@SUM(A1)` → `'@SUM(A1)`; `+84 912 345 678` unchanged; `-5` unchanged; `a,b` → `"a,b"`
  - `buildCsv writes BOM and CRLF`
  - `saveExport returns null when dialog cancelled` (mock `@tauri-apps/plugin-dialog` save → null; `writeExportFile` not called)

  The existing `employeeListExport.test.ts` must stay green, unchanged apart from cells that now get the guard.
- [ ] **Step 2: Run** `npx vitest run src/lib/tableExport.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `npx vitest run src/lib src/features/employees` and `npx tsc -b`. Expected: PASS.
- [ ] **Step 5: Commit** `refactor(export): shared table export module with CSV formula guard`.

### Task 7: Condition in the dashboard, Edit drawer, manual create, import preview

**Files:**
- Modify: `src/types/staff.ts`:
  - `AssetCondition = "new" | "spare" | "broken"`
  - `AssetDashboardSerializedRecord` gains `condition: AssetCondition` and `holderTeamName: string | null`
  - `AssetRecord` gains `condition`
  - `AssetSeedItemInput` gains `condition?: AssetCondition`
  - the direct-import preview row gains `condition` and `reviewNotes: string[]`
  - `AssetDirectImportReport` gains `conditionUpdatedRows`
  - `HistoricalAssignmentItem` gains `currentCondition` and `reviewNote`
- Modify: `src/services/staff-api.ts`. Add `updateAssetCondition(assetId: number, condition: AssetCondition) => call<AssetRecord>("update_asset_condition", { assetId, condition })`. `createAssetManually` passes `condition`.
- Modify: `src/features/assets/useAssetDashboardState.ts`. Add `updateAssetCondition(assetId, condition): Promise<AssetRecord | null>`, `isSavingConditionAssetId: number | null`, and `conditionError: string | null` (the same pattern as `updateAssetDetails`).
- Modify: `src/features/assets/AssetDashboard.tsx`:
  - a `condition` column in `SERIALIZED_ASSET_COLUMN_MAP` (label `Condition`) that renders a `<select aria-label="Condition {assetCode}">` for `auth.isAdminAccount` and plain text otherwise;
  - a Condition filter (`All / New / Spare / Broken`) beside the category filter;
  - cards Total / Assigned / New / Spare / Broken computed from `serializedRows` (in-stock rows only for New/Spare/Broken).
  - `auth` Pick already has `isAdminAccount`.
- Modify: `src/features/assets/EditAssetDrawer.tsx`. Show Condition read-only in the `dl`.
- Modify: the manual create panel in `src/features/assets/AssetImportWizard.tsx` (`ManualAssetPanel` around L319) plus its state hook. Add a Condition select, `New` (default) / `Spare`.
- Modify: the direct-import preview table in `AssetImportWizard.tsx` / `useAssetDirectImportState.ts`. Add a Condition column. Rows with `reviewNotes.length > 0` get the class `search-highlight-cell` on the row, with the notes listed under the row. The report shows `conditionUpdatedRows`.
- Modify: `src/features/assets/HistoricalAssignmentImportModal.tsx`. Highlight rows that have a `reviewNote` and show the note.
- Test: `src/features/assets/AssetDashboard.condition.test.tsx` (copy the harness from `AssetDashboard.editAsset.test.tsx`), plus additions to `src/services/staff-api.test.ts`.

**Interfaces:**
- Consumes: the PR 1 command and fields.
- Produces: `assetDashboard.updateAssetCondition`, the `condition` column key, and the `CONDITION_LABELS: Record<AssetCondition, string>` export in `src/features/assets/assetCondition.ts` (also used by Task 8).

- [ ] **Step 1: Failing tests:**
  - `updateAssetCondition invokes update_asset_condition` (staff-api)
  - `admin sees condition selector and saving calls updateAssetCondition(3, "broken")`
  - `plain user sees condition text, no selector`
  - `New option is disabled for an assigned or spare asset`
  - `condition filter Broken shows only broken rows`
  - `cards count in-stock New/Spare/Broken and assigned` (rows: 1 new in stock, 2 spare in stock, 1 broken in stock, 1 assigned → 1/2/1, Assigned 1, Total 5)
  - `import preview highlights rows with review notes and shows the note text`
- [ ] **Step 2: Run** them. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the full vitest suite, `npx tsc -b`, and `npx eslint src/features/assets src/lib`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(assets): condition column, filter, cards and admin selector`.

### Task 8: Asset export dialog

**Files:**
- Create: `src/features/assets/assetExport.ts` and `src/features/assets/assetExport.test.ts`.
- Create: `src/features/assets/AssetExportDialog.tsx` (uses `Drawer`, or a small modal following `AssetMetadataUpdateModal`).
- Modify: `AssetDashboard.tsx`. Add an `Export` button (for `auth.isAdminAccount`) in the header button row that opens the dialog.

**Interfaces:**
- Consumes: `tableExport` (Task 6), `CONDITION_LABELS` (Task 7), and the dashboard rows.
- Produces:
  - `export type AssetExportScope = "all" | "assignable" | "spare" | "new" | "broken" | "assigned" | "currentFilter"`
  - `export type AssetExportOptions = { scope: AssetExportScope; categoryCode: string | null; teamName: string | null; teams: TeamRecord[] }`
  - `export function selectAssetExportRows(rows, filteredRows, options): Row[]`. Scopes `assignable`/`spare`/`new`/`broken` are in-stock only. `currentFilter` uses `filteredRows`. The team filter matches the holder team or its direct parent (the same rule as `employee.rs:951`).
  - `export function buildAssetExportSheets(rows, scope): ExportSheet[]` returns `[Summary, Assets]`.
    - Assets columns: `Asset code, Category, Name, Brand, Model, Serial, Computer name, Location, Loan status, Condition, Holder staff ID, Holder name, Holder team`. For scope `assigned`, the 3 holder columns come first.
    - Loan status text is `Assigned` / `In stock`.
    - Summary rows: `Total`, `Assigned`, `In stock — New`, `In stock — Spare`, `In stock — Broken` (numeric), then a per-category table with headers `Category, Total, Assigned, New, Spare, Broken`.
  - `export function assetExportFileName(scope, date: Date, ext): string` → `StaffKit_Assets_<ScopeLabel>_<YYYY-MM-DD>.<ext>`. Scope labels: `All, Assignable, Spare, New, Broken, Assigned, Filtered`.

- [ ] **Step 1: Failing tests** (`assetExport.test.ts`):
  - `assignable scope returns in-stock new and spare only`
  - `broken scope excludes assigned broken`
  - `assigned scope puts holder columns first`
  - `team filter matches holder team and child team`
  - `team filter with in-stock scope yields empty Assets and zero Summary`
  - `summary counts are numbers and per-category totals add up`
  - `file name uses scope label and date` (`StaffKit_Assets_Broken_2026-11-23.xlsx`)

  `AssetDashboard.condition.test.tsx`: `export button hidden for plain user; dialog Export calls saveExport with csv for CSV format` (mock `tableExport.saveExport`).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** The dialog fields are Scope (radio), Category (select from `categoryDetails`), Team (select from `staffApi.listTeams()`, loaded when the dialog opens) and Format (Excel/CSV). After a save, show `Saved to {path}` in the dialog; on `null`, close quietly.
- [ ] **Step 4: Run** the full vitest suite, `npx tsc -b`, eslint, and `npx vite build`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(assets): export dialog with scope, category and team filters`. Add an End entry to `daily_log.md` and update `optimization-backlog.md` (CSV formula guard fixed). Push, then open PR 2.
