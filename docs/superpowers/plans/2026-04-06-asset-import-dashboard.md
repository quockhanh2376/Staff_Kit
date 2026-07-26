# Asset Import Dashboard Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the current asset-import foundation into an Asset Dashboard inside Settings that supports the three real workbook shapes (`AssetList.xlsx`, `Monitor.xlsx`, and `Mouse-Key.xlsx`), exposes serialized and quantity inventory summaries, and gives IT a category-management surface without creating a second asset source of truth.

**Architecture:** Build on the existing staged import wizard and current database tables instead of introducing a parallel dashboard model. Serialized inventory continues to live in `assets + asset_loans`, quantity inventory continues to live in `stock_items`, and the dashboard becomes a read/operate surface that layers summary/detail queries plus category CRUD on top of those tables. Add only the schema needed for monitor metadata (`usage_location`, `display_name_short`) and multi-prefix category recognition.

**Tech Stack:** Tauri v2, Rust, rusqlite/SQLite, React 19, TypeScript, existing asset import wizard/state hooks, existing Settings view, targeted Rust unit tests, targeted Node helper tests, `npm run check:quality`, `npm run test:tauri`.

---

## File Structure

- Modify: `src-tauri/src/db/schema.rs`
  - add schema definitions for dashboard metadata and category-prefix child records
- Modify: `src-tauri/src/db/mod.rs`
  - add migrations/backfill for new asset columns, category-prefix table, and seed data updates
- Modify: `src-tauri/src/db/asset.rs`
  - add category-prefix helpers, dashboard summary/detail queries, stock quantity updates, and category CRUD
- Modify: `src-tauri/src/db/asset_import.rs`
  - extend workbook mapping/classification for the three real file shapes and commit rows into the correct official tables
- Modify: `src-tauri/src/lib.rs`
  - register new Tauri commands for dashboard summary/detail/category operations
- Modify: `src/services/staff-api.ts`
  - expose typed IPC calls for dashboard data and category management
- Modify: `src/types/staff.ts`
  - add TypeScript contracts for dashboard summary/detail records, category prefixes, and new asset metadata
- Modify: `src/features/assets/assetImportModeConfig.ts`
  - add alias coverage and helper functions for dashboard workbook headers such as `Usuage Location` and `Quantity `
- Modify: `src/features/assets/useAssetImportState.ts`
  - keep wizard logic focused on import semantics while threading through new staging fields
- Modify: `src/features/assets/AssetImportWizard.tsx`
  - render any extra review fields/errors needed by dashboard workbook rows without taking on dashboard read-state concerns
- Create: `src/features/assets/useAssetDashboardState.ts`
  - own dashboard summary/detail/category-management fetch/mutate state
- Create: `src/features/assets/AssetDashboard.tsx`
  - render the Settings-level dashboard cards, tabs, and actions
- Create: `src/features/assets/assetDashboardCopy.ts`
  - hold dashboard labels/formatting helpers so copy does not bloat the component
- Modify: `src/features/settings/SettingsView.tsx`
  - mount the dashboard and keep the existing import/manual entry points reachable from it
- Create: `scripts/asset-dashboard-formatting.test.ts`
  - verify front-end formatting helpers for computer/display names and usage-location labels

## Chunk 1: Data model and category foundation

