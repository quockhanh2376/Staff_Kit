# Asset Import Dashboard Design

## Goal

Add an `Asset Dashboard` section inside `Settings` so IT can manage imported assets from one place, using the existing staged import workflow as the foundation instead of building a second asset-management system in parallel.

This slice must support the three real workbook shapes now in use:

- `AssetList.xlsx`
- `Monitor.xlsx`
- `Mouse-Key.xlsx`

It must also turn the current import tooling into a more complete operational surface with dashboard summary cards, detail tables, and category-management controls.

## Current Baseline

`v2.0.3` already has:

- owner-aware serialized asset import for laptop-style rows
- quantity import into `stock_items`
- active-loan-based employee `Computer Name` display
- seeded `asset_categories`
- staged batch review in SQLite

This means the dashboard slice is not a greenfield import rewrite. It is a consolidation and extension of the existing asset/import foundation.

## Chosen Approach

### Recommended approach

Keep one asset-management system:

- extend the existing staged import wizard
- add a dashboard read layer on top of the current `assets`, `asset_loans`, `asset_categories`, and `stock_items` tables
- add the minimum extra schema needed for new monitor/dashboard metadata
- add category-prefix metadata in a way that supports multiple prefixes per logical category

This keeps the source of truth stable:

- serialized ownership remains in `assets + asset_loans`
- quantity stock remains in `stock_items`
- the dashboard becomes a read-and-operate surface, not a second persistence model

### Rejected alternatives

#### 1. Separate dashboard data model

Rejected because it would duplicate the existing import/batch model and immediately create drift between the dashboard and the wizard.

#### 2. Mirror serialized borrow/return state into `stock_items`

Rejected because current Borrow / Return approval works on serialized `assets`, not on quantity rows in `stock_items`. Writing serialized state into `stock_items` would create a second source of truth and make reconciliation harder.

#### 3. Treat `MACPRO`, `IMACPRO`, and `MACAIR` as separate business categories

Rejected because the current business meaning is still `Laptop`. Those values are code families and import-recognition prefixes, not a reason to split the logical category model.

## Scope

In scope:

- add a full-width `Asset Dashboard` block to `Settings`
- support the three workbook structures currently used in operations
- generalize serialized import so owner-aware rows are not limited to laptop sheets only
- store monitor-specific metadata needed by the dashboard
- expose dashboard read APIs for serialized assets and quantity stock
- provide IT-facing category management with prefix uniqueness validation
- keep using the existing import wizard and manual-add flow as the operational entry points

Out of scope:

- new top-level navigation tab
- employee/team assignment reports
- direct dashboard editing for active loans
- physical audit workflows
- adapter tracking as first-class stored data
- rewriting the existing Borrow / Return phone flow

## Workbook Semantics

### 1. `AssetList.xlsx`

Observed shape:

- `Laptop` sheet
- `Available` sheet
- columns include `StaffID`, `Tên Nhân Viên`, `Team`, `Phone Number`, `Assetcode`, `Category`, `Computer Name`, `Asset Name`, `Model`, `Serrial Number`, `Adapter number`, `Note`

Operational meaning:

- rows with `StaffID` populated are already assigned serialized assets
- rows with empty `StaffID` are serialized assets in stock
- `Computer Name` and `Asset Name` in this file are operational display values, not the primary key
- `Adapter number` should stay in raw source payload only for now

### 2. `Monitor.xlsx`

Observed shape:

- `Monitor` sheet
- `Available` sheet
- columns include `Usuage Location` plus the same owner snapshot columns

Operational meaning:

- rows with `StaffID` populated are already assigned serialized monitor assets
- rows with empty `StaffID` are serialized monitor assets in stock
- `Usuage Location` is a source typo that the importer must still recognize
- the imported monitor `Asset Name` already carries the short display form like `Mon709`

### 3. `Mouse-Key.xlsx`

Observed shape:

- one `Mouse-Keyboard-Headset` sheet
- columns include `Assetcode`, `Category`, `Asset Name`, `Model`, `Quantity`, `Note`

