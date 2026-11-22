# Edit Asset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Super admin can edit the descriptive fields of one serialized asset from the Asset Dashboard, with an audit row holding before/after values.

**Architecture:** New Rust `update_asset_details_conn` (one transaction, diff, update, audit) behind a `require_super_admin` Tauri command; `staffApi.updateAssetDetails`; `useAssetDashboardState.updateAssetDetails`; an Edit button per Serialized row that opens the existing `Drawer` component.

**Tech Stack:** Tauri v2 / rusqlite (SQLCipher), React 19, vitest + Testing Library.

**Spec:** `docs/superpowers/specs/2026-11-22-edit-asset-design.md`

## Global Constraints

- Gate: backend `auth_session::require_super_admin`; frontend `auth.isSuperAdminAccount`.
- Editable fields only: `display_name` (required, trimmed, non-empty), `display_name_short`, `brand`, `model`, `serial_number`, `adapter_number`, `usage_location`, `notes`. Trim all; blank optional → NULL.
- Read-only: asset code, category, status, holder.
- Errors start with `Not found` / `Invalid` so `classifyError` shows them.
- Audit: event `asset.update_details`, actor_type `local_account`, entity `asset` / asset id, payload `{"assetCode": .., "changes": {"<camelField>": {"before": .., "after": ..}}}` with changed fields only. No change → no UPDATE, no audit.
- Some files are CRLF: keep line endings (byte-level replace if the Edit tool would rewrite the file).
- Build env: Strawberry Perl first on PATH for cargo (`export PATH="/c/Strawberry/perl/bin:$PATH"`).

## Review Focus

- Whitespace-only display name → `Invalid display name` error, nothing written (Task 1 test).
- Same values re-saved (incl. `"  A17 "` vs stored `"A17"`, `""` vs NULL) → treated as no change, no audit (Task 1 test).
- Asset id that no longer exists (deleted meanwhile) → `Not found: asset` (Task 1 test).
- Drawer opened from a filtered/paged table edits the clicked row, not the row index (Task 3 test uses a row from page 1 with a distinct id).
- Save failure keeps the drawer open with the drafted values (Task 3 test).

---

### Task 1: Backend update + audit

**Files:**
- Modify: `src-tauri/src/db/asset.rs` (new input struct near `AssetUpsertInput` ~L16; new fn after `load_asset_record_by_id_conn` ~L324; tests in `mod tests` ~L2216)
- Modify: `src-tauri/src/db/asset_import.rs` (wrapper next to `create_asset_manually` ~L598)
- Modify: `src-tauri/src/lib.rs` (command next to `create_asset_manually` ~L1334; register in `invoke_handler` ~L1675 and the command-name list ~L1834)

**Interfaces:**
- Produces:
  - `#[derive(Debug, Clone, Deserialize)] #[serde(rename_all = "camelCase")] pub struct AssetDetailsUpdateInput { display_name: String, display_name_short, brand, model, serial_number, adapter_number, usage_location, notes: Option<String> }` (all `pub`)
  - `pub(crate) fn update_asset_details_conn(conn: &mut Connection, asset_id: i64, input: AssetDetailsUpdateInput, actor_ref: Option<&str>) -> Result<AssetRecord, String>`
  - `pub fn db::update_asset_details(app: &AppHandle, asset_id: i64, input: AssetDetailsUpdateInput) -> Result<AssetRecord, String>` (uses `active_actor_ref`)
  - Tauri command `update_asset_details(session_token: String, asset_id: i64, payload: AssetDetailsUpdateInput) -> Result<AssetRecord, String>`
- Consumes: `load_asset_by_id_tx`, `load_asset_record_by_id_conn`, `audit::insert_audit_log_conn`.

- [ ] **Step 1: Write failing tests** in `asset.rs` `mod tests` (seed one asset via `upsert_assets_conn` with display_name "Samsung Galaxy A18", model "A18", notes "x"):
  - `update_asset_details_conn_updates_fields_and_audits_changes`: update to display_name "  Samsung Galaxy A17 ", model "A17", notes None → record.display_name == "Samsung Galaxy A17", model Some("A17"), notes None; exactly 1 row in `audit_logs` with event_type `asset.update_details`; its payload JSON `changes.displayName.before == "Samsung Galaxy A18"`, `.after == "Samsung Galaxy A17"`, and `changes` has no `serialNumber` key.
  - `update_asset_details_conn_rejects_blank_display_name`: display_name "   " → Err starting with "Invalid"; display_name in DB unchanged; 0 audit rows.
  - `update_asset_details_conn_without_changes_writes_nothing`: same values (optional "" where stored NULL) → Ok; 0 audit rows; `updated_at` unchanged.
  - `update_asset_details_conn_missing_asset_is_not_found`: asset_id 9999 → Err starting with "Not found".
