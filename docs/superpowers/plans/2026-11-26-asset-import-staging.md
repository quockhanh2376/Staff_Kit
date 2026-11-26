# Asset Import Staging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Serialized asset import loads into the existing staging batch. The owner edits and skips rows there, then approves all-or-nothing. The file's serial wins over the app's.

**Architecture:** Reuse `asset_import_batches` / `asset_import_rows` and their commands in `src-tauri/src/db/asset_import.rs`. The backend gets dash-insensitive serial matching, a serial update for existing assets, and a strict approve that deletes the batch. The frontend gets `AssetImportStagingDrawer.tsx`, opened from the wizard's serialized "Preview Import".

**Tech Stack:** Rust + rusqlite (SQLCipher), React 19, vitest.

**Spec:** `docs/superpowers/specs/2026-11-26-asset-import-staging-design.md`

## Global Constraints
- New error strings start with `Invalid`, `Not found` or `Cannot`.
- Keep each file's existing line endings.
- No new Tauri command, so `ADMIN_COMMANDS` is unchanged.
- The quantity import path and the direct `import_asset_import_file` path keep their current partial-import behaviour.
- The UI must work in light and dark mode: use `var(--…)` tokens only.

## Review Focus
- A serial that differs only by `-`, spaces or case (`PF-5MBXRW` vs `PF5MBXRW`): the row is Existing, with no note and no update.
- The file serial already belongs to a different existing asset: the row stays an error. Covered by the existing `(Some, Some) id differs` branch; a test pins it.
- A row edited into an error between load and Approve: Approve returns `Cannot approve…` and writes nothing.
- Two rows in the file whose serials differ only by a dash: flagged as a duplicate.
- Reopening the wizard with a pending batch: the owner can continue or discard it, and loading a new file deletes the old pending batches.

---

### Task 1: Serial rules (backend)

**Files:** Modify `src-tauri/src/db/asset_import.rs` (classification ~3300-3390, identities ~3280-3296, revalidate ~2994-3100, commit ~1061-1415, preview ~1452). Tests go in the same file's `mod tests`.

**Interfaces:**
- Produces:
  - `fn normalize_serial_key(value: &str) -> String`: uppercases and drops `-` and whitespace.
  - `SerializedAssetImportClassification::Existing { asset_id, condition, serial_update: Option<String> }`.
  - `revalidate_batch_tx` returns `RevalidateOutcome { condition_updates: Vec<(i64, &'static str)>, serial_updates: Vec<(i64, String)> }`.
  - Review notes: `EXISTING_ASSET_NOTE = "Already in Staff Kit"` and `"Serial will update: {old} → {new}"`.

- [ ] Test `serial_differing_only_by_dash_is_existing_without_update`: seed asset `VNLAP515` with serial `PF-5MBXRW`; stage a row with the same code and serial `PF5MBXRW`. Assert row status `skipped`, no errors, no "Serial will update" note.
- [ ] Test `file_serial_wins_for_existing_code`: seed `VNLAP204` / `HK4FMG3`; stage `VNLAP204` / `HK4MFG3`. Assert status `skipped`, `review_notes` contains `Serial will update: HK4FMG3 → HK4MFG3`. After `import_asset_import_batch_valid_rows_conn`, assert `assets.serial_number = 'HK4MFG3'`.
- [ ] Test `file_serial_owned_by_other_asset_stays_conflict`: seed A1 / S1 and A2 / S2; stage A1 / S2. Assert status `error`.
- [ ] Run `cargo test … asset_import::tests::serial` and confirm the tests fail.
- [ ] Implement:
  - Use `normalize_serial_key` for the `by_serial` keys, the classify `serial_key`, `serial_matches_existing` and the duplicate serial counts.
  - The `(Some(code), None)` branch with a differing serial becomes `Existing` with `serial_update = row.serial_number`. Also set it when the existing serial is `None` and the row has one.
  - Revalidate's Existing branch pushes `EXISTING_ASSET_NOTE`, plus the serial note when `serial_update` is set, and collects `serial_updates`.
  - The commit applies `UPDATE assets SET serial_number = ?, updated_at = datetime('now') WHERE id = ?` inside the transaction, before new assets are created, and adds `serialUpdates` to the audit payload.
  - The preview counts `serial_updates` into `condition_update_rows`' sibling: add `serial_update_rows: i64` to `AssetDirectImportPreview`.