Operational meaning:

- this is quantity stock, not serialized assignment
- no owner resolution
- imported rows land in `stock_items`

## Data Model Design

### Serialized source of truth

Serialized assets continue to live in:

- `assets`
- `asset_loans`

Assigned serialized rows from import must create:

- the `assets` record
- an active `asset_loan` when the row is owner-resolved and represents an already-issued asset

### Quantity source of truth

Quantity stock continues to live in:

- `stock_items`

The dashboard must read quantity metrics from `stock_items`, not infer them from serialized assets.

### New asset metadata

Add these columns to `assets`:

- `usage_location TEXT NULL`
- `display_name_short TEXT NULL`

Rules:

- `usage_location` is only populated for monitor-style assets in this slice
- `display_name_short` is used for compact dashboard display, especially for monitor rows such as `Mon709`

### Category prefix model

The current `asset_categories.prefix_code` field can hold only one value, but the next slice needs multiple code families under the same logical category, especially for `Laptop`.

To support that cleanly, add a new child table for category prefixes, for example:

- `asset_category_prefixes`
  - `id`
  - `category_id`
  - `prefix_value`
  - `is_primary`
  - `updated_at`

Usage:

- keep `asset_categories` as the logical category layer
- store multiple import-recognition prefixes per category
- enforce uniqueness of active prefixes across categories

Initial laptop-family prefixes should include:

- `VNLAP`
- `VNMACPRO`
- `VNIMACPRO`
- `VNMACAIR`

Monitor-family prefixes should include:

- `VNMON`

This avoids forcing `MACPRO`, `IMACPRO`, and `MACAIR` into separate business categories.

## Derived Naming Rules

### Computer name

Laptop-like serialized assets keep the existing derived rule:

- `computerName = "ASW" + assetCode`

Examples:

- `VNLAP122` -> `ASWVNLAP122`
- `VNMACPRO003` -> `ASWVNMACPRO003`
- `VNIMACPRO016` -> `ASWVNIMACPRO016`
- `VNMACAIR001` -> `ASWVNMACAIR001`

This rule stays read-derived and does not become a second persisted ownership field.

### Display short name

`display_name_short` is a compact dashboard label.

Rules:

- if the file already provides a compact name like `Mon709`, preserve it after normalization
- if not present, derive from asset-code suffix when possible
- for laptops, the dashboard may fall back to `display_name` if no short form is needed

### Usage location

Normalize imported monitor usage-location text into canonical values:

- `office`
- `home`
- `null`

UI labels should display friendly Vietnamese copy:

- `Tại CTY`
- `Tại Nhà`

The long helper phrase found in the sample workbook should not be persisted as the canonical value.

## Import Pipeline Design

### Keep one import wizard

Do not create a second import wizard for the dashboard.

Instead:

- keep the current `AssetImportWizard`
- extend its mapping/inspection logic
- let the new dashboard trigger the same wizard and manual-add flow

### Serialized row classification

Current owner-aware import is laptop-focused. The new slice should generalize that behavior:

- serialized row with empty `StaffID` -> `in_stock`
- serialized row with populated `StaffID` -> assigned serialized row requiring owner resolution

This applies to both laptop and monitor style workbooks.

### Quantity row classification

Rows from `Mouse-Key.xlsx` remain quantity-only:

- no owner resolution
- import into `stock_items`
- existing quantity validation remains the baseline

### Header alias support

The importer must recognize both correct and incorrect source headers, including:

- `Assetcode`
- `Asset code`
- `Serrial Number`
- `Usuage Location`
- `Quantity `

Header aliases belong in the import-configuration layer, not scattered through the parser.

### Raw source preservation

Keep the existing rule:

- original raw source values remain stored in staged rows
- new fields such as `usage_location` or `adapter number` can be parsed without losing raw traceability

`Adapter number` should remain raw-only in this slice.

### Commit behavior

#### Serialized assigned row

When valid and owner-resolved:

- create or upsert serialized asset
- persist `usage_location` and `display_name_short` when applicable
- create active loan
- mark asset as `assigned`

#### Serialized available row

When valid:

- create or upsert serialized asset
- no active loan
- mark asset as `in_stock`

#### Quantity row

When valid:

- create or update `stock_items`
- no asset loan
- no serialized asset row unless a future slice requires one

## Dashboard Read Model

### Summary cards

The first dashboard version should show a compact card row with:

- total serialized assets
- serialized assets in stock
- serialized assets assigned
- total quantity on hand
- total quantity assigned

These values must come from the existing source-of-truth tables:

- serialized counts from `assets`
- quantity counts from `stock_items`

### Detail tabs

The detail area should have two tabs:

- `Serialized`
- `Quantity`

#### Serialized tab

Show fields that matter operationally:

- asset code
- category
- display name
- short name
- model
- serial number
- usage location
- status
- current holder when an active loan exists

#### Quantity tab

Show:

- category
- item name
- brand
- model
- warehouse
- quantity on hand
- assigned quantity
- note

The quantity tab is the place for manual quantity correction in this slice.

## Category Management UI

The dashboard must include a `Manage Categories` action.

This UI should support:

- create category
- edit category label and tracking mode
- activate/deactivate category
- manage multiple code prefixes per category

Validation rules:

- prefix values must be unique across active categories
- categories already referenced by `assets` or `stock_items` should be deactivated instead of hard-deleted

## Borrow / Return Integration

### What this slice does

This slice must stay compatible with Borrow / Return by preserving the serialized source of truth:

- asset assignment state remains `assets.status`
- current ownership remains active `asset_loans`

Dashboard counts for serialized assets therefore already reflect Borrow / Return outcomes without extra mirrored writes.

### What this slice does not do

Do not add mirrored `stock_items` mutations inside `approve_borrow_request_conn()` for serialized asset approvals.

Reason:

- current borrow requests operate on serialized asset codes
- `stock_items` is the source of truth for quantity-only inventory
- writing serialized borrow activity into `stock_items` would duplicate state

If quantity-tracked items later enter the Borrow / Return request flow, that should be a separate design slice.

## UI Placement

The dashboard stays inside `Settings`.

Recommended structure:

- `Asset Dashboard` full-width section
- dashboard actions at the top
- current import wizard/manual-add actions launched from inside the dashboard
- legacy standalone import block removed once the dashboard entry point is stable

## Codebase Boundary Decisions

### Frontend

`AssetImportWizard.tsx` and `useAssetImportState.ts` are already large. This slice should avoid adding more dashboard-specific logic directly into those files when a focused helper or component can hold the behavior.

Recommended boundary:

- keep import state in import-specific files
- put dashboard state in a new `useAssetDashboardState.ts`
- put dashboard rendering in a new `AssetDashboard.tsx`
- use a small copy/helper module if the dashboard text/formatting grows

### Backend

Keep responsibilities split:

- `asset_import.rs` for staged import and commit logic
- `asset.rs` for category helpers, dashboard summary/detail queries, and quantity updates
- `schema.rs` / `mod.rs` for schema and migrations

## Validation

Required validation for this slice:

- `AssetList.xlsx` imports both assigned laptop rows and available laptop rows correctly
- `Monitor.xlsx` imports assigned and available monitor rows correctly
- `Mouse-Key.xlsx` imports quantity rows correctly
- `usage_location` is normalized from workbook text into canonical values
- assigned serialized imports create active loans
- unresolved assigned rows stay out of successful import totals
- dashboard summary counts match the underlying tables
- serialized tab and quantity tab both load the expected records
- category prefix uniqueness validation blocks duplicates
- `npm run check:quality` passes

## Rollout Order

Recommended rollout order:

1. schema/category foundation
2. import pipeline extension
3. dashboard read APIs
4. dashboard UI in Settings
5. category-management UI

This order keeps the dashboard slice testable in layers and avoids building UI on top of unstable import semantics.
