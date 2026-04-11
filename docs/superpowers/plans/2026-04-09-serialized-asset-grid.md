# Serialized Asset Grid Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the serialized asset dashboard into one workbook-shaped shared table with persisted `Computer Name` and `Adapter Number`, sortable headers, drag/drop column order, resizable widths, and account-scoped layout persistence.

**Architecture:** Extend the current serialized asset source of truth instead of creating a parallel table model. Persist the missing serialized metadata in `assets`, teach the import pipeline to commit those values, then replace the current serialized dashboard grid with an account-scoped configurable column model that reuses the employee-table preference pattern while keeping `Status` and `Holder` derived from current asset-loan state.

**Tech Stack:** Tauri v2, Rust, rusqlite/SQLite, React 19, TypeScript, existing asset import pipeline, existing asset dashboard UI, existing account-scoped column preference pattern in `App.tsx`, targeted Rust unit tests, targeted Node UI helper tests, `npm run check:quality`.

---

## File Structure

- Modify: `src-tauri/src/db/schema.rs`
  - add persisted serialized metadata columns for `computer_name` and `adapter_number`
- Modify: `src-tauri/src/db/mod.rs`
  - migrate existing databases to the new serialized asset columns
- Modify: `src-tauri/src/db/asset_import.rs`
  - commit normalized `computer_name` and `adapter_number` into official serialized assets
- Modify: `src-tauri/src/db/asset.rs`
  - expose the wider serialized dashboard row shape, add column-specific sort handling, and keep holder/status derived from active loans
- Modify: `src-tauri/src/lib.rs`
  - thread any new serialized sorting/filter arguments through Tauri commands if the current command boundary requires it
- Modify: `src/types/staff.ts`
  - expand serialized dashboard row contracts and any sort/layout payloads
- Modify: `src/services/staff-api.ts`
  - thread new dashboard query args or return fields through the typed IPC wrapper
- Modify: `src/features/assets/assetImportModeConfig.ts`
  - recognize `Computer Name`, `Adapter number`, and typo aliases while keeping normalized UI labels separate
- Modify: `src/features/assets/assetDashboardCopy.ts`
  - centralize normalized column labels and any display helpers needed by the new shared grid
- Modify: `src/features/assets/useAssetDashboardState.ts`
  - own serialized-grid sort state and load the wider row shape
- Modify: `src/features/assets/AssetDashboard.tsx`
  - replace the current technical serialized table with the approved shared-grid columns, sortable headers, drag/drop order, and resizable widths
- Create: `src/features/assets/serializedAssetGridConfig.ts`
  - define the default column order, labels, widths, and persistence keys for the serialized table
- Create: `src/features/assets/useSerializedAssetGridState.ts`
  - reuse the employee-table preference pattern for this dashboard table without leaking that complexity into the main dashboard hook
- Modify: `scripts/asset-dashboard-formatting.test.ts`
  - verify normalized labels, fallback `Computer Name`, and any client-side column config helpers
- Create: `scripts/serialized-asset-grid.test.ts`
  - verify serialized-grid default order / persistence-key helpers / label normalization if that logic becomes too large for the existing helper test

## Chunk 1: Persist the missing serialized metadata

### Task 1: Add schema support for `computer_name` and `adapter_number`

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/src/db/mod.rs`
- Test: `src-tauri/src/db/asset.rs`

- [ ] **Step 1: Write the failing Rust tests**

Add targeted tests in `src-tauri/src/db/asset.rs` for:

- persisting `computer_name`
- persisting `adapter_number`
- reading both fields back from stored asset records

- [ ] **Step 2: Run the targeted tests to confirm the gap**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: FAIL because the new persisted asset columns do not exist yet.

- [ ] **Step 3: Implement the schema changes**

Add to `src-tauri/src/db/schema.rs` and mirror in migrations in `src-tauri/src/db/mod.rs`:

- `assets.computer_name TEXT NULL`
- `assets.adapter_number TEXT NULL`

Ensure upgrades for existing databases are additive and idempotent.

- [ ] **Step 4: Re-run the targeted tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: PASS for the new persistence coverage.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/db/schema.rs src-tauri/src/db/mod.rs src-tauri/src/db/asset.rs
git commit -m "feat: persist serialized computer and adapter metadata"
```

## Chunk 2: Teach import to commit workbook-shaped serialized fields

### Task 2: Normalize and commit `Computer Name` and `Adapter number`

