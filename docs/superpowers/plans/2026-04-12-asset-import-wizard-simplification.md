# Asset Import Wizard Simplification Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Simplify `Asset Import Wizard` so standardized CSV/Excel files import through only two modes (`Serialized Asset` and `Quantity Asset`) with auto-detected headers and no user-facing manual column mapping.

**Architecture:** Keep the existing staged-review import pipeline and simplify the entry flow instead of rewriting the whole subsystem. Tighten backend required-field rules, add serialized `Asset Name` fallback behavior, then remove the mapping step from the frontend so file pick -> auto-detect -> review/import becomes the normal path.

**Tech Stack:** Tauri v2, Rust, rusqlite/SQLite, React 19, TypeScript, existing asset import pipeline, existing employee import UX as reference, targeted Rust unit tests, targeted Node helper tests, `npm run check:quality`.

---

## File Structure

- Modify: `src-tauri/src/db/asset_import.rs`
  - relax serialized required-field rules, preserve quantity rules, add `display_name` fallback behavior, and keep staged review intact
- Modify: `src/features/assets/assetImportModeConfig.ts`
  - define the simplified two-mode field requirements and keep alias detection aligned with the backend
- Modify: `src/features/assets/useAssetImportState.ts`
  - remove the mapping-driven step logic from the normal flow and drive choose-file -> review behavior
- Modify: `src/features/assets/AssetImportWizard.tsx`
  - remove the `Map Columns` screen and simplify the wizard UI around file choice, validation, and review
- Modify: `src/features/assets/assetImportCopy.ts`
  - update copy so the wizard explains direct import/review behavior instead of column mapping
- Modify: `src/types/staff.ts`
  - adjust any inspection/staging types if the UI needs clearer file validation metadata
- Modify: `src/services/staff-api.ts`
  - thread any changed inspection/staging payloads through the Tauri wrapper if required
- Create: `scripts/asset-import-wizard-simplification.test.ts`
  - lock the simplified field requirements and frontend helper behavior
- Modify: `scripts/asset-dashboard-formatting.test.ts`
  - keep any shared import-copy or formatting rails aligned if helper logic stays there

## Chunk 1: Simplify backend file-shape requirements

### Task 1: Make serialized imports require `Assetcode` and `Category`, not `Asset Name`

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`
- Test: `src-tauri/src/db/asset_import.rs`

- [ ] **Step 1: Write the failing Rust tests**

Add focused tests in `src-tauri/src/db/asset_import.rs` for:

- serialized CSV parsing succeeds when the file has `Assetcode` + `Category` but no `Asset Name`
- quantity CSV parsing still fails when `Quantity` is missing
- the missing-required-field error names the right columns for each mode

- [ ] **Step 2: Run the targeted tests to confirm they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: FAIL because serialized mode still requires `displayName`.

- [ ] **Step 3: Update required-field logic in Rust**

In `src-tauri/src/db/asset_import.rs`:

- change serialized required fields to:
  - `assetCode`
  - `assetType`
- keep quantity required fields as:
  - `displayName`
  - `assetType`
  - `quantity`
- keep header alias detection and sheet/header-row detection unchanged

- [ ] **Step 4: Re-run the targeted Rust tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS for the required-field coverage.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/db/asset_import.rs
git commit -m "feat: simplify serialized asset import requirements"
```

## Chunk 2: Add serialized `Asset Name` fallback behavior

### Task 2: Persist a valid `display_name` without requiring the source column

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`
- Test: `src-tauri/src/db/asset_import.rs`

- [ ] **Step 1: Write the failing Rust tests**

Add tests that assert serialized import uses this fallback chain:

1. imported `Asset Name`
2. imported or derived `Computer Name`
3. `Assetcode`

Cover at least:

- explicit `Asset Name` present
- `Asset Name` blank but `Computer Name` present
- both blank so `Assetcode` is used

- [ ] **Step 2: Run the targeted Rust tests to confirm they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: FAIL because serialized import currently requires `displayName` and does not yet guarantee the fallback chain end-to-end.

- [ ] **Step 3: Implement the fallback chain**

In `src-tauri/src/db/asset_import.rs`:

- derive a normalized serialized `display_name` from:
  - `display_name`
  - else `computer_name`
  - else `asset_code`
- make sure staged rows, row validation, and official import commit all use the same fallback result

- [ ] **Step 4: Re-run the targeted Rust tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS for serialized `display_name` fallback behavior.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/db/asset_import.rs
git commit -m "feat: add serialized asset name fallback"
```

## Chunk 3: Remove the manual mapping step from the frontend flow

### Task 3: Simplify wizard state to choose-file -> review

**Files:**
- Modify: `src/features/assets/useAssetImportState.ts`
- Modify: `src/features/assets/AssetImportWizard.tsx`
- Modify: `src/features/assets/assetImportCopy.ts`
- Modify: `src/features/assets/assetImportModeConfig.ts`
- Test: `scripts/asset-import-wizard-simplification.test.ts`

- [ ] **Step 1: Write the failing frontend helper test**

Create `scripts/asset-import-wizard-simplification.test.ts` with assertions for:

