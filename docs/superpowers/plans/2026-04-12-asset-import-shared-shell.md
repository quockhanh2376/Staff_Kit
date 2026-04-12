# Shared Asset Import Shell Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the legacy asset import wizard with a shared file-import shell used by both employee import and asset import, where asset import becomes `choose file -> preview -> approve -> import valid rows directly into the database`.

**Architecture:** Extract a shared frontend import shell for file selection, preview, approval, and reporting. Keep employee import and asset import as separate adapters behind that shell, and add new direct asset preview/import backend commands so the new asset flow no longer depends on staged batch review.

**Tech Stack:** Tauri v2, Rust, rusqlite/SQLite, React 19, TypeScript, existing employee import flow, existing asset import parsing logic, targeted Rust tests, targeted Node tests, `npm run check:quality`.

---

## File Structure

- Create: `src/features/import/sharedImportShell.tsx`
  - reusable preview/approve shell component for file-driven imports
- Create: `src/features/import/sharedImportTypes.ts`
  - shared TypeScript contract for import preview/report data used by the shell
- Modify: `src/features/import/useImportState.ts`
  - adapt EE list import to the shared shell contract without changing employee business logic
- Modify: `src/features/import/ImportDrawer.tsx`
  - replace employee-specific preview UI with the shared shell
- Create: `src/features/assets/useAssetDirectImportState.ts`
  - new asset import state for `Serialized Asset` and `Quantity Asset` using direct preview/import
- Modify: `src/features/assets/AssetImportWizard.tsx`
  - replace the old staged wizard UI with the shared shell wired to the asset adapter
- Modify: `src/features/assets/assetImportCopy.ts`
  - remove mapping/review-batch copy and replace with direct preview/approve copy
- Modify: `src/features/assets/assetImportModeConfig.ts`
  - keep mode labels and alias rules aligned with the new direct import flow
- Modify: `src/services/staff-api.ts`
  - add direct asset preview/import API wrappers and shared import payload helpers if needed
- Modify: `src/types/staff.ts`
  - add shared import preview/report DTOs and new asset direct-import DTOs
- Modify: `src-tauri/src/db/asset_import.rs`
  - add direct preview/import functions for `Serialized Asset` and `Quantity Asset`
- Modify: `src-tauri/src/db/import.rs`
  - align employee import preview/report shape with the shared shell if needed
- Modify: `src-tauri/src/lib.rs`
  - register the new asset direct-import Tauri commands
- Create: `scripts/shared-import-shell.test.ts`
  - lock the shared shell contract and copy invariants
- Modify: `scripts/asset-import-copy.test.ts`
  - update asset copy assertions for the new preview/approve wording
- Modify: `scripts/asset-import-wizard-simplification.test.ts`
  - replace old “no map columns” assertions with direct-import flow assertions

## Chunk 1: Define the shared preview/approve contract

### Task 1: Add a shared import shell data shape

**Files:**
- Create: `src/features/import/sharedImportTypes.ts`
- Modify: `src/types/staff.ts`
- Test: `scripts/shared-import-shell.test.ts`

- [ ] **Step 1: Write the failing Node test**

Create `scripts/shared-import-shell.test.ts` with assertions for a minimal shared contract:

- preview summary has `totalRows`, `validRows`, `errorRows`
- preview rows support column/field display for both employee and asset flows
- final report supports `imported`, `skipped`, `failed`, and error items

- [ ] **Step 2: Run the test to confirm it fails**

Run:

```powershell
node --experimental-strip-types scripts/shared-import-shell.test.ts
```

Expected: FAIL because the shared import shell contract does not exist yet.

- [ ] **Step 3: Create the shared contract**

Add `src/features/import/sharedImportTypes.ts` and thread the matching DTOs into `src/types/staff.ts`.

Keep the contract narrow:

- file metadata
- preview summary
- preview rows
- error items
- import report

- [ ] **Step 4: Re-run the test**

Run:

```powershell
node --experimental-strip-types scripts/shared-import-shell.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add src/features/import/sharedImportTypes.ts src/types/staff.ts scripts/shared-import-shell.test.ts
git commit -m "feat: add shared import shell contract"
```

## Chunk 2: Build the shared frontend shell and move EE list onto it

### Task 2: Extract the reusable preview/approve UI from the employee import flow

**Files:**
- Create: `src/features/import/sharedImportShell.tsx`
- Modify: `src/features/import/ImportDrawer.tsx`
- Modify: `src/features/import/useImportState.ts`
- Test: `scripts/shared-import-shell.test.ts`

- [ ] **Step 1: Extend the failing Node test**

Add assertions for:

- the shared shell uses `Approve` wording
- employee import still exposes file pick, preview, approve, and final report
- the employee flow no longer owns bespoke preview markup that duplicates the shared shell

- [ ] **Step 2: Run the test to confirm it fails**

Run:

```powershell
node --experimental-strip-types scripts/shared-import-shell.test.ts
```

Expected: FAIL because the shared shell component is not implemented.

- [ ] **Step 3: Build the shared shell**

Create `src/features/import/sharedImportShell.tsx` as the reusable component that renders:

- selected file section
- preview summary
- preview rows
- error list
- approve / cancel actions
- final import report

- [ ] **Step 4: Rewire EE list import to use it**

In `src/features/import/useImportState.ts` and `src/features/import/ImportDrawer.tsx`:

- adapt existing employee preview/report data into the shared contract
- keep employee backend behavior unchanged
- keep the employee UX materially the same, but rendered through the shared shell

- [ ] **Step 5: Re-run the test**

Run:

```powershell
node --experimental-strip-types scripts/shared-import-shell.test.ts
npm run check:frontend
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src/features/import/sharedImportShell.tsx src/features/import/ImportDrawer.tsx src/features/import/useImportState.ts scripts/shared-import-shell.test.ts
git commit -m "refactor: move employee import to shared shell"
```

