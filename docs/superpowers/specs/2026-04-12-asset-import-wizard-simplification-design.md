# Asset Import Wizard Simplification Design

## Goal

Simplify `Asset Import Wizard` so it behaves much closer to the existing `EE list` import flow: the user chooses one of two import modes, picks a CSV/Excel file, and the app auto-detects the headers and rows without requiring manual column mapping.

This slice should make asset import feel like a file-driven operational workflow, not a technical staging tool that asks IT to map columns by hand on every import.

## Current Baseline

`main` already has:

- a staged asset import pipeline in SQLite
- two import modes in backend and frontend: `serialized` and `quantity`
- header alias detection for CSV and Excel
- automatic sheet/header-row detection for workbooks
- row review before official import into `assets` or `stock_items`

The current friction is mostly in the frontend flow:

- the wizard exposes a dedicated `Map Columns` step
- users must confirm or fix mappings even when the file already matches the known workbook structure
- serialized import still treats `displayName` as a required mapped column, which is stricter than the actual workflow IT wants

The `EE list` import already behaves more simply:

- pick file(s)
- auto-detect columns
- preview/import

That simpler behavior is the reference direction for assets.

## Chosen Approach

### Recommended approach

Implement this as a `backend-required-field simplification + frontend flow simplification` slice.

That means:

- keep only two import modes:
  - `Serialized Asset`
  - `Quantity Asset`
- keep the existing staged-review model
- remove the manual `Map Columns` step from the normal flow
- rely on backend alias detection and strict file-shape validation
- allow import only when the file matches the supported workbook conventions

### Rejected alternatives

#### 1. Keep manual mapping as the main step

Rejected because backend already auto-detects most known headers, so the mapping step adds repeated friction without adding much value for the standard files IT actually uses.

#### 2. Keep manual mapping as a fallback UI

Rejected for this slice because Zon explicitly wants the app to import standardized files directly and not require manual matching. Supporting a hidden fallback path now would preserve complexity that the product is trying to remove.

#### 3. Rewrite asset import to skip staging entirely

Rejected because the staged-batch review flow already exists and still provides value for checking rows before official import. The problem is the mapping UX, not the staged commit architecture.

## Import Modes

The wizard must expose exactly two user-facing import types:

1. `Serialized Asset`
2. `Quantity Asset`

There should not be additional import sub-modes in the UI for this slice.

## New User Flow

### Shared import flow

For both import types, the intended flow is:

1. User opens `Asset Import Wizard`
2. User selects `Serialized Asset` or `Quantity Asset`
3. User picks a `csv`, `xlsx`, or `xls` file
4. App auto-detects:
   - file type
   - workbook sheet
   - header row
   - supported columns
5. If the file is valid, the app moves directly into staged review
6. User reviews rows and imports the valid rows

### No manual column mapping

The normal asset import flow should no longer ask the user to map source headers to app fields.

Instead:

- standardized headers are auto-detected
- typo/header variants are normalized through aliases
- unsupported or missing required file shapes are rejected with a clear error

## File-Shape Requirements

### General rule

The app should import directly when the file is in the supported standardized format.

If the file does not contain the minimum required shape for the selected mode, the app should:

- show a clear validation error
- explain which required columns are missing
- stop before staging/importing

There should be no manual mapping rescue path in this slice.

### Serialized Asset minimum requirements

Serialized import should require:

- `Assetcode / ID`
- `Category`

`Asset Name` should no longer be a hard requirement if the row can still produce a valid persisted `display_name`.

### Quantity Asset minimum requirements

Quantity import should require:

- `Item Name`
- `Category`
- `Quantity`

## Serialized Field Semantics

Serialized rows should continue committing into the official `assets` table and keep compatibility with the dashboard and Borrow / Return flows.

### Source-of-truth mapping

For serialized imports:

- `Assetcode` -> `asset_code`
- `Category` -> resolved category
- `Computer Name` -> `computer_name`
- `Asset Name` -> `display_name`
- `Model` -> `model`
- `Serial Number` -> `serial_number`
- `Adapter Number` -> `adapter_number`
- `Usage Location` -> `usage_location`
- `Note` -> `notes`

### Computer Name precedence

For serialized rows:

- prefer `Computer Name` from the file
- if blank, fall back to `ASW + Assetcode`

### Asset Name fallback

For serialized rows:

- prefer imported `Asset Name`
- if blank, fall back to `Computer Name`
- if that is also blank, fall back to `Assetcode`

This allows standardized files that omit `Asset Name` to still import cleanly without forcing manual mapping.

## Quantity Field Semantics

Quantity imports should continue staging and committing rows into `stock_items`.

The target outcome after import is operationally simple:

- the correct rows appear in the quantity table
- the correct quantity totals appear in the dashboard

The user should not need to map columns manually to achieve that result when the file follows the supported format.

## Header Normalization

The app UI should display normalized labels such as:

- `ID`
- `Serial Number`
- `Usage Location`

Import should still accept workbook variants and typos, including examples such as:

- `Assetcode`
- `Asset code`
- `Serrial Number`
- `Usuage Location`
- `Adapter number`

This keeps the product UI clean while staying tolerant of the operational spreadsheets IT already uses.

## Sheet Handling

Workbook sheet auto-detection should remain.

If a workbook contains multiple sheets:

- the app should auto-pick the best matching sheet by score
- the user may still switch the sheet before staging if needed

This is a sheet-selection fallback, not a column-mapping step.

## Review Behavior

The staged review step remains part of the flow.

This slice does not remove:

- row validation
- row skipping
- row-level edits
- owner-resolution review for serialized assigned assets

The simplification target is the import entry flow, not the review/commit safety rail.

## Relationship To Column Profiles

This slice does not yet add:

- editable column titles
- column visibility configuration inside the wizard

However, it should stay compatible with the existing direction Zon wants:

- normalized app labels
- future editable column titles
- future per-user visible-column choices based on the logged-in account profile

## Validation

This slice is complete when:

- `Asset Import Wizard` exposes only `Serialized Asset` and `Quantity Asset`
- users can import a standardized CSV/Excel asset file without manual column mapping
- the app rejects unsupported files with a clear missing-column error
- serialized import accepts `Assetcode`, `Serrial Number`, `Usuage Location`, and similar aliases
- serialized import no longer requires `Asset Name` when fallback values can produce a valid `display_name`
- quantity import still stages and imports the correct rows and quantities
- the review step still works after auto-detection
- `npm run check:quality` passes

## Non-goals

Out of scope for this slice:

- user-facing manual column mapping
- a generic import engine shared with employee import
- editable asset column titles in the UI
- per-user visible-column controls for the asset dashboard
- changes to Borrow / Return business logic