- only two user-facing modes exist
- serialized required mapping keys are `assetCode` and `category`
- quantity required mapping keys are `itemName`, `category`, and `quantity`
- the simplified flow no longer references a dedicated `map_columns` step in helper state/config

- [ ] **Step 2: Run the helper test to confirm it fails**

Run:

```powershell
node --experimental-strip-types scripts/asset-import-wizard-simplification.test.ts
```

Expected: FAIL because the current frontend still models the mapping step and old required keys.

- [ ] **Step 3: Update frontend mode requirements**

In `src/features/assets/assetImportModeConfig.ts`:

- keep the two modes only
- change serialized required keys to `assetCode` + `category`
- keep quantity required keys as `itemName` + `category` + `quantity`
- keep header aliases intact

- [ ] **Step 4: Simplify the state machine**

In `src/features/assets/useAssetImportState.ts`:

- remove the normal-flow dependency on `map_columns`
- after file inspection, move straight toward staged review when the detected file shape is valid
- keep review, refresh, row edit, skip, and import behaviors intact

- [ ] **Step 5: Simplify the wizard UI**

In `src/features/assets/AssetImportWizard.tsx` and `src/features/assets/assetImportCopy.ts`:

- remove the dedicated `Map Columns` step
- update step pills and explanatory copy
- keep sheet switching available where useful
- show clear missing-column errors instead of mapping controls

- [ ] **Step 6: Re-run the helper test**

Run:

```powershell
node --experimental-strip-types scripts/asset-import-wizard-simplification.test.ts
```

Expected: PASS for simplified mode requirements and flow assumptions.

- [ ] **Step 7: Commit**

```powershell
git add src/features/assets/useAssetImportState.ts src/features/assets/AssetImportWizard.tsx src/features/assets/assetImportCopy.ts src/features/assets/assetImportModeConfig.ts scripts/asset-import-wizard-simplification.test.ts
git commit -m "feat: simplify asset import wizard flow"
```

## Chunk 4: Polish validation messaging and verify end to end

### Task 4: Align inspection messaging with the direct-import workflow

**Files:**
- Modify: `src/types/staff.ts`
- Modify: `src/services/staff-api.ts`
- Modify: `src/features/assets/useAssetImportState.ts`
- Modify: `src/features/assets/AssetImportWizard.tsx`
- Modify: `scripts/asset-dashboard-formatting.test.ts`

- [ ] **Step 1: Add the failing validation/copy assertions**

Extend or add tests that assert:

- unsupported files show a clear missing-column reason
- the wizard copy no longer tells the user to inspect headers before mapping
- success-path copy reflects direct review/import behavior

- [ ] **Step 2: Run the focused frontend checks to confirm they fail**

Run:

```powershell
node --experimental-strip-types scripts/asset-import-wizard-simplification.test.ts
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: FAIL because legacy mapping-oriented copy and validation wording still exists.

- [ ] **Step 3: Thread any required validation metadata**

If clearer validation messaging needs additional typed data, update:

- `src/types/staff.ts`
- `src/services/staff-api.ts`

Prefer minimal additive contract changes. Do not broaden the API without a concrete message/UI need.

- [ ] **Step 4: Update the copy and validation surfaces**

In the relevant asset import frontend files:

- replace mapping-oriented copy with direct-import copy
- show the missing required columns for the selected mode
- keep the review step language intact

- [ ] **Step 5: Run the focused frontend checks again**

Run:

```powershell
node --experimental-strip-types scripts/asset-import-wizard-simplification.test.ts
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: PASS for simplified validation and copy.

- [ ] **Step 6: Commit**

```powershell
git add src/types/staff.ts src/services/staff-api.ts src/features/assets/useAssetImportState.ts src/features/assets/AssetImportWizard.tsx scripts/asset-dashboard-formatting.test.ts scripts/asset-import-wizard-simplification.test.ts
git commit -m "style: align asset import validation copy"
```

## Chunk 5: Full verification and handoff

### Task 5: Prove the simplified flow works before claiming completion

**Files:**
- Modify only if verification reveals a real defect

- [ ] **Step 1: Run backend verification**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run frontend helper verification**

Run:

```powershell
node --experimental-strip-types scripts/asset-import-wizard-simplification.test.ts
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: PASS.

- [ ] **Step 3: Run repository quality checks**

Run:

```powershell
npm run check:quality
```

Expected: PASS.

- [ ] **Step 4: Manual smoke test**

Run:

```powershell
npm run tauri:dev
```

Then verify manually:

- `Serialized Asset` import with a standard workbook file opens review without any column-mapping step
- `Quantity Asset` import opens review without any column-mapping step
- a file missing required columns shows a clear validation message and does not stage/import
- serialized rows still show expected row counts and quantity rows still show expected totals

- [ ] **Step 5: Commit any final fixes**

```powershell
git add -A
git commit -m "fix: polish asset import wizard simplification"
```

- [ ] **Step 6: Prepare branch handoff**

Summarize:

- what changed in backend rules
- what changed in the wizard UI
- which test commands passed
- whether manual Tauri smoke test passed

This summary is the handoff note for the execution session.
