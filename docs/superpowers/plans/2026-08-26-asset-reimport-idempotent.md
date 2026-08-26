# Idempotent Asset Re-import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make serialized Asset imports classify exact existing records as Existing/Skip, preserve code/serial conflicts as errors, and insert only genuinely new records on approval or direct import.

**Architecture:** Add one backend identity classifier in `src-tauri/src/db/asset_import.rs` that compares normalized incoming code/serial pairs with the current `assets` table and with other incoming rows. Reuse it for staged-batch revalidation and direct preview/commit; represent Existing rows with the existing schema-compatible `skipped` status plus a non-error reason, while keeping New rows `valid` and Conflict rows `error`. Extend the direct preview contract with skipped/existing counts and labels only as needed by the current Asset wizard.

**Tech Stack:** Rust, rusqlite, calamine, Tauri command serialization, React 19, TypeScript, Vitest, Rust unit tests.

## Global Constraints

- Asset import is additive and idempotent by default.
- Existing matching Assets are `Existing / Skip`, never blocking duplicate errors.
- Genuinely new Assets are `New` and are the only rows eligible for insertion.
- Code/serial identity conflicts are `Error / Conflict` and are never imported.
- Existing Asset metadata, active loans, holders, statuses, Employee import, canonical identity, uniqueness constraints, and schema remain unchanged.
- `E:\Staff_Kit\00_ExSource\AssetList.xlsx` is read-only acceptance input; automated validation must not write the production database.

---

### Task 1: Add failing backend classification and acceptance tests

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs` test module near the existing duplicate-serial and serialized-import tests.
- Reference: `src-tauri/src/db/asset.rs` for asset creation and serialized identity behavior; `src-tauri/src/db/borrow.rs` for active-loan fixtures.

**Interfaces:**
- Consumes: existing `AssetImportRowSeedInput`, `create_asset_import_batch_seed_conn`, `import_asset_import_batch_valid_rows_conn`, `preview_asset_import_seed_conn`, and test DB setup helpers.
- Produces: executable regression cases that define the classifier behavior before implementation.

- [ ] **Step 1: Add a focused exact-existing test**

  Seed an asset with code `VNLAP504` and serial `SN-504`, create one serialized import row with the same code and serial, and assert the staged row is `skipped`, has no validation error, and the batch has `skippedRows == 1`, `validRows == 0`, and `errorRows == 0`.

- [ ] **Step 2: Add compatible-serial tests**

  Cover an incoming exact code with no serial and an existing exact code with no serial; both must be Existing/Skip. Cover an exact code where only one side has a serial; it is compatible and must not be an error. Cover exact code with equal serial using different case/whitespace normalization.

- [ ] **Step 3: Add cross-conflict tests**

  Seed `ASSET-A/SN-A` and `ASSET-B/SN-B`; import `ASSET-A/SN-B`; assert one `error` row with a conflict message and zero eligible imports. Also assert two incoming new rows cannot reuse one code or one serial.

- [ ] **Step 4: Add mixed acceptance fixture test**

  Build ten serialized rows using the six listed existing Lenovo codes and four listed Samsung codes, seed only the six existing rows, and assert preview/batch counts are total 10, valid/new 4, skipped/existing 6, and errors 0.

- [ ] **Step 5: Add commit/idempotency and state-preservation tests**

  Approve the mixed batch and assert four assets are inserted, six existing assets are unchanged, an existing active loan/status/holder remains unchanged, and no duplicate codes exist. Import the same ten rows again and assert all ten are skipped, imported count is zero, and Asset count is unchanged. Approve an Existing-only batch and assert zero Asset inserts.

- [ ] **Step 6: Run the focused tests and verify they fail for the old behavior**

  Run:

  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml asset_import --lib -- --nocapture
  ```

  Expected before implementation: exact existing rows are currently `error` because `validate_staged_row` rejects existing codes/serials; the new assertions must expose those failures.

---

