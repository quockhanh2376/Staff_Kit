# Owner-Aware Asset Import Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the asset import wizard so `Available` rows import serialized in-stock assets, `Laptop` rows import assigned serialized assets with owner resolution against the employee master, and the employee table shows derived multi-laptop `Computer Name` values from active laptop loans.

**Architecture:** Build on the existing staged asset-import pipeline instead of bypassing it. Add owner-aware staged row fields and validation in Rust, keep raw source payloads intact, let the React wizard review and edit owner mismatches before commit, and derive employee-table computer names from `asset_loans` rather than mutating `employees.computername`.

**Tech Stack:** Tauri v2, Rust + rusqlite, React 19, TypeScript, existing asset import wizard, existing employee table query pipeline, `npm run check:quality`, targeted `cargo test`, targeted node script tests.

---

## File Structure

- Modify: `src-tauri/src/db/schema.rs`
  - add staged owner-related columns or supporting structures for owner-aware asset import
- Modify: `src-tauri/src/db/mod.rs`
  - add migrations for new staged import columns and any employee-query support indexes if needed
- Modify: `src-tauri/src/db/asset_import.rs`
  - parse workbook owner fields, resolve employee ownership, validate staged owner rows, commit `Laptop` rows into `assets + asset_loans`
- Modify: `src-tauri/src/db/asset.rs`
  - expose any helper queries needed to classify laptop-style asset categories and asset lookup behavior
- Modify: `src-tauri/src/db/employee.rs`
  - enrich employee query output with derived multi-laptop computer-name display values from active loans
- Modify: `src-tauri/src/db/schema.rs`
  - keep employee-table core columns consistent while preserving derived `Computer Name` behavior
- Modify: `src/services/staff-api.ts`
  - extend typed IPC contracts for owner-aware staged row data
- Modify: `src/types/staff.ts`
  - add owner-resolution fields, warning state, and derived employee computer-name contract
- Modify: `src/features/assets/assetImportModeConfig.ts`
  - add owner-aware mapping/read helpers while preserving generic asset import behavior
- Modify: `src/features/assets/useAssetImportState.ts`
  - load owner-aware row state, update row editing, and keep review/import CTA semantics aligned
- Modify: `src/features/assets/AssetImportWizard.tsx`
  - render owner review UI, warning/error messaging, editable owner fields, and mixed import-result feedback
- Modify: `src/features/employees/EmployeeView.tsx`
  - render derived multi-line comma-separated computer names in the existing employee-table `Computer Name` column
- Test: `src-tauri/src/db/asset_import.rs`
  - add backend unit tests for owner resolution, unresolved rows, and loan creation
- Test: `src-tauri/src/db/employee.rs`
  - add backend unit tests for derived employee computer-name output
- Create: `scripts/owner-aware-asset-import-mapping.test.ts`
  - add a focused node test for UI-level computer-name formatting and mapping helpers

## Chunk 1: Extend staged import data for owner-aware laptop rows

### Task 1: Add staged owner fields and migration coverage

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/src/db/mod.rs`
- Test: `src-tauri/src/db/asset_import.rs`

- [ ] **Step 1: Write the failing backend test for owner-aware staged rows**

Add a Rust unit test in `src-tauri/src/db/asset_import.rs` that seeds a serialized import batch from a `Laptop`-style row and expects staged owner snapshot/resolution fields to exist in the loaded batch detail.

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: FAIL because staged owner fields do not exist yet.

- [ ] **Step 3: Add schema and migration support**

Add owner-aware staged storage for:

- `submitted_staff_id`
- `submitted_full_name`
- `submitted_team`
- `submitted_phone_number`
- `resolved_employee_id`
- `resolved_employee_row_id`
- `resolved_full_name`
- `resolved_team_name`
- `owner_match_status`
- `owner_warnings_json`

Preserve `raw_row_json` unchanged.

- [ ] **Step 4: Update row record structs and SQL mapping**

Extend Rust structs in `src-tauri/src/db/asset_import.rs` so batch detail responses expose the new owner-aware staged fields.

- [ ] **Step 5: Run the targeted test to verify it passes**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS for the new staged-row test.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/db/schema.rs src-tauri/src/db/mod.rs src-tauri/src/db/asset_import.rs
git commit -m "feat: stage owner-aware asset import rows"
```