**Files:**
- Modify: `src/features/assets/assetImportModeConfig.ts`
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src/types/staff.ts`
- Modify: `src/services/staff-api.ts`
- Modify: `scripts/asset-dashboard-formatting.test.ts`

- [ ] **Step 1: Write the failing tests**

Add backend tests in `src-tauri/src/db/asset_import.rs` for:

- importing explicit `Computer Name` from workbook rows
- falling back to `ASW + Assetcode` when imported `Computer Name` is blank
- persisting `Adapter number` into official serialized rows
- still accepting typo headers such as `Serrial Number` and `Usuage Location`

Add or extend Node helper assertions for:

- `Computer Name` fallback behavior
- normalized UI labels (`ID`, `Serial Number`, `Usage Location`)

- [ ] **Step 2: Run the focused tests to verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: FAIL because import does not yet commit the new serialized fields end to end.

- [ ] **Step 3: Extend import alias coverage**

Update `src/features/assets/assetImportModeConfig.ts` to recognize:

- `Computer Name`
- `Adapter number`
- `Assetcode`
- `Asset code`
- `Serrial Number`
- `Usuage Location`

Keep UI labels normalized even though import accepts workbook typos.

- [ ] **Step 4: Commit the new fields in Rust**

Update `src-tauri/src/db/asset_import.rs` so serialized-row commit logic:

- stores imported `computer_name` when present
- computes `ASW + Assetcode` when `computer_name` is blank
- stores imported `adapter_number`
- keeps existing holder/status semantics intact

- [ ] **Step 5: Thread any new staged/offical fields through the IPC contracts**

Update `src/types/staff.ts` and `src/services/staff-api.ts` if the dashboard/import state needs the new fields in staged row or official row payloads.

- [ ] **Step 6: Re-run the focused tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: PASS for import fallback and alias handling.

- [ ] **Step 7: Commit**

```powershell
git add src/features/assets/assetImportModeConfig.ts src-tauri/src/db/asset_import.rs src/types/staff.ts src/services/staff-api.ts scripts/asset-dashboard-formatting.test.ts
git commit -m "feat: import serialized computer and adapter fields"
```

## Chunk 3: Expose the workbook-shaped serialized row model

### Task 3: Expand the serialized dashboard backend/query contract

**Files:**
- Modify: `src-tauri/src/db/asset.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types/staff.ts`
- Modify: `src/services/staff-api.ts`
- Test: `src-tauri/src/db/asset.rs`

- [ ] **Step 1: Write the failing Rust tests**

Add or extend serialized dashboard tests in `src-tauri/src/db/asset.rs` so they assert:

- `ID` still reads from `asset_code`
- `Computer Name` comes from stored `computer_name`
- `Asset Name` comes from `display_name`
- `Serial Number` comes from `serial_number`
- `Adapter Number` comes from `adapter_number`
- `Usage Location` comes from `usage_location`
- `Note` comes from `notes`
- `Status` and `Holder` still reflect the active-loan state correctly

- [ ] **Step 2: Run the focused tests to confirm they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: FAIL because the current serialized row model does not yet expose the full workbook-shaped field set.

- [ ] **Step 3: Expand the Rust row contract and query**

Update `list_asset_dashboard_serialized_conn` and related structs in `src-tauri/src/db/asset.rs` to return the widened row shape:

- `assetCode`
- `category`
- `computerName`
- `displayName`
- `model`
- `serialNumber`
- `adapterNumber`
- `usageLocation`
- `notes`
- `status`
- `holder`

Also add explicit sort-key handling for the approved column set.

- [ ] **Step 4: Thread the new contract through the command layer**

Update `src-tauri/src/lib.rs`, `src/types/staff.ts`, and `src/services/staff-api.ts` so the frontend receives the widened serialized row contract and can request sorted data.

- [ ] **Step 5: Re-run the focused tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: PASS for the widened serialized dashboard row coverage.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/db/asset.rs src-tauri/src/lib.rs src/types/staff.ts src/services/staff-api.ts
git commit -m "feat: expose workbook-aligned serialized dashboard rows"
```

## Chunk 4: Build the shared serialized grid UI

### Task 4: Replace the current serialized table with the approved columns and sorts

**Files:**
- Modify: `src/features/assets/assetDashboardCopy.ts`
- Modify: `src/features/assets/useAssetDashboardState.ts`
- Modify: `src/features/assets/AssetDashboard.tsx`
- Create: `src/features/assets/serializedAssetGridConfig.ts`
- Modify: `scripts/asset-dashboard-formatting.test.ts`
- Create: `scripts/serialized-asset-grid.test.ts`

- [ ] **Step 1: Write the failing front-end helper tests**

Add tests that lock:

- the default column order
- normalized labels (`ID`, `Serial Number`, `Usage Location`)
- presence of default trailing columns `Status` and `Holder`
- sort cycling helpers for serialized-grid headers