### Task 2: Implement shared backend identity classification

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs` around `revalidate_batch_tx`, `validate_staged_row`, existing code/serial lookup helpers, direct preview conversion, and serialized commit loop.
- Test: `src-tauri/src/db/asset_import.rs` tests from Task 1.

**Interfaces:**
- Consumes: normalized staged row values, current `assets` rows, and existing `AssetImportRowState` flow.
- Produces: deterministic row statuses/errors for both batch and direct import paths; no new schema/API command required.

- [ ] **Step 1: Introduce normalized existing identity lookup**

  Load `asset_code`, `serial_number`, and `id` pairs from `assets` into maps keyed by the existing normalization rules. Keep SQL parameterized and preserve null/blank serial semantics.

- [ ] **Step 2: Implement pair classification**

  Return one internal classification for each serialized row:

  ```rust
  enum SerializedAssetImportClassification {
      New,
      Existing { asset_id: i64 },
      Conflict(String),
  }
  ```

  An exact code match is Existing when serials are absent on either side or equal after normalization. A code match plus a different existing serial is Conflict. A serial-only match to another asset is Conflict. A row with neither existing identity is New. Apply the same logic against earlier incoming rows to prevent duplicate new identities.

- [ ] **Step 3: Replace duplicate validation errors for Existing rows**

  Update `revalidate_batch_tx` to classify serialized rows before generic required-field/owner validation. Existing rows must become `ROW_STATUS_SKIPPED`, have an empty `validation_errors_json`, and retain a clear internal/presentation reason without invoking an asset write. New rows continue through required fields, category, owner, and duplicate validation. Conflicts become `ROW_STATUS_ERROR` with `Conflict` in their validation message.

- [ ] **Step 4: Revalidate approval in the transaction**

  Before selecting rows for serialized insertion, re-run classification against the transaction’s live `assets` table. Convert newly exact-existing valid rows to skipped, convert newly conflicting rows to errors, refresh the batch summary, and select only remaining `ROW_STATUS_VALID` New rows. Do not call `create_asset_tx` for skipped rows.

- [ ] **Step 5: Keep direct preview/report aligned**

  Make `preview_asset_import_seed_conn` use the same staged classification and expose skipped/existing counts through the direct preview type. Make `asset_direct_report_from_batch_detail` report Existing rows as skipped and conflicts as failed/errors without treating Existing rows as failures.

- [ ] **Step 6: Run the focused Rust tests green**

  Run the focused `asset_import` test filter again and confirm all new classification, mixed-batch, idempotency, and state-preservation tests pass.

---

### Task 3: Update Asset preview presentation and frontend tests

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs` direct preview serialization if required by Task 2.
- Modify: `src/types/staff.ts` `AssetDirectImportPreview` if skipped/new count fields are added.
- Modify: `src/features/assets/AssetImportWizard.tsx` summary/status rendering.
- Modify: `src/features/assets/assetImportStatusMeta.ts` only if shared labels need `existing` support.
- Test: relevant `src/features/assets/*.test.tsx` or a new focused Asset import wizard test matching existing test patterns.

**Interfaces:**
- Consumes: backend direct preview/report fields and existing `SharedImportShell` summary/error contracts.
- Produces: a preview that visibly distinguishes Existing/Skipped from New/Ready and keeps Approve Import disabled when there are no new rows.

- [ ] **Step 1: Extend the typed preview summary**

  Add only the backend fields needed to show `Total Rows`, `New Rows`, `Existing / Skipped`, and `Error Rows`; preserve compatibility with the existing Tauri camelCase contract.

- [ ] **Step 2: Render status labels and counts**

  Map backend `valid` to `New`/`Ready`, `skipped` to `Existing`/`Skipped`, `error` to `Error`/`Conflict`, and keep Existing rows out of the red error collection. The row card must not render Existing as a generic green `valid` or red error.

- [ ] **Step 3: Preserve approval behavior**

  Ensure `previewApproveDisabled` uses the New/valid count, so an Existing-only preview cannot issue an import request, while the mixed 6+4 preview can approve exactly four rows.

- [ ] **Step 4: Add/adjust frontend contract tests**

  Assert the summary labels and button disabled/enabled behavior for an Existing-only preview, a mixed preview with four New rows, and a conflict preview. Run the focused frontend test file(s).

---

### Task 4: Verify, review, and checkpoint

**Files:**
- Modify: `daily_log.md` with the End record after validation.
- Review: all changed source/test files and `git diff --check`.

**Interfaces:**
- Consumes: completed backend/frontend implementation and test evidence.
- Produces: a focused commit on `fix/asset-reimport-idempotent`; no merge, push, or worktree deletion.

- [ ] **Step 1: Run focused Rust and frontend checks**

  Run the focused Asset import Rust tests and focused Asset import frontend tests, then run the real workbook preview/acceptance harness against `E:\Staff_Kit\00_ExSource\AssetList.xlsx` using an isolated test database or copy. Confirm 6 Existing + 4 New + 0 Error and four inserted rows.

- [ ] **Step 2: Run relevant full gates**

  Run `npm run test:tauri`, `npm run test:unit`, and `npm run check:quality`. Confirm no command opened the production DB for writes.

- [ ] **Step 3: Review scope and generated files**

  Run `git diff --check`, inspect `git status`, and ensure only Asset import code/tests/types/UI plus the daily log are included. Do not stage generated schemas or unrelated work.

- [ ] **Step 4: Commit the implementation**

  ```powershell
  git add src-tauri/src/db/asset_import.rs src/types/staff.ts src/features/assets/AssetImportWizard.tsx src/features/assets/assetImportStatusMeta.ts src/features/assets/*.test.tsx daily_log.md
  git commit -m "fix(import): make asset reimport idempotent"
  ```

- [ ] **Step 5: Report without merge or push**

  Report the worktree branch/HEAD, files changed, exact matching behavior, focused/full test counts, DB writes, commit SHA, remaining risk, and QEEN readiness. End with `READY FOR QEEN VALIDATION` unless a verified blocker remains.
