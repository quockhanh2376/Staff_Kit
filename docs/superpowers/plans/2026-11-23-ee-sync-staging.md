# EE List Sync Staging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** EE List sync loads HR data into a persisted, editable staging batch (conflicts never block). It writes exactly what the owner reviewed in one transaction, then deletes the batch.

**Architecture:** Two new tables, `employee_sync_batches` and `employee_sync_rows` (schema v4).
- The Rust logic lives in a new module `src-tauri/src/db/employee_sync_staging.rs` with pure `_conn`/`_tx` functions. Thin async wrappers in `mssql_import.rs` fetch from HR.
- New admin commands replace the in-memory preview/apply.
- The React drawer `EmployeeSyncDrawer.tsx` replaces the popover in `EmployeeView.tsx`.

**Tech Stack:** Tauri v2, Rust rusqlite/SQLCipher, React 19 + TypeScript, vitest.

**Spec:** `docs/superpowers/specs/2026-11-23-ee-sync-staging-design.md`

## Global Constraints

- All new commands use `auth_session::require_admin`. Each goes in `ADMIN_COMMANDS` in lib.rs, where the command-classification test must pass.
- Errors start with `Invalid`, `Not found` or `Cannot`.
- Row kinds: `new` | `changed` | `missing` | `conflict`. `unchanged` is a count only.
- Actions:
  - `new`/`changed`: `apply` (default) | `skip`
  - `conflict`: `move_to_employee_list` (default) | `skip`
  - `missing`: `move_to_offboarding` (default; `none` when `is_floating = 1`) | `none`
  - duplicate HR code: `skip` only
- Row `state`: `pending` | `stale`.
- After a successful Approve, and after a discard, the batch and all its rows are deleted. Only one audit row (`business.import.mssql`) remains.
- Approve rules:
  - All-or-nothing: if any row is stale, nothing is written.
  - The stale marks are kept, and the error is `Cannot apply: {n} rows changed in the app — review them`.
- Stale note text: `Changed in the app after this list was loaded — review again`.
- Duplicate note text: `Duplicate Staff Code in HR source`.
- Value normalisation (same as today):
  - trim every value;
  - email lowercased;
  - empty optional → NULL;
  - `full_name` required for any row that writes (`Invalid full name: required`).
- Mandatory safety snapshot `before_mssql_sync_apply` before Approve writes (`backup::create_mandatory_safety_snapshot`).
- Group moves go through `move_employees_group_tx`. Never copy the move SQL.
- Line endings: many files are CRLF or mixed. Keep each line's ending.
- Build env:
  - Strawberry Perl first on PATH (`export PATH="/c/Strawberry/perl/bin:$PATH"`).
  - Rust tests: `node ./scripts/run-with-shared-cargo-target.mjs cargo test --manifest-path src-tauri/Cargo.toml <filter>`.
  - `dist/` must exist.
- Known flaky tests: `backup::test_move_database_internal_disk_full_fails_closed`, RemoteEeSyncSettings, LoginPage version. `scripts/action-icon-ui.test.ts` may already fail.

## Review Focus

- A New row whose staff code was added locally (any group) after staging → stale, no duplicate insert. (Task 4 test `apply_new_row_code_added_locally_is_stale`.)
- Owner edits a name to only spaces on a row that writes → `Invalid full name: required` at update time, nothing saved. (Task 3 test `update_row_rejects_blank_full_name_for_writing_row`.)
- HR source has the same code twice → one row, kind `conflict`, skip only, never written. (Task 2 test `stage_duplicate_hr_code_is_single_skip_only_row`.)
- Pressing update while a batch is open → no silent loss: `Cannot load: an EE List sync is already open` unless `replace = true`. (Task 2 test `stage_refuses_when_open_batch_exists_unless_replace`.)
- Approve fails halfway, e.g. on an insert error → no employee writes, and the batch still exists. (Task 4 test `apply_error_rolls_back_and_keeps_batch`.)

---

### Task 1: Schema v4 tables + group-move `_tx`

**Files:**
- Modify: `src-tauri/src/db/schema.rs`. Add both `CREATE TABLE`s to the base schema.
- Modify: `src-tauri/src/db/mod.rs`:
  - `SCHEMA_USER_VERSION` 3 → 4;
  - an `ensure_employee_sync_tables(conn)` called from the migration path, following how `ensure_asset_model_tables` is wired;
  - update any test that hard-codes 3 to use the constant.