## Chunk 3: Add direct asset preview/import backend commands

### Task 3: Implement asset preview/import without staged batch review

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/services/staff-api.ts`
- Modify: `src/types/staff.ts`
- Test: `src-tauri/src/db/asset_import.rs`

- [ ] **Step 1: Write the failing Rust tests**

Add focused Rust tests covering:

- serialized preview returns correct `totalRows`, `validRows`, `errorRows`
- quantity preview returns correct summary counts
- direct import imports only valid rows and skips invalid rows
- serialized import still applies:
  - `Computer Name` fallback
  - `Asset Name -> Computer Name -> Assetcode` fallback

- [ ] **Step 2: Run the targeted Rust tests to confirm failure**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: FAIL because the direct preview/import command path does not exist yet.

- [ ] **Step 3: Implement new asset direct-import functions**

In `src-tauri/src/db/asset_import.rs`:

- add preview functions for:
  - `Serialized Asset`
  - `Quantity Asset`
- add direct import functions that:
  - re-parse the file
  - import valid rows
  - skip invalid rows
  - return a final report

Do not route the new UI through `asset_import_batches`.

- [ ] **Step 4: Register and thread the Tauri API**

In `src-tauri/src/lib.rs`, `src/services/staff-api.ts`, and `src/types/staff.ts`:

- register the new preview/import commands
- add typed wrappers for the new frontend adapter

- [ ] **Step 5: Re-run the targeted tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/db/asset_import.rs src-tauri/src/lib.rs src/services/staff-api.ts src/types/staff.ts
git commit -m "feat: add direct asset preview and import commands"
```

## Chunk 4: Replace the asset wizard UI with the shared shell

### Task 4: Remove `Map Columns / Review Batch` from the asset entry flow

**Files:**
- Create: `src/features/assets/useAssetDirectImportState.ts`
- Modify: `src/features/assets/AssetImportWizard.tsx`
- Modify: `src/features/assets/assetImportCopy.ts`
- Modify: `src/features/assets/assetImportModeConfig.ts`
- Modify: `scripts/asset-import-copy.test.ts`
- Modify: `scripts/asset-import-wizard-simplification.test.ts`

- [ ] **Step 1: Write the failing Node tests**

Update/add tests asserting:

- asset import shows only `Serialized Asset` and `Quantity Asset`
- there is no `Map Columns` copy
- there is no `Stage Batch` copy
- the primary action is `Approve Import`
- preview is read-only

- [ ] **Step 2: Run the tests to confirm failure**

Run:

```powershell
node --experimental-strip-types scripts/asset-import-copy.test.ts
node --experimental-strip-types scripts/asset-import-wizard-simplification.test.ts
```

Expected: FAIL because the old wizard flow is still rendered.

- [ ] **Step 3: Add the new asset adapter hook**

Create `src/features/assets/useAssetDirectImportState.ts` to own:

- mode selection
- file inspection
- preview data loading
- approval call
- import report state

Keep it separate from the legacy `useAssetImportState.ts` so responsibilities stay clean during the transition.

- [ ] **Step 4: Replace the asset wizard UI**

In `src/features/assets/AssetImportWizard.tsx`:

- remove the old staged wizard layout from the active render path
- render the shared shell with the asset adapter
- keep file-mode selection and optional sheet selection where needed

In `src/features/assets/assetImportCopy.ts` and `src/features/assets/assetImportModeConfig.ts`:

- remove mapping-oriented text
- keep mode labels and alias rules aligned with the new flow

- [ ] **Step 5: Re-run the Node tests**

Run:

```powershell
node --experimental-strip-types scripts/asset-import-copy.test.ts
node --experimental-strip-types scripts/asset-import-wizard-simplification.test.ts
npm run check:frontend
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src/features/assets/useAssetDirectImportState.ts src/features/assets/AssetImportWizard.tsx src/features/assets/assetImportCopy.ts src/features/assets/assetImportModeConfig.ts scripts/asset-import-copy.test.ts scripts/asset-import-wizard-simplification.test.ts
git commit -m "feat: replace asset wizard with shared import shell"
```

## Chunk 5: Verify direct import behavior with real file semantics

### Task 5: Lock the valid-row import semantics and final report

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `scripts/shared-import-shell.test.ts`
- Modify: `scripts/asset-import-copy.test.ts`

- [ ] **Step 1: Add the failing regression tests**

Cover:

- approve remains enabled when at least one row is valid
- invalid rows are reported but do not block import
- final report wording matches:
  - imported
  - skipped
  - failed

- [ ] **Step 2: Run the tests to confirm failure**

Run:

```powershell
node --experimental-strip-types scripts/shared-import-shell.test.ts
node --experimental-strip-types scripts/asset-import-copy.test.ts
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: FAIL before the final report and approval semantics are aligned.

- [ ] **Step 3: Implement the final behavior**

Update the asset adapter/backend/report copy so:

- approval is allowed when `validRows > 0`
- import writes only the valid rows
- errors remain visible in the final report

- [ ] **Step 4: Re-run full verification**

Run:

```powershell
node --experimental-strip-types scripts/shared-import-shell.test.ts
node --experimental-strip-types scripts/asset-import-copy.test.ts
node --experimental-strip-types scripts/asset-import-wizard-simplification.test.ts
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
npm run check:quality
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/db/asset_import.rs scripts/shared-import-shell.test.ts scripts/asset-import-copy.test.ts
git commit -m "fix: import valid asset rows through shared approval flow"
```

Plan complete and saved to `docs/superpowers/plans/2026-04-12-asset-import-shared-shell.md`. Ready to execute?
