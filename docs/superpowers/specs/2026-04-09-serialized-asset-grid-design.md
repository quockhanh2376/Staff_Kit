# Serialized Asset Grid Design

## Goal

Align the serialized asset table in the app with the workbook structure IT already uses in Excel, while keeping one shared serialized table in the product and preserving Borrow / Return state, holder, and account-scoped column preferences.

This slice should make the in-app serialized table feel like the operational spreadsheet after import, not like a separate technical admin table.

## Current Baseline

`main` already has:

- a serialized asset dashboard inside `Settings`
- owner-aware import for laptop and monitor style rows
- persisted asset metadata such as `display_name`, `display_name_short`, `serial_number`, `usage_location`, and `notes`
- holder and status derived from active `asset_loans`
- account-scoped column preference infrastructure already used by the employee table

The remaining gap is that the serialized dashboard table still uses a narrower technical schema:

- `Asset Code`
- `Category`
- `Display`
- `Model`
- `Serial`
- `Usage`
- `Status`
- `Holder`

That does not yet match the workbook mental model that IT wants to see after import.

## Chosen Approach

### Recommended approach

Implement this as an end-to-end `schema + import + dashboard + profile layout` slice.

That means:

- persist missing serialized metadata as first-class asset fields
- extend import commit logic so workbook columns land in official asset records
- replace the current serialized dashboard columns with one fixed, wider table
- reuse the existing account-scoped column preference model for per-user order/width settings

### Rejected alternatives

#### 1. UI-only relabeling

Rejected because the dashboard cannot truthfully show `Computer Name` and `Adapter Number` unless those values are persisted and loaded from the official serialized asset source of truth.

#### 2. A generic table engine for the whole app

Rejected for this slice because it is broader than the immediate need. The product only needs a better serialized asset grid right now, and the employee-table preference system already provides a reuse path.

#### 3. A category-specific dashboard table per asset family

Rejected because IT wants one common serialized table, with inapplicable columns left blank, not separate tables for laptops and monitors.

## Serialized Table Contract

### One shared table

The app should keep exactly one serialized-asset table for all serialized asset categories.

Rows from different categories share one fixed default column set. If a value does not apply to a row, the cell stays empty.

### Default visible columns

The default column set and default order should be:

1. `ID`
2. `Category`
3. `Computer Name`
4. `Asset Name`
5. `Model`
6. `Serial Number`
7. `Adapter Number`
8. `Usage Location`
9. `Note`
10. `Status`
11. `Holder`

### Meaning of `ID`

`ID` is the UI label for the persisted asset code.

Rules:

- `ID = Assetcode`
- it is the primary visible identifier in the table
- it is the first default column
- it remains sortable and movable like the other columns

### `Status` and `Holder`

`Status` and `Holder` remain default columns.

They should sit later in the default order so the table matches the workbook more closely, but users must still be able to drag them to any position.

## Source of Truth Per Column

### Persisted serialized asset fields

The serialized dashboard must read from persisted asset fields for:

- `ID` -> `asset_code`
- `Category` -> resolved category label
- `Computer Name` -> new `computer_name`
- `Asset Name` -> `display_name`
- `Model` -> `model`
- `Serial Number` -> `serial_number`
- `Adapter Number` -> new `adapter_number`
- `Usage Location` -> `usage_location`
- `Note` -> `notes`

### Derived holder/status fields

The serialized dashboard must continue deriving:

- `Status` from the serialized asset status / active-loan state
- `Holder` from the current active loan and employee data

Borrow / Return therefore remains compatible with the new table without introducing a second holder source of truth.

## Import Semantics

### Header normalization

The app UI should use clean labels:

- `ID`
- `Serial Number`
- `Usage Location`

But import must continue accepting workbook headers and typo variants such as:

- `Assetcode`
- `Asset code`
- `Serrial Number`
- `Usuage Location`
- `Adapter number`

### Computer Name precedence

For serialized rows:

- prefer the imported `Computer Name` value when the file provides it
- if the workbook value is blank, fall back to `ASW + Assetcode`

This fallback must happen before the row is committed into the official serialized asset record so the dashboard reads one stable stored value afterward.

### Column mapping on import

Commit imported serialized fields as:

- `Assetcode` -> `asset_code`
- `Category` -> category resolution
- `Computer Name` -> `computer_name`
- `Asset Name` -> `display_name`
- `Model` -> `model`
- `Serrial Number` -> `serial_number`
- `Adapter number` -> `adapter_number`
- `Usuage Location` -> `usage_location`
- `Note` -> `notes`

Owner-aware assignment semantics for `Status` and `Holder` do not change in this slice.

## Column Behavior

### Sort

Each column header should provide sort behavior.

First slice expectation:

- single-column sort
- click to cycle through at least ascending / descending / default

### Reorder

Users must be able to drag and drop columns into a different order.

The default order above is only the starting point.

### Resize

Users must be able to resize column widths.

Widths should persist with the same account-scoped layout profile as order preferences.

### Persistence scope

Column preferences must be saved per logged-in account profile, not globally.

Reuse the existing account-scoped storage pattern already used by employee-table column preferences:

- account-scoped key
- separate persisted order / widths / label overrides where relevant

This keeps the slice aligned with the planned role-based local account model.

## Column Labels And Future Editing

For this slice:

- ship with normalized default labels
- structure the label definitions so they can be overridden later

This slice does **not** need to ship a visible column-title editor yet, but it must avoid hardcoding the labels in a way that blocks that future feature.

## UI Scope

The work happens inside the existing `Asset Dashboard` serialized table.

This slice does not:

- change quantity-table semantics
- redesign the whole dashboard shell
- add a new top-level route
- change Borrow / Return review UI

## Data Model Changes

Add persisted serialized asset fields for:

- `computer_name TEXT NULL`
- `adapter_number TEXT NULL`

Migration requirements:

- existing databases must gain the new columns without data loss
- existing serialized rows should keep null values until later imports or edits provide data

## Validation

This slice is complete when:

- the serialized dashboard shows the approved default columns
- `Computer Name` persists from import or falls back to `ASW + Assetcode`
- `Adapter Number` persists from import
- `Note` becomes visible in the serialized dashboard
- `Status` and `Holder` remain present and correct
- column order can be dragged and saved per logged-in account
- column widths can be resized and saved per logged-in account
- each column can be sorted
- typo workbook headers still import successfully
- `npm run check:quality` passes

## Non-goals

Out of scope for this slice:

- a whole-app generic grid framework
- multi-column sort
- a visible UI for renaming serialized table columns
- changing quantity-dashboard columns
- reworking Borrow / Return business rules