- [ ] Run the asset_import tests. All pass.
- [ ] Commit `feat(assets): file serial wins on import; ignore dashes when matching`.

### Task 2: Strict approve that deletes the batch (backend)

**Files:** `src-tauri/src/db/asset_import.rs`, tests in the same file.

**Interfaces:**
- Produces: `pub(crate) fn approve_asset_import_batch_conn(conn: &mut Connection, batch_id: i64) -> Result<AssetImportCommitResult, String>`. The public `import_asset_import_batch_valid_rows(app, batch_id)` (snapshot, then commit) now calls it.
- The existing `import_asset_import_batch_valid_rows_conn` keeps its behaviour. It becomes a wrapper over `commit_batch_rows_conn(conn, batch_id, strict: bool)`.

- [ ] Test `approve_refuses_when_errors_remain`: a batch with one valid row and one error row. Assert `Err` starting with `Cannot approve`, no asset created, batch still present.
- [ ] Test `approve_imports_and_deletes_batch`: a batch with valid rows only. Assert the asset is created, `asset_import_batches` has 0 rows for that id, and the `asset_import.import_valid_rows` audit row exists.
- [ ] Implement:
  - In `commit_batch_rows_conn`, after the in-transaction revalidate: if `strict && summary.error_rows > 0`, return `Err(format!("Cannot approve: {n} rows still have errors"))`, which rolls the transaction back.
  - Strict mode deletes the batch row (rows cascade) before `tx.commit()`.
- [ ] Run the tests and commit `feat(assets): approve staged asset import all-or-nothing`.

### Task 3: Staging drawer (frontend)

**Files:**
- Create `src/features/assets/AssetImportStagingDrawer.tsx` and `src/features/assets/AssetImportStagingDrawer.test.tsx`.
- Modify:
  - `src/features/assets/useAssetDirectImportState.ts`;
  - `src/features/assets/AssetImportWizard.tsx`;
  - `src/types/staff.ts`, where `AssetImportBatchDetail` is flattened to match Rust `#[serde(flatten)]`, and `serialUpdateRows` is added to the preview type;
  - `src/App.tsx`, to mount the drawer.

**Interfaces (added to `AssetImportState`):**
- `stagingBatch: AssetImportBatchDetail | null`
- `pendingStagingBatch: AssetImportBatchSummary | null`
- `isStagingBusy: boolean`
- `stagingError: string | null`
- `loadStaging(): Promise<void>`: deletes every `pending_review` batch, then `createAssetImportBatch` for the selected file.
- `continueStaging(): Promise<void>`
- `updateStagingRow(rowId: number, fieldKey: string, value: string | null): Promise<void>`
- `setStagingRowSkipped(rowId: number, skipped: boolean): Promise<void>`
- `approveStaging(): Promise<boolean>`
- `discardStaging(): Promise<boolean>`
- Each mutation reloads the detail with `getAssetImportBatchDetail`, because a single edit can change other rows.

- [ ] Tests (vitest, mock `staffApi`):
  - rows render with editable Asset code / Serial / Model / Location / Condition;
  - the error reason shows and Approve is disabled while `errorRows > 0`;
  - the "Serial will update" note renders;
  - an Existing row has its Skip checkbox disabled;
  - Discard confirms, then calls `deleteAssetImportBatch`;
  - Approve calls `importAssetImportBatchValidRows`.
- [ ] Implement the drawer, following `EmployeeSyncDrawer.tsx`:
  - Columns: Row, Asset code, Name, Computer name, Brand, Model, Serial, Location, Condition (select from `ASSET_CONDITIONS`), Note, Skip, Status / notes.
  - Error rows get the `search-highlight-cell` class.
  - Filter chips: All, New, Existing, Errors, Skipped.
  - Approve label: `Approve — {new} new · {serialUpdates} serial updates`.
  - Discard confirm text: `Discard this staged import? Nothing will be written.`
- [ ] In the wizard's serialized mode, the primary action becomes "Load to staging" (calls `loadStaging`). When a pending batch exists, a banner offers Continue / Discard. Quantity mode is unchanged.
- [ ] Run `npx vitest run src/features/assets` and `npx tsc --noEmit`, then commit.

### Task 4: Verify and ship
- [ ] Run the full Rust asset_import tests and the vitest asset tests.
- [ ] Run `npm run tauri:build`.
- [ ] Add a `daily_log.md` End entry.
- [ ] Open a PR, then read each CI check result.
