# Shared Asset Import Shell Design

## Goal

Replace the current `Asset Import Wizard` flow with a direct file-import experience that matches the operational feel of `EE list` import:

1. choose file
2. inspect automatically
3. preview
4. approve
5. import valid rows directly into the database

The new asset flow must support exactly two modes:

- `Serialized Asset`
- `Quantity Asset`

It must not expose manual column mapping, staged-batch management, or row-by-row editing inside the preview screen.

## Problem Statement

The current asset import UI is still shaped like a technical staging tool:

- `Choose File`
- `Map Columns`
- `Review Batch`

That flow is heavier than the files IT actually uses, and it is also the line currently associated with the black-screen transitions Zon reported. Even after simplification attempts, the architecture is still anchored to the old staged wizard model.

The `EE list` import already proves a simpler pattern works better for operations:

- file selection
- automatic inspection
- preview
- approve

That pattern should become the baseline for asset import as well.

## Chosen Approach

### Recommended approach

Build a shared import shell that is reused by both:

- `EE list` import
- `Asset` import

The shared shell handles the user-facing workflow:

- file selection
- inspection summary
- preview
- approval
- import report

Domain-specific adapters stay separate:

- employee import adapter
- asset import adapter

This keeps the user experience aligned while avoiding a risky attempt to unify all business logic into one backend path.

### Rejected alternatives

#### 1. Patch the existing asset wizard again

Rejected because the old wizard still centers on `Map Columns` and `Review Batch`. That architecture is the wrong shape for the product direction and is too easy to break during UI state transitions.

#### 2. Build a new asset-only import flow and copy the EE list UX

Rejected because that would leave two very similar import systems in the codebase. Zon explicitly wants duplication reduced where possible.

#### 3. Fully merge employee and asset import business logic

Rejected because the domains differ too much:

- employee import merges profile data into `employees`
- asset import writes into `assets` or `stock_items`

The right reuse boundary is the UI shell and adapter contract, not one universal parser/mutator.

## Shared Import Shell

The shared shell is the reusable frontend contract for file-driven imports.

### Shared shell responsibilities

- open file dialog
- show selected file metadata
- show inspection status
- show preview summary counts
- render preview rows
- render preview errors
- approve import
- show import result/report

### Shared shell must not do

- domain validation rules
- asset/employee-specific field derivation
- row editing logic
- direct database mutation logic

Those remain in the import adapters and backend commands.

## Asset Import Flow

### User flow

For both `Serialized Asset` and `Quantity Asset`, the new flow is:

1. user opens asset import
2. user chooses mode
3. user picks one `csv`, `xlsx`, or `xls` file
4. app inspects the file automatically
5. app shows preview and validation results
6. user clicks `Approve Import`
7. app imports only valid rows into the official database tables
8. app shows the final report

### No mapping UI

The new flow removes:

- `Map Columns`
- `Stage Batch`
- `Review Batch`
- batch list management inside the import entry screen

The app either recognizes the standardized file shape or rejects it with a clear error.

## Approval Semantics

### Preview behavior

Preview is read-only.

The user can:

- inspect summary counts
- inspect example rows
- inspect the first set of validation errors
- approve the import
- cancel and return

The user cannot:

- edit rows in preview
- skip individual rows
- remap columns

### Approve behavior

If a file contains both valid rows and invalid rows:

- approval is still allowed
- only valid rows are imported
- invalid rows are skipped
- the final report must show the skipped/error counts and reasons

This matches Zon’s decision that operations should not be blocked by a few bad rows when the rest of the file is clean.

## Asset Modes

### Serialized Asset

Target table: `assets`

Accepted file headers remain tolerant of operational variants and typos, including:

- `Assetcode`
- `Computer Name`
- `Asset Name`
- `Serrial Number`
- `Adapter number`
- `Usuage Location`

The app UI should still normalize labels to:

- `ID`
- `Computer Name`
- `Asset Name`
- `Serial Number`
- `Adapter Number`
- `Usage Location`

### Serialized field semantics

- `ID` maps to `asset_code`
- `Computer Name` prefers the file value
- if `Computer Name` is blank, use `ASW + Assetcode`
- `Asset Name` falls back in this order:
  1. imported `Asset Name`
  2. `Computer Name`
  3. `Assetcode`

### Quantity Asset

Target table: `stock_items`

The user expectation is operationally simple:

- correct rows appear in the quantity table
- correct counts appear in the dashboard

No column mapping or row editing should be required when the input file follows the supported shape.

## Adapter Model

The frontend should use separate adapters behind the shared shell.

### Employee adapter

Keeps using the employee import backend logic, but adapts its preview/report data into the shared shell contract.

### Asset adapter

Implements asset-specific preview and commit commands for:

- `Serialized Asset`
- `Quantity Asset`

This adapter owns:

- mode-specific required columns
- field alias normalization
- preview row shaping
- import report shaping

## Backend Direction

The new asset flow should move away from the staged-batch UX contract and toward direct preview/import commands.

### Asset preview command

The preview command should:

- inspect the selected file
- auto-detect the best sheet/header row
- validate rows for the selected mode
- return:
  - summary counts
  - preview rows
  - validation errors
  - selected sheet metadata

### Asset import command

The import command should:

- re-read the file using the approved mode and selected sheet
- import only valid rows
- skip invalid rows
- return:
  - imported count
  - skipped count
  - failed/error details

The import path should write directly into the official tables instead of depending on the current `asset_import_batches` review loop.

## Legacy Staged Import Compatibility

The old staged import tables and commands may remain in the backend temporarily during the transition, but the new asset UI should no longer depend on them.

This reduces migration risk:

- UI can move first
- legacy cleanup can happen later

## Error Handling

The shared shell and asset adapter must surface three classes of issues clearly:

1. file open / parse errors
2. missing required columns
3. row-level validation errors

Missing-column failures must stop the flow before preview approval.

Row-level validation errors must not block approval if at least one valid row exists.

## Validation

This slice is complete when:

- asset import uses a `preview + approve` flow instead of the old wizard
- there is no user-facing manual column mapping for asset import
- there is no row editing in the asset preview screen
- `Serialized Asset` and `Quantity Asset` are the only two visible modes
- valid rows import directly into `assets` or `stock_items`
- invalid rows are skipped and reported
- the shared shell is used by both employee import and asset import
- the black-screen path tied to the old review transition is removed with the old flow

## Non-goals

Out of scope for this slice:

- editable asset column titles
- per-user visible-column configuration
- drag/drop column management in the import preview
- redesigning the asset dashboard table itself
- deleting the legacy staged import tables immediately