## Chunk 2: Resolve laptop owners and keep reviewable warnings editable

### Task 2: Add employee resolution and owner warning validation

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src/types/staff.ts`
- Modify: `src/services/staff-api.ts`
- Test: `src-tauri/src/db/asset_import.rs`

- [ ] **Step 1: Write failing backend tests for owner resolution**

Add tests covering:

- `StaffID` values like `1302`, `ASW1302`, and `ASWVN1302` resolving to the same employee
- resolved employee with name mismatch -> warning only
- resolved employee with team mismatch -> warning only
- unresolved employee -> blocking error, not importable

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: FAIL because owner resolution logic is missing.

- [ ] **Step 3: Implement employee resolution logic**

In `src-tauri/src/db/asset_import.rs`:

- normalize submitted `StaffID` to its numeric suffix
- search the full employee master across all staff groups
- populate canonical owner fields from the resolved employee row
- compare submitted name/team snapshots against canonical employee data
- write warning-only vs blocking status into staged rows

- [ ] **Step 4: Extend row update handling for owner edits**

Allow review-time row edits for owner-oriented fields such as canonical employee ID selection, then re-run owner validation after each edit.

- [ ] **Step 5: Update TypeScript contracts**

Expose owner-aware row fields and warnings in:

- `src/types/staff.ts`
- `src/services/staff-api.ts`

- [ ] **Step 6: Run the targeted tests to verify they pass**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS for resolution and warning cases.

- [ ] **Step 7: Commit**

```powershell
git add src-tauri/src/db/asset_import.rs src/types/staff.ts src/services/staff-api.ts
git commit -m "feat: resolve laptop import owners against employee master"
```

## Chunk 3: Commit `Available` and `Laptop` rows with correct ownership semantics

### Task 3: Import `Available` rows as in-stock assets and `Laptop` rows as assigned assets

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src-tauri/src/db/asset.rs`
- Modify: `src-tauri/src/db/borrow.rs`
- Test: `src-tauri/src/db/asset_import.rs`

- [ ] **Step 1: Write failing backend tests for commit behavior**

Add tests covering:

- `Available` row imports with blank serial number and no loan record
- `Laptop` row imports create both asset record and active `asset_loan`
- unresolved `Laptop` row stays out of the successful import count
- duplicate asset/loan integrity is rejected

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: FAIL because `Laptop` owner-aware commit behavior does not exist yet.

- [ ] **Step 3: Implement split commit behavior**

Update `import_asset_import_batch_valid_rows_conn` so:

- `Available`-style valid serialized rows create `assets` in `in_stock`
- `Laptop`-style valid rows require a resolved employee and then create:
  - `assets`
  - active `asset_loans`
  - `assets.status = 'assigned'`

Reject rows that are unresolved or violate duplicate integrity rules.

- [ ] **Step 4: Keep raw source payloads and import-result counts intact**

Return enough commit result detail for IT to compare successful imports against the source file and identify unresolved rows for manual follow-up.