### Task 1: Add the schema needed for monitor metadata and multi-prefix categories

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/src/db/mod.rs`
- Test: `src-tauri/src/db/asset.rs`

- [ ] **Step 1: Write the failing Rust tests**

Add targeted tests in `src-tauri/src/db/asset.rs` for:

- `assets` rows persisting `usage_location`
- `assets` rows persisting `display_name_short`
- one logical category supporting multiple active prefixes
- duplicate active prefixes being rejected across categories

- [ ] **Step 2: Run the targeted tests to confirm the gap**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: FAIL because the new columns/table and validation do not exist yet.

- [ ] **Step 3: Implement the schema changes**

Add to `src-tauri/src/db/schema.rs` and mirror in `src-tauri/src/db/mod.rs`:

- `assets.usage_location TEXT NULL`
- `assets.display_name_short TEXT NULL`
- new `asset_category_prefixes` table with:
  - `id INTEGER PRIMARY KEY`
  - `category_id INTEGER NOT NULL`
  - `prefix_value TEXT NOT NULL`
  - `is_primary INTEGER NOT NULL DEFAULT 0`
  - `created_at TEXT NOT NULL`
  - `updated_at TEXT NOT NULL`
- unique index for active prefix uniqueness logic

- [ ] **Step 4: Seed and backfill the category foundation**

Update the category seed/migration code so:

- logical `Laptop` remains one category
- laptop prefixes include `VNLAP`, `VNMACPRO`, `VNIMACPRO`, `VNMACAIR`
- monitor prefixes include `VNMON`
- old `asset_categories.prefix_code` data is migrated into the new child table for existing databases

- [ ] **Step 5: Add Rust helpers around category prefixes**

In `src-tauri/src/db/asset.rs`, add small helpers to:

- list prefixes per category
- resolve category by asset-code prefix
- enforce duplicate-prefix validation before insert/update

- [ ] **Step 6: Re-run the targeted tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: PASS for the new schema/category tests.

- [ ] **Step 7: Commit**

```powershell
git add src-tauri/src/db/schema.rs src-tauri/src/db/mod.rs src-tauri/src/db/asset.rs
git commit -m "feat: add asset dashboard schema foundation"
```

## Chunk 2: Import the three workbook shapes into the existing asset system

### Task 2: Extend workbook parsing and staging rules

**Files:**
- Modify: `src/features/assets/assetImportModeConfig.ts`
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src/types/staff.ts`
- Modify: `src/services/staff-api.ts`
- Create: `scripts/asset-dashboard-formatting.test.ts`

- [ ] **Step 1: Write the failing tests for workbook parsing**

Add backend tests in `src-tauri/src/db/asset_import.rs` that cover:

- `AssetList.xlsx`-style serialized rows with and without `StaffID`
- `Monitor.xlsx` rows that expose `Usuage Location`
- `Mouse-Key.xlsx` rows that stage quantity rows from `Quantity `
- preserving raw source columns such as `Adapter number`

Add Node helper assertions in `scripts/asset-dashboard-formatting.test.ts` for:

- usage-location label normalization
- `display_name_short` fallback behavior
- asset-code-to-computer-name formatting helper behavior

- [ ] **Step 2: Run the focused tests to confirm they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: FAIL because the parser and helper layer do not yet understand the dashboard workbook shapes.

- [ ] **Step 3: Add header alias coverage in the front-end mapping layer**

Update `src/features/assets/assetImportModeConfig.ts` so the wizard can detect:

- `Assetcode`
- `Asset code`
- `Serrial Number`
- `Usuage Location`
- `Quantity `

Also add a new wizard field key for `usageLocation` if the current mapping model needs one for review/edit.

- [ ] **Step 4: Extend staged-row parsing in Rust**

In `src-tauri/src/db/asset_import.rs`:

- classify serialized rows by asset category + presence of `StaffID`, not by laptop sheet name alone
- keep `Available` serialized rows as `in_stock`
- keep populated `StaffID` serialized rows as assigned rows requiring owner resolution
- classify `Mouse-Key.xlsx` rows as quantity-only rows
- persist normalized `usage_location` and `display_name_short`
- keep `Adapter number` only inside raw row payload

- [ ] **Step 5: Thread the new staging fields through the IPC contracts**

Update `src/types/staff.ts` and `src/services/staff-api.ts` so staged rows and batch detail responses include:

- `usageLocation`
- `displayNameShort`
- any dashboard-specific validation warnings needed by the review UI

- [ ] **Step 6: Re-run the focused tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: PASS for workbook parsing and helper coverage.

- [ ] **Step 7: Commit**

```powershell
git add src/features/assets/assetImportModeConfig.ts src-tauri/src/db/asset_import.rs src/types/staff.ts src/services/staff-api.ts scripts/asset-dashboard-formatting.test.ts
git commit -m "feat: extend asset import parsing for dashboard workbooks"
```