- Modify: `src-tauri/src/db/employee.rs`. Split `move_employees_group_conn` (L546) into `pub(crate) fn move_employees_group_tx(tx: &Transaction<'_>, payload: &MoveEmployeesGroupInput) -> Result<i64, String>`. The `_conn` function becomes: open tx → call `_tx` → commit.

**Table shapes:**
- `employee_sync_batches`:
  - `id INTEGER PRIMARY KEY`
  - `created_at TEXT NOT NULL DEFAULT (datetime('now'))`
  - `created_by TEXT`
  - `source_row_count INTEGER NOT NULL`
  - `unchanged_count INTEGER NOT NULL DEFAULT 0`
- `employee_sync_rows`:
  - `id INTEGER PRIMARY KEY`
  - `batch_id INTEGER NOT NULL REFERENCES employee_sync_batches(id) ON DELETE CASCADE`
  - `staff_code TEXT NOT NULL`
  - `kind TEXT NOT NULL CHECK (kind IN ('new','changed','missing','conflict'))`
  - `local_employee_id INTEGER`
  - `local_group TEXT`
  - `source_json TEXT NOT NULL DEFAULT '[]'`
  - `local_json TEXT`
  - `edited_json TEXT`
  - `action TEXT NOT NULL`
  - `notes_json TEXT NOT NULL DEFAULT '[]'`
  - `state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','stale'))`
  - `assets_held INTEGER NOT NULL DEFAULT 0`
  - `is_floating INTEGER NOT NULL DEFAULT 0`
- Index on `employee_sync_rows(batch_id)`.

- [ ] **Step 1: Failing tests:**
  - `apply_migrations_v3_db_gains_employee_sync_tables` (mod.rs): a v3 DB → both tables exist, `user_version == SCHEMA_USER_VERSION`.
  - `move_employees_group_tx_rolls_back_with_outer_tx` (employee.rs): move inside a tx, drop it without committing → the group is unchanged.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the filtered tests, then the full Rust suite. Expected: PASS.
- [ ] **Step 5: Commit** `feat(employees): staging tables for EE List sync`.

### Task 2: Staging a batch

**Files:**
- Create: `src-tauri/src/db/employee_sync_staging.rs` and register it in `db/mod.rs`.
- Modify: `src-tauri/src/db/mssql_import.rs`:
  - make `MssqlStaffRecord`, `compare_employee_list` and `normalized_staff_code` `pub(crate)` as needed;
  - add the async wrapper `stage_employee_list_sync(app, replace: bool)` that fetches HR (`fetch_employee_list_sync_source`) and calls the `_conn` fn.

**Interfaces:**
- Produces:
  - `#[serde(rename_all = "camelCase")] pub struct EmployeeSyncFields { pub full_name: String, pub nick_name: Option<String>, pub email: Option<String>, pub azure_ad_account: Option<String> }` (Serialize + Deserialize + Clone + PartialEq).
  - `pub struct EmployeeSyncStagedRow { pub id: i64, pub staff_code: String, pub kind: String, pub local_employee_id: Option<i64>, pub local_group: Option<String>, pub source: Vec<EmployeeSyncFields>, pub local: Option<EmployeeSyncFields>, pub edits: Option<EmployeeSyncFields>, pub action: String, pub notes: Vec<String>, pub state: String, pub assets_held: i64, pub is_floating: bool }` (camelCase).
  - `pub struct EmployeeSyncBatch { pub id: i64, pub created_at: String, pub counts: EmployeeSyncCounts, pub rows: Vec<EmployeeSyncStagedRow> }` (camelCase). `counts` uses the existing `EmployeeSyncCounts`.
  - `pub(crate) fn stage_employee_sync_batch_conn(conn: &mut Connection, source: &[MssqlStaffRecord], replace: bool, actor_ref: Option<&str>) -> Result<EmployeeSyncBatch, String>`
  - `pub(crate) fn load_open_employee_sync_batch_conn(conn: &Connection) -> Result<Option<EmployeeSyncBatch>, String>`
  - `pub(crate) fn discard_employee_sync_batch_conn(conn: &mut Connection, batch_id: i64) -> Result<(), String>` (missing batch → `Not found: EE List sync`)
- Local snapshot:
  - `local_json` stores `EmployeeSyncFields` plus `staffGroup` and `updatedAt` (the fingerprint), read from `employees` by `id`.
  - Conflict rows load the full local row for codes in other groups.
  - `assets_held` = count of active `asset_loans` (`returned_at IS NULL`) for that employee.
  - `is_floating` comes from `employees.is_floating`.