- [ ] **Step 5: Run the targeted tests to verify they pass**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS for `Available` and `Laptop` commit semantics.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/db/asset_import.rs src-tauri/src/db/asset.rs src-tauri/src/db/borrow.rs
git commit -m "feat: import assigned laptop rows into assets and loans"
```

## Chunk 4: Add owner-aware review UI to the import wizard

### Task 4: Show owner snapshot, canonical owner, warnings, and editable owner fields

**Files:**
- Modify: `src/features/assets/assetImportModeConfig.ts`
- Modify: `src/features/assets/useAssetImportState.ts`
- Modify: `src/features/assets/AssetImportWizard.tsx`
- Modify: `src/types/staff.ts`
- Create: `scripts/owner-aware-asset-import-mapping.test.ts`

- [ ] **Step 1: Write the failing UI helper test**

Create `scripts/owner-aware-asset-import-mapping.test.ts` covering:

- `computerName = "ASW" + assetCode`
- multi-laptop output joins values with `,\n`
- owner-aware mapping helpers preserve raw field lookups for future columns

- [ ] **Step 2: Run the helper test to verify it fails**

Run:

```powershell
node --experimental-strip-types scripts/owner-aware-asset-import-mapping.test.ts
```

Expected: FAIL because helper functions do not exist yet.

- [ ] **Step 3: Extend the import state layer**

Update `useAssetImportState.ts` to:

- surface owner warnings and blocking states
- support owner-field edits during review
- keep `Available` rows owner-free
- expose import-result counts for successful vs unresolved rows

- [ ] **Step 4: Extend the wizard UI**

Update `AssetImportWizard.tsx` so the review screen shows:

- submitted source snapshot
- resolved employee target
- warning badges for name/team mismatch
- editable owner field controls
- clear blocking state for unresolved employee rows

- [ ] **Step 5: Add and use helper functions**

In `assetImportModeConfig.ts`, add small focused helpers for:

- owner-aware field access
- derived computer-name formatting from asset codes
- multi-line comma-separated display formatting

- [ ] **Step 6: Run the helper test and frontend quality checks**

Run:

```powershell
node --experimental-strip-types scripts/owner-aware-asset-import-mapping.test.ts
npm run lint
npm run typecheck
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add src/features/assets/assetImportModeConfig.ts src/features/assets/useAssetImportState.ts src/features/assets/AssetImportWizard.tsx src/types/staff.ts scripts/owner-aware-asset-import-mapping.test.ts
git commit -m "feat: add owner-aware asset import review ui"
```

## Chunk 5: Derive employee-table `Computer Name` values from active laptop loans

### Task 5: Enrich employee query results and employee-table rendering

**Files:**
- Modify: `src-tauri/src/db/employee.rs`
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src/types/staff.ts`
- Modify: `src/features/employees/EmployeeView.tsx`
- Test: `src-tauri/src/db/employee.rs`

- [ ] **Step 1: Write failing employee-query tests**

Add tests covering:

- one employee with one active laptop loan -> one derived computer name
- one employee with multiple active laptop loans -> comma-separated multi-line value
- employees without active laptop loans keep blank derived value
- search/query output still includes the derived computer-name text

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml employee:: -- --nocapture
```

Expected: FAIL because employee queries do not derive laptop-backed computer names yet.

- [ ] **Step 3: Enrich backend employee records**

Update `employee.rs` so employee list/search responses derive `computerName` display from active laptop loans using:

- `computerName = "ASW" + assetCode`
- multiple values joined as `",\n"`

Do not persist the derived value into `employees.computername`.

- [ ] **Step 4: Render the derived multi-line cell in the employee table**

Update `EmployeeView.tsx` so the existing `Computer Name` column renders the derived multi-line text cleanly in desktop and mobile layouts.

- [ ] **Step 5: Run backend and frontend checks**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml employee:: -- --nocapture
npm run lint
npm run typecheck
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/db/employee.rs src-tauri/src/db/schema.rs src/types/staff.ts src/features/employees/EmployeeView.tsx
git commit -m "feat: derive employee computer names from laptop loans"
```

## Final Verification

- [ ] Run:

```powershell
node --experimental-strip-types scripts/owner-aware-asset-import-mapping.test.ts
npm run check:quality
npm run test:tauri
```

Expected:

- helper test passes
- frontend lint/typecheck/build pass
- tauri cargo checks/tests pass

- [ ] Manual smoke:
  - import `Available` sheet rows from `ExSource/AssetList.xlsx`
  - import `Laptop` sheet rows from `ExSource/AssetList.xlsx`
  - confirm unresolved rows stay out of the successful import total
  - confirm resolved `Laptop` rows create assigned assets and active loans
  - confirm employee-table `Computer Name` shows derived comma-separated multi-line laptop names

- [ ] Final commit if any verification fixes were needed

```powershell
git add <files>
git commit -m "test: finalize owner-aware asset import verification"
```