- [ ] **Step 2: Run** `node scripts/run-with-shared-cargo-target.mjs test --manifest-path src-tauri/Cargo.toml update_asset_details` (or the repo's usual cargo test invocation) → FAIL: `update_asset_details_conn` not found.
- [ ] **Step 3: Implement** `update_asset_details_conn`: normalize input (trim; empty optional → None); reject empty name with `Invalid display name: required`; open tx; `load_asset_by_id_tx` → None ⇒ `Not found: asset {asset_id}`; load before via `load_asset_record_by_id_conn(&tx, id)`; build `changes` map comparing before vs normalized; empty ⇒ commit-free return of before; else one `UPDATE assets SET <8 fields>, updated_at = datetime('now') WHERE id = ?`, insert audit, commit, return reloaded record. `AssetRecord` has no `brand`: read the before-brand with `SELECT brand FROM assets WHERE id = ?` inside the tx (do not add brand to `AssetRecord`).
- [ ] **Step 4: Add** the `db::update_asset_details` wrapper and the `lib.rs` command with `auth_session::require_super_admin(&session_store, &session_token)?`; register it in both lists.
- [ ] **Step 5: Run** the Step 2 command → PASS; then full `cargo test` for src-tauri → all pass.
- [ ] **Step 6: Commit** `feat(assets): add super-admin update_asset_details command with audit`.

### Task 2: Expose brand on dashboard rows + API wrapper + state action

**Files:**
- Modify: `src-tauri/src/db/asset.rs` (`AssetDashboardSerializedRecord` L110 add `pub brand: Option<String>`; its SELECT/mapping ~L1536)
- Modify: `src/types/staff.ts` (`AssetDashboardSerializedRecord` ~L250 add `brand: string | null`; new `AssetDetailsUpdateInput` type)
- Modify: `src/services/staff-api.ts` (next to `createAssetManually` ~L409), `src/services/staff-api.test.ts`
- Modify: `src/features/assets/useAssetDashboardState.ts` (new callback, returned)
- Modify: `src/features/assets/AssetDashboard.pagination.test.tsx` (fixture gets `brand: null`)

**Interfaces:**
- Produces:
  - TS `type AssetDetailsUpdateInput = { displayName: string; displayNameShort: string | null; brand: string | null; model: string | null; serialNumber: string | null; adapterNumber: string | null; usageLocation: string | null; notes: string | null }`
  - `staffApi.updateAssetDetails(assetId: number, payload: AssetDetailsUpdateInput) => Promise<AssetRecord>` calling `call("update_asset_details", { assetId, payload })`
  - `useAssetDashboardState` returns `updateAssetDetails(assetId: number, payload: AssetDetailsUpdateInput): Promise<AssetRecord | null>` and `isSavingAssetId: number | null`; on success sets status `Updated asset {assetCode}.` and `loadDashboard({ refresh: true })`; on error `setGlobalError(getUserErrorMessage(error))`, returns null.

- [ ] **Step 1: Failing test** in `staff-api.test.ts`, following the file's existing invoke-mock pattern: `updateAssetDetails(7, {...displayName: "Samsung Galaxy A17"...})` invokes `"update_asset_details"` with `{ sessionToken: <token>, assetId: 7, payload: {...} }` (match how sibling tests assert the session token).
- [ ] **Step 2: Run** `npx vitest run src/services/staff-api.test.ts` → FAIL (not a function).
- [ ] **Step 3: Implement** type, wrapper, Rust `brand` column on the dashboard record, fixture update, state hook callback.
- [ ] **Step 4: Run** `npx vitest run src/services src/features/assets` and `npx tsc --noEmit` → pass; Rust tests still pass.
- [ ] **Step 5: Commit** `feat(assets): wire updateAssetDetails API and dashboard state`.

### Task 3: Edit drawer in the Serialized table

**Files:**
- Modify: `src/features/assets/AssetDashboard.tsx` (`AssetDashboardProps.auth` L61 add `"isSuperAdminAccount"`; `SerializedDashboardTable` ~L973 gets an actions column + `onEdit(row)`; new `EditAssetDrawer` component using `src/components/Drawer`)
- Modify: `src/App.version.test.tsx` / other renderers only if typecheck requires the new auth key (SettingsView already passes full `auth`)
- Create: `src/features/assets/AssetDashboard.editAsset.test.tsx` (copy harness from `AssetDashboard.pagination.test.tsx`)

**Interfaces:**
- Consumes: `assetDashboard.updateAssetDetails`, `assetDashboard.isSavingAssetId`, row `brand`.
- Produces: button with accessible name `Edit {assetCode}`; drawer fields labelled `Display name`, `Short name`, `Brand`, `Model`, `Serial number`, `Adapter number`, `Location`, `Notes`; Save button `Save changes` (disabled when name blank, not dirty, or saving).

- [ ] **Step 1: Failing tests:**
  - `hides Edit buttons for admins who are not super admin`: auth `{ isAdminAccount: true, isSuperAdminAccount: false, canImportData: true }` → `queryByRole("button", { name: /^Edit AST-/ })` is null.
  - `saves edited display name for the clicked row`: super admin; click `Edit AST-0003`; drawer shows asset code `AST-0003` as text (not an input); change `Display name` to "Samsung Galaxy A17"; click `Save changes` → `updateAssetDetails` called with `(3, expect.objectContaining({ displayName: "Samsung Galaxy A17", model: "M3" }))`; mock resolves a record → drawer closes.
  - `keeps the drawer open with the draft when save fails`: `updateAssetDetails` resolves null → drawer still open, input still "Samsung Galaxy A17".
- [ ] **Step 2: Run** `npx vitest run src/features/assets/AssetDashboard.editAsset.test.tsx` → FAIL (no Edit button).
- [ ] **Step 3: Implement** the actions column (only when `auth.isSuperAdminAccount`), drawer state keyed by `assetId`, draft from the row (null → ""), send trimmed values with "" → null.
- [ ] **Step 4: Run** the test file → PASS; then `npm test` (full vitest) and `npx tsc --noEmit` → all pass.
- [ ] **Step 5: Commit** `feat(assets): super-admin Edit drawer for serialized assets`; add a `daily_log.md` End entry in the same commit.

### Task 4: Ship

- [ ] Full Rust + vitest suites green; push branch `feat/edit-asset`; open PR; merge only on the user's OK; rebuild installer (Strawberry Perl on PATH).