- [ ] **Step 1: Failing tests** (in-memory DB via the module's test helper; seed employees with `INSERT`):
  - `stage_with_conflicts_creates_batch_with_default_actions`:
    - Source: 1 new, 1 changed, 1 unchanged, 1 code in onboarding.
    - Local: 1 EE List employee missing from source, plus 1 floating EE List employee missing from source.
    - Expect: kinds and actions `apply`, `apply`, `move_to_employee_list`, `move_to_offboarding`, `none` (floating); counts.unchanged == 1; conflict row `local_group == Some("onboarding")` and `local.full_name` set.
  - `stage_missing_row_reports_assets_held`: a missing employee with 2 active loans → `assets_held == 2`.
  - `stage_duplicate_hr_code_is_single_skip_only_row`: source has code X twice → one row for X, kind `conflict`, action `skip`, `source.len() == 2`, notes contain `Duplicate Staff Code in HR source`.
  - `stage_refuses_when_open_batch_exists_unless_replace`:
    - second stage with `replace=false` → Err starting `Cannot load: an EE List sync is already open`;
    - with `replace=true` → Ok, and only one batch exists.
  - `discard_deletes_batch_and_rows`: both tables are empty afterwards.
  - `load_open_batch_round_trips_rows`.
- [ ] **Step 2: Run** `cargo test employee_sync_staging`. Expected: FAIL.
- [ ] **Step 3: Implement.** Reuse `compare_employee_list` for classification. Duplicate detection runs before classification: group the source by `normalized_staff_code`.
- [ ] **Step 4: Run** the filtered tests and the full Rust suite. Expected: PASS.
- [ ] **Step 5: Commit** `feat(employees): stage EE List sync into a persisted batch`.

### Task 3: Editing and refreshing rows

**Files:**
- Modify: `src-tauri/src/db/employee_sync_staging.rs`.

**Interfaces:**
- Produces:
  - `#[serde(rename_all = "camelCase")] pub struct EmployeeSyncRowUpdate { pub action: Option<String>, pub edits: Option<EmployeeSyncFields> }`. `edits: Some` replaces the stored edits.
  - `pub(crate) fn update_employee_sync_row_conn(conn: &mut Connection, row_id: i64, update: EmployeeSyncRowUpdate) -> Result<EmployeeSyncStagedRow, String>`
  - `pub(crate) fn refresh_employee_sync_row_conn(conn: &mut Connection, row_id: i64) -> Result<EmployeeSyncStagedRow, String>`
- Rules:
  - The action must be allowed for the row's kind (Global Constraints), else `Invalid action for {kind} row: {action}`.
  - Edits are normalised as in Global Constraints. A blank `full_name` is rejected when the row's effective action writes (`apply`/`move_to_employee_list`).
  - Missing row id → `Not found: EE List sync row {id}`.
  - Refresh:
    - re-read the local employee (by `local_employee_id`; for `new`, by staff code in any group);
    - rewrite `local_json`, `local_group`, `assets_held` and `is_floating`;
    - recompute `kind`, and reset `action` to that kind's default only if the kind changed;
    - keep edits;
    - set `state='pending'` and remove the stale note.

- [ ] **Step 1: Failing tests:**
  - `update_row_sets_action_and_edits`
  - `update_row_rejects_action_not_allowed_for_kind` (missing + `apply` → `Invalid`)
  - `update_row_rejects_blank_full_name_for_writing_row`
  - `update_duplicate_row_only_allows_skip`
  - `refresh_stale_row_resets_state_keeps_edits`
  - `refresh_new_row_whose_code_now_exists_becomes_changed_or_conflict`
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the filtered tests and the full Rust suite. Expected: PASS.
- [ ] **Step 5: Commit** `feat(employees): edit and refresh staged EE List rows`.

### Task 4: Approve + commands + remove old preview

**Files:**
- Modify: `src-tauri/src/db/employee_sync_staging.rs`:
  - `pub(crate) fn apply_employee_sync_batch_conn(conn: &mut Connection, batch_id: i64, actor_ref: Option<&str>) -> Result<EmployeeSyncApplyResult, String>`
  - `#[serde(rename_all = "camelCase")] pub struct EmployeeSyncApplyResult { pub added: usize, pub updated: usize, pub moved_to_employee_list: usize, pub offboarded: usize, pub skipped: usize, pub completed_at: String }`
- Modify: `src-tauri/src/db/mssql_import.rs`:
  - add async/app wrappers: `stage_employee_list_sync(app, replace)`, `get_open_employee_sync_batch(app)`, `update_employee_sync_row(app, row_id, update)`, `refresh_employee_sync_row(app, row_id)`, `discard_employee_sync_batch(app, batch_id)`, `apply_employee_sync_batch(app, batch_id)`. The apply wrapper takes the safety snapshot first, then calls `_conn`.
  - delete `preview_employee_list_sync`, `apply_employee_list_sync`, `PendingEmployeeListSync`, `PENDING_EMPLOYEE_LIST_SYNCS`, `apply_employee_list_diff_tx`, `next_employee_sync_preview_id` and `EmployeeListSyncPreview`;
  - `invalidate_pending_employee_list_syncs` (its 4 callers) now deletes any open batch through the connection its callers already hold. If a caller has no connection, open one; if none can be opened, skip silently, since the batch would be stale anyway.
- Modify: `src-tauri/src/lib.rs`:
  - replace the commands `preview_employee_list_mssql_sync` / `apply_employee_list_mssql_sync` with `stage_employee_list_sync(replace: bool)`, `get_open_employee_sync_batch`, `update_employee_sync_row(row_id, payload)`, `refresh_employee_sync_row(row_id)`, `discard_employee_sync_batch(batch_id)` and `apply_employee_sync_batch(batch_id)`;
  - all use `require_admin`; register them in `invoke_handler` and `ADMIN_COMMANDS`.
- Apply algorithm, in one transaction:
  1. Load the rows.
  2. Drift check every row whose action writes:
     - `new` is stale if its code exists in `employees` (any group);
     - `changed`/`conflict`/`missing` are stale if the employee is gone, or if its fields, group or `updated_at` differ from `local_json`.
  3. If any row is stale: mark those rows `stale` with the stale note, commit **only the marks**, and return the `Cannot apply: …` error.
  4. Otherwise write each row:
     - `new`: insert like today's `insert_employee_list_record`, using edits or else source.
     - `changed`: update like `update_employee_list_record`.
     - `conflict` + `move_to_employee_list`: `move_employees_group_tx(target employee_list)`, then update the fields.
     - `missing` + `move_to_offboarding`: `move_employees_group_tx(target offboarding)`.
  5. Write the audit row with the counts.
  6. Delete the batch (rows cascade, or delete them explicitly).
  7. Commit.
  8. Approve when no row writes → `Cannot apply: nothing to write`.

- [ ] **Step 1: Failing tests:**
  - `apply_writes_new_changed_with_edits_and_deletes_batch`
  - `apply_conflict_move_keeps_employee_id_and_loans`: lifecycle `active`, or `rehired` from offboarding, and the fields are updated.
  - `apply_missing_offboard_sets_resigned`
  - `apply_skipped_rows_write_nothing`
  - `apply_changed_row_edited_locally_after_staging_is_stale`: nothing is written, the row state is `stale`, and the batch still exists.
  - `apply_new_row_code_added_locally_is_stale`
  - `apply_error_rolls_back_and_keeps_batch`: force an insert error, e.g. a pre-existing unique violation inserted after the drift check is bypassed, or a CHECK violation via the edits path. No employee rows change and the batch remains.
  - `apply_nothing_to_write_is_refused`
  - lib.rs: the command-classification test passes with the new names.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** Remove the old code and fix its old tests: tests of `compare_employee_list` stay; tests of the removed apply are deleted or ported.
- [ ] **Step 4: Run** the full Rust suite. Expected: PASS.
- [ ] **Step 5: Commit** `feat(employees): approve staged EE List sync; drop in-memory preview`.

### Task 5: Frontend API + state

**Files:**
- Modify: `src/types/staff.ts`. Replace `EmployeeListSyncPreview`/`EmployeeListSyncResult` with `EmployeeSyncFields`, `EmployeeSyncStagedRow`, `EmployeeSyncBatch`, `EmployeeSyncRowUpdate` and `EmployeeSyncApplyResult`. They mirror the Rust camelCase shapes exactly.
- Modify: `src/services/staff-api.ts`. Remove `previewEmployeeListMssqlSync`/`applyEmployeeListMssqlSync` and add:
  - `stageEmployeeListSync(replace: boolean)`
  - `getOpenEmployeeSyncBatch()` → `EmployeeSyncBatch | null`
  - `updateEmployeeSyncRow(rowId, payload)`
  - `refreshEmployeeSyncRow(rowId)`
  - `discardEmployeeSyncBatch(batchId)`
  - `applyEmployeeSyncBatch(batchId)`
- Modify: `src/features/employees/useEmployeeState.ts`. Replace the mssql preview state with:
  - `syncBatch: EmployeeSyncBatch | null`
  - `isSyncBusy`
  - `syncError`
  - `startEmployeeSync()`: if an open batch exists, return `"open-exists"` so the UI asks; otherwise stage.
  - `replaceEmployeeSync()`
  - `resumeEmployeeSync()`
  - `updateSyncRow(rowId, update)`: optimistic merge of the returned row.
  - `refreshSyncRow(rowId)`
  - `discardEmployeeSync()`
  - `applyEmployeeSync()`: on success, clear the batch, refresh employees, and return the result.
  - `useEmployeeState` loads `getOpenEmployeeSyncBatch()` once, only for admins, so an open batch is offered on next start.
- Modify: `src/features/employees/useEmployeeState.mssql.test.tsx`. Update it to the new API.

- [ ] **Step 1: Failing tests:**
  - staff-api: each new method invokes its command with camelCase args (`replace`, `rowId`, `payload`, `batchId`).
  - hook:
    - `startEmployeeSync returns open-exists when a batch is open`
    - `applyEmployeeSync clears batch and refreshes employees`
    - `updateSyncRow replaces the row from the response`
- [ ] **Step 2: Run** `npx vitest run src/services src/features/employees`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same, plus `npx tsc -b`. Expected: PASS (EmployeeView may still need Task 6; keep it compiling by adapting the minimum).
- [ ] **Step 5: Commit** `feat(employees): staged EE List sync API and state`.

### Task 6: Sync drawer UI

**Files:**
- Create: `src/features/employees/EmployeeSyncDrawer.tsx` (uses `src/components/Drawer`), plus `EmployeeSyncDrawer.test.tsx`.
- Modify: `src/features/employees/EmployeeView.tsx`. The **update** button calls `startEmployeeSync`.
  - On `"open-exists"`, ask: `An EE List sync is already open. Continue it, or discard it and load a fresh list?` with the buttons **Continue** / **Load fresh** / Cancel.
  - The old popover (`aria-label="EE List sync preview"`) is removed. The drawer has `aria-label="EE List sync"`.
- Drawer content:
  - **Count chips:** New / Changed / Missing / Conflicts / Unchanged.
  - **Kind filter:** All, New, Changed, Missing, Conflicts.
  - **Table columns:**
    - Staff code
    - Kind
    - Local name
    - Source name
    - Full name / Nick / Email / Azure AD, as inputs prefilled from edits ?? source ?? local, saved on blur via `updateSyncRow`
    - Action, as a checkbox or select per kind using these labels:
      - `Apply`
      - `Skip`
      - `Move to EE List`
      - `Move to Offboarding`
      - `Keep`
    - Notes, which also show `Holds {n} assets` when `assetsHeld > 0`
  - Stale rows are highlighted (`search-highlight-cell`) and get a **Reload from app** button (`refreshSyncRow`).
  - Duplicate rows are read-only, with Skip only.
- Footer:
  - **Skip:** confirm `Discard this EE List sync? Nothing will be written.`, then `discardEmployeeSync`.
  - **Approve:**
    - The label shows totals, e.g. `Approve — Add 4 · Update 9 · Move 6 to EE List · Offboard 2`.
    - It is disabled when nothing writes or the batch is busy.
    - If any `missing` row with `move_to_offboarding` has `assetsHeld > 0`, confirm first: `{p} people being offboarded still hold {a} assets. Continue?`
    - On success, call `setGlobalNotice` with `EE List updated: added {added}, updated {updated}, moved {movedToEmployeeList}, offboarded {offboarded}.` and close.
    - On a stale error, show it in the drawer and keep it open; the rows come back stale after a reload of the batch.
- Theme: use `var(--…)` tokens for text, surfaces and borders, so it works in light and dark mode. No fixed `-200/-300` text colours.

- [ ] **Step 1: Failing tests** (`EmployeeSyncDrawer.test.tsx`):
  - `renders rows with default actions per kind`
  - `changing action saves the row`
  - `blur on edited name saves edits`
  - `approve label shows totals and is disabled when nothing writes`
  - `approve asks to confirm when offboarded people hold assets`
  - `stale row shows Reload from app and calls refresh`
  - `skip asks to confirm then discards`
  - EmployeeView: `update with an open batch asks continue or load fresh`
- [ ] **Step 2: Run** `npx vitest run src/features/employees`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the full `npx vitest run`, `npx tsc -b`, `npx eslint src/features/employees` and `npx vite build`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(employees): EE List sync staging drawer`, with a `daily_log.md` End entry in the same commit.