- [ ] **Step 2: Run the focused helper tests to confirm they fail**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
node --experimental-strip-types scripts/serialized-asset-grid.test.ts
```

Expected: FAIL because the grid config and helper layer do not yet exist.

- [ ] **Step 3: Create a dedicated serialized-grid config module**

In `src/features/assets/serializedAssetGridConfig.ts`, define:

- stable column keys
- default order
- default widths
- normalized labels
- per-column sort keys
- helper for future label overrides

- [ ] **Step 4: Update the dashboard state**

Update `src/features/assets/useAssetDashboardState.ts` so it owns:

- serialized sort key / direction
- reload behavior when sort changes
- any row-shape transformation needed for the new grid

- [ ] **Step 5: Replace the table UI in `AssetDashboard.tsx`**

Render one shared serialized table with:

- `ID`
- `Category`
- `Computer Name`
- `Asset Name`
- `Model`
- `Serial Number`
- `Adapter Number`
- `Usage Location`
- `Note`
- `Status`
- `Holder`

Keep cells empty where a row/category does not use a field.

Add clickable sortable headers for each column.

- [ ] **Step 6: Re-run the focused helper tests**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
node --experimental-strip-types scripts/serialized-asset-grid.test.ts
```

Expected: PASS for labels/order/sort helper coverage.

- [ ] **Step 7: Commit**

```powershell
git add src/features/assets/assetDashboardCopy.ts src/features/assets/useAssetDashboardState.ts src/features/assets/AssetDashboard.tsx src/features/assets/serializedAssetGridConfig.ts scripts/asset-dashboard-formatting.test.ts scripts/serialized-asset-grid.test.ts
git commit -m "feat: add workbook-style serialized asset grid"
```

## Chunk 5: Persist per-user order and width preferences

### Task 5: Reuse the account-scoped column preference model for the serialized grid

**Files:**
- Create: `src/features/assets/useSerializedAssetGridState.ts`
- Modify: `src/features/assets/AssetDashboard.tsx`
- Modify: `src/App.tsx`
- Modify: `scripts/serialized-asset-grid.test.ts`

- [ ] **Step 1: Write the failing preference/persistence helper tests**

Add focused tests for:

- the serialized-grid storage key including the active account scope
- default-width reconciliation
- restoring saved order/widths with new columns appended safely

- [ ] **Step 2: Run the helper test to confirm the gap**

Run:

```powershell
node --experimental-strip-types scripts/serialized-asset-grid.test.ts
```

Expected: FAIL because the serialized-grid preference model does not exist yet.

- [ ] **Step 3: Implement the grid preference hook**

Create `src/features/assets/useSerializedAssetGridState.ts` to manage:

- ordered columns
- hidden columns if needed by the current drawer pattern
- width map
- drag/drop reorder
- resize handlers
- account-scoped localStorage keys

Prefer borrowing the employee-table pattern rather than copying unrelated employee-specific behavior wholesale.

- [ ] **Step 4: Wire the hook into the dashboard**

Update `AssetDashboard.tsx` and `src/App.tsx` as needed so the serialized grid:

- loads the current logged-in account scope
- applies saved order/widths
- saves changes automatically for that account

- [ ] **Step 5: Re-run the helper test**

Run:

```powershell
node --experimental-strip-types scripts/serialized-asset-grid.test.ts
```

Expected: PASS for serialized-grid preference persistence coverage.

- [ ] **Step 6: Commit**

```powershell
git add src/features/assets/useSerializedAssetGridState.ts src/features/assets/AssetDashboard.tsx src/App.tsx scripts/serialized-asset-grid.test.ts
git commit -m "feat: persist serialized asset grid layout per account"
```

## Chunk 6: Full verification and manual smoke test

### Task 6: Verify the slice end to end

**Files:**
- Modify: `docs/superpowers/specs/2026-04-09-serialized-asset-grid-design.md`
- Modify: `docs/superpowers/plans/2026-04-09-serialized-asset-grid.md`

- [ ] **Step 1: Run the focused backend tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run the focused front-end helper tests**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
node --experimental-strip-types scripts/serialized-asset-grid.test.ts
```

Expected: PASS.

- [ ] **Step 3: Run the repo quality gate**

Run:

```powershell
npm run check:quality
```

Expected: PASS.

- [ ] **Step 4: Manual smoke test with real files**

Use the real workbook(s) and verify:

- imported laptop rows show explicit `Computer Name` when present
- imported rows without `Computer Name` fall back to `ASW + Assetcode`
- `Adapter Number` appears in the serialized table after import
- `Status` and `Holder` remain correct
- sort works on the approved columns
- drag/drop order persists for the logged-in account
- width resize persists for the logged-in account

- [ ] **Step 5: Commit any final doc touch-ups**

```powershell
git add docs/superpowers/specs/2026-04-09-serialized-asset-grid-design.md docs/superpowers/plans/2026-04-09-serialized-asset-grid.md
git commit -m "docs: finalize serialized asset grid execution docs"
```

## Definition of done

- serialized dashboard table matches the approved workbook-shaped default column set
- `Computer Name` and `Adapter Number` persist as official serialized asset data
- imported typo headers still work
- `Status` and `Holder` remain correct
- users can sort, reorder, and resize serialized columns
- layout persists per logged-in account
- `npm run check:quality` passes