### Task 3: Commit staged rows into the correct official tables

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src-tauri/src/db/asset.rs`
- Test: `src-tauri/src/db/asset_import.rs`

- [ ] **Step 1: Write the failing commit-path tests**

Add Rust tests in `src-tauri/src/db/asset_import.rs` for:

- assigned laptop rows creating `assets + asset_loans`
- assigned monitor rows creating `assets + asset_loans` plus `usage_location`
- available serialized rows creating `assets` with `status = 'in_stock'`
- quantity rows landing in `stock_items`
- unresolved assigned rows staying out of the successful import count

- [ ] **Step 2: Run the targeted tests to confirm they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: FAIL because the current commit path is not generalized for monitor/quantity dashboard semantics.

- [ ] **Step 3: Implement the split commit behavior**

Update `import_asset_import_batch_valid_rows_conn` and supporting helpers so:

- serialized assigned rows create/update `assets`, set `status = 'assigned'`, and create active loans
- serialized available rows create/update `assets` with `status = 'in_stock'`
- quantity rows create/update `stock_items`
- duplicate serialized asset codes, duplicate serial numbers, and duplicate active loans are still blocked

- [ ] **Step 4: Keep import-result accounting explicit**

Make sure commit results report:

- number of serialized rows imported
- number of quantity rows imported
- unresolved/blocked row count still left in review

This is needed so IT can compare the success count against the source workbook and adjust remaining bad rows manually.

- [ ] **Step 5: Re-run the targeted tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
```

Expected: PASS for the new commit semantics.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/db/asset_import.rs src-tauri/src/db/asset.rs
git commit -m "feat: commit dashboard workbook rows into official asset tables"
```

## Chunk 3: Dashboard read APIs and Settings integration

### Task 4: Add the backend queries and commands for dashboard data

**Files:**
- Modify: `src-tauri/src/db/asset.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/services/staff-api.ts`
- Modify: `src/types/staff.ts`
- Test: `src-tauri/src/db/asset.rs`

- [ ] **Step 1: Write the failing Rust tests for dashboard read models**

Add tests in `src-tauri/src/db/asset.rs` covering:

- summary cards returning the correct serialized and quantity counts
- serialized detail rows including holder and usage location
- quantity detail rows returning category/name/model/warehouse counts
- manual quantity updates mutating only `stock_items`

- [ ] **Step 2: Run the focused tests to verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: FAIL because the dashboard read-model queries do not exist yet.

- [ ] **Step 3: Implement backend queries in `asset.rs`**

Add focused functions such as:

- `get_asset_dashboard_summary_conn`
- `list_asset_dashboard_serialized_conn`
- `list_asset_dashboard_quantity_conn`
- `update_stock_item_quantity_conn`

Keep serialized data sourced from `assets + asset_loans` and quantity data sourced from `stock_items`.

- [ ] **Step 4: Register and type the new commands**

Wire the new queries through:

- `src-tauri/src/lib.rs`
- `src/services/staff-api.ts`
- `src/types/staff.ts`

Use precise record types for summary cards, serialized rows, quantity rows, and category-prefix payloads.

- [ ] **Step 5: Re-run the focused tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: PASS for the summary/detail/update tests.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/db/asset.rs src-tauri/src/lib.rs src/services/staff-api.ts src/types/staff.ts
git commit -m "feat: add asset dashboard backend queries"
```

### Task 5: Build the Settings-level dashboard UI

**Files:**
- Create: `src/features/assets/useAssetDashboardState.ts`
- Create: `src/features/assets/AssetDashboard.tsx`
- Create: `src/features/assets/assetDashboardCopy.ts`
- Modify: `src/features/settings/SettingsView.tsx`
- Modify: `src/features/assets/AssetImportWizard.tsx`
- Modify: `src/features/assets/useAssetImportState.ts`
- Test: `scripts/asset-dashboard-formatting.test.ts`

- [ ] **Step 1: Write or extend the failing UI helper assertions**

Extend `scripts/asset-dashboard-formatting.test.ts` so it covers:

- summary-card label helpers
- usage-location display labels
- multi-line display-name formatting helpers used by the dashboard tables

- [ ] **Step 2: Run the helper test to verify the missing behavior**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: FAIL because the dashboard UI helpers do not exist yet.

- [ ] **Step 3: Create the isolated dashboard state hook**

Add `src/features/assets/useAssetDashboardState.ts` to load:

- summary cards
- serialized tab rows
- quantity tab rows
- refresh state
- manual quantity update action

Do not move unrelated import-wizard state into this hook.

- [ ] **Step 4: Build `AssetDashboard.tsx`**

Render:

- header and action row
- summary cards
- `Serialized` and `Quantity` tabs
- import/manual entry points that call the existing asset import state actions

Keep the layout full-width and consistent with the current Settings visual language.

- [ ] **Step 5: Integrate the dashboard into `SettingsView.tsx`**

Mount the dashboard in Settings and keep the old standalone asset-import entry visible only as long as the new dashboard path needs a transition state.

- [ ] **Step 6: Re-run front-end verification**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
npm run check:quality
```

Expected: PASS, including lint, typecheck, build, and tauri cargo check from `check:quality`.

- [ ] **Step 7: Commit**

```powershell
git add src/features/assets/useAssetDashboardState.ts src/features/assets/AssetDashboard.tsx src/features/assets/assetDashboardCopy.ts src/features/settings/SettingsView.tsx src/features/assets/AssetImportWizard.tsx src/features/assets/useAssetImportState.ts scripts/asset-dashboard-formatting.test.ts
git commit -m "feat: add asset dashboard ui in settings"
```

## Chunk 4: Category management and hardening

### Task 6: Add IT-facing category CRUD and finish end-to-end verification

**Files:**
- Modify: `src-tauri/src/db/asset.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/services/staff-api.ts`
- Modify: `src/types/staff.ts`
- Modify: `src/features/assets/useAssetDashboardState.ts`
- Modify: `src/features/assets/AssetDashboard.tsx`
- Test: `src-tauri/src/db/asset.rs`
- Test: `scripts/asset-dashboard-formatting.test.ts`

- [ ] **Step 1: Write the failing category-management tests**

Add Rust tests for:

- creating a category with multiple prefixes
- editing category labels/tracking mode
- deactivating categories already referenced by data instead of deleting them
- duplicate active prefix rejection during create/update

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
```

Expected: FAIL because category CRUD and deactivation rules are not implemented yet.

- [ ] **Step 3: Implement backend CRUD for dashboard category management**

In `src-tauri/src/db/asset.rs` and `src-tauri/src/lib.rs`, add commands for:

- create category
- update category
- deactivate category
- list category details with prefixes

Keep prefix validation in the backend so the UI cannot bypass it.

- [ ] **Step 4: Add the dashboard management UI**

Update `AssetDashboard.tsx` and `useAssetDashboardState.ts` so IT can:

- open a category-management panel
- create/edit categories
- add/remove prefixes
- see duplicate-prefix validation before save

- [ ] **Step 5: Run the full verification suite**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml asset_import:: -- --nocapture
npm run check:quality
npm run test:tauri
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: PASS across the targeted backend tests, the full quality gate, the Tauri test suite, and the dashboard helper test.

- [ ] **Step 6: Manual smoke-test with the real workbooks**

Use:

- `ExSource/AssetList.xlsx`
- `ExSource/Monitor.xlsx`
- `ExSource/Mouse-Key.xlsx`

Verify:

- assigned laptop rows create active loans
- available laptop rows land in stock
- monitor rows preserve normalized `usage_location`
- quantity-only rows update `stock_items`
- dashboard counts match the resulting tables
- category edits change import recognition as expected

- [ ] **Step 7: Commit**

```powershell
git add src-tauri/src/db/asset.rs src-tauri/src/lib.rs src/services/staff-api.ts src/types/staff.ts src/features/assets/useAssetDashboardState.ts src/features/assets/AssetDashboard.tsx scripts/asset-dashboard-formatting.test.ts
git commit -m "feat: add asset dashboard category management"
```

## Execution notes

- Do not add mirrored serialized stock writes into `stock_items` from Borrow / Return approval code. Serialized assignment truth already lives in `assets + asset_loans`.
- If quantity-tracked peripherals later need Borrow / Return workflow, write a separate spec for that instead of overloading this dashboard slice.
- Prefer extracting focused helper modules over adding more bulk directly into `AssetImportWizard.tsx` and `useAssetImportState.ts`.
- Keep raw workbook source values intact in staged rows so future header mapping and review fields can grow without re-importing source files.

## Definition of done

- Spec-aligned dashboard docs exist and the implementation follows them.
- All three real workbook shapes import correctly.
- Dashboard summary/detail views load inside Settings.
- IT can adjust quantity stock and manage category prefixes from the dashboard.
- `npm run check:quality` and `npm run test:tauri` pass after the slice lands.
