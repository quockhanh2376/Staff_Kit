# Owner-Aware Asset Import Design

## Goal

Extend the desktop asset import wizard so it can ingest the `AssetList.xlsx` style workbook where:

- `Available` rows represent serialized assets that are in stock and not assigned to any employee yet.
- `Laptop` rows represent serialized assets that are already assigned to employees.

The same slice must also make the employee table show laptop-derived `Computer Name` values from active laptop loans, including multiple laptop values per employee.

## Scope

This slice adds an ownership-aware import path for serialized assets. It does not replace the existing quantity import model.

In scope:

- detect and stage workbook rows from `Available` and `Laptop` style sheets
- keep raw source columns for future mapping expansion
- resolve `Laptop` owner rows against the imported employee master
- let IT review and edit mismatched owner data before approval
- import `Available` rows as in-stock serialized assets
- import `Laptop` rows as assigned serialized assets by creating both asset records and active loan records
- render employee-table `Computer Name` values from active laptop loans

Out of scope:

- assign report UI
- team-level or employee-level assignment dashboard
- non-laptop ownership reporting beyond the employee table column
- changing employee import behavior
- forcing this wizard to work only with the current sample workbook

## Business Rules

### Workbook semantics

- `Available` sheet rows are warehouse assets only.
- `Laptop` sheet rows are already issued assets.
- The import wizard must continue to accept CSV/Excel sources and must not become hardcoded to one workbook name.

### Employee ownership semantics

- Owner resolution must search the full employee master across all staff groups, including `employee_list`, `onboarding`, `internal_movement`, and `offboarding`.
- `Available` rows do not require owner resolution.
- `Laptop` rows require owner resolution before import.
- If an employee cannot be resolved, the row stays out of the successful import set and IT can later fix data manually outside the import.
- If `StaffID` resolves but file name/team snapshots do not match the employee master, the row remains reviewable and editable.

### Employee ID normalization

- The file may provide `StaffID` values like `1302`, `ASW1302`, or `ASWVN1302`.
- Import must normalize the input into a lookup token based on the numeric suffix.
- The canonical employee ID written into the import result must come from the existing employee master, not from a hardcoded `ASWVNXXXX` formatter.
- Zero-padding must not be forced during normalization because the real employee master contains both three-digit and four-digit suffixes.

### Computer name semantics

- `Computer Name` is derived from asset ownership for laptop-style devices in this slice.
- The derived value is:
  - `computerName = "ASW" + assetCode`
- Example mappings:
  - `VNLAP122` -> `ASWVNLAP122`
  - `VNMACPRO003` -> `ASWVNMACPRO003`
  - `VNIMACPRO016` -> `ASWVNIMACPRO016`
  - `VNMACAIR001` -> `ASWVNMACAIR001`
- If an employee has multiple active laptop loans, the employee table must render them in one cell using comma-separated values with line breaks, for example:

```text
ASWVNMACPRO010,
ASWVNLAP293
```

## Current-System Constraints

- The existing asset import model currently stages asset-only rows and explicitly avoids assigning assets to employees.
- Official ownership currently lives in `asset_loans`, not in the employee row itself.
- The existing employee record stores `computername` as a single text field, which is not sufficient for a multi-laptop ownership model.

Because of those constraints:

- ownership-aware import must create active `asset_loans` records for `Laptop` rows
- employee-table `Computer Name` display must be derived from active loans instead of overwriting `employees.computername`

## Parsing and Staging Design

### Sheet handling

- The wizard keeps the current file-inspection flow.
- For Excel files, sheet choice remains explicit when needed.
- `Available` and `Laptop` are treated as source semantics, not as required sheet names.
- If a future file uses equivalent columns under different sheet names, IT can still map it manually.

### Raw column preservation

- Every staged row continues to retain the full raw source payload.
- The import schema grows to store owner-related staged fields separately from the raw payload.
- Unknown columns remain preserved in raw JSON so future mapping fields can be added without losing source data.

### New staged owner fields for serialized-owner rows

Each staged `Laptop` row needs both source snapshots and resolved owner fields:

- source snapshot:
  - `submittedStaffId`
  - `submittedFullName`
  - `submittedTeam`
  - `submittedPhoneNumber`
- resolved owner:
  - `resolvedEmployeeId`
  - `resolvedEmployeeRowId`
  - `resolvedFullName`
  - `resolvedTeamName`
- review metadata:
  - `ownerMatchStatus`
  - `ownerWarnings`

The row continues to store current asset import fields such as:

- `assetCode`
- `assetType`
- `displayName`
- `brand`
- `model`
- `serialNumber`
- `warehouse`
- `notes`

## Owner Resolution Rules

### Resolution order

For each `Laptop` row:

1. Normalize the submitted `StaffID` into a numeric suffix lookup token.
2. Find employee-master rows whose `employee_id` has the same numeric suffix.
3. If exactly one employee matches, use that employee as the canonical owner.
4. Compare file snapshot name/team against the canonical employee record to produce warnings.
5. If no employee matches, leave the row unresolved and block it from successful import.

### Warning vs blocking rules

- Missing employee resolution:
  - blocking
  - row cannot be imported as assigned
- Resolved employee with mismatched name:
  - warning only
  - row stays editable/reviewable
- Resolved employee with mismatched team:
  - warning only
  - row stays editable/reviewable
- Resolved employee with both mismatches:
  - warning only
  - row stays editable/reviewable

### Edit behavior

IT must be able to edit owner review data after staging:

- corrected employee ID
- corrected employee display fields when necessary for review
- corrected asset fields already supported by the wizard

Editing a row triggers owner revalidation so the row status can move between warning/error/valid states.

## Import Commit Semantics

### `Available` rows

Approving valid `Available` rows:

- creates serialized asset records
- allows blank `serialNumber`
- lands assets in `in_stock`
- does not create loan records

### `Laptop` rows

Approving valid `Laptop` rows:

- creates serialized asset records if the asset does not already exist
- requires a resolved employee owner
- creates an active `asset_loan` linked to that employee
- sets the asset status to `assigned`

### Duplicate and integrity checks

Before committing:

- reject duplicate active `assetCode` values
- reject duplicate active `serialNumber` values when serial is present
- reject duplicate active loans for the same asset
- reject rows whose resolved employee no longer exists
- reject rows whose asset category is invalid for serialized tracking

## Employee Table Design

### Display model

- Keep the existing `Computer Name` column label.
- Do not overwrite `employees.computername` in this slice.
- Load a derived `computerName` display value from active laptop loans.
- The derived values are ordered, concatenated, comma-separated, and rendered with line breaks.

### Multi-laptop rendering

- One employee may have zero, one, or multiple active laptop loans.
- When multiple values exist, the desktop table cell and mobile card must display all values.
- Search behavior for employee-table queries should continue to find these derived computer names.

### Future-safe reporting

- This slice only adds derived employee-table visibility.
- Future `Assign Report` screens for employee/team reporting can reuse the same ownership source of truth from `asset_loans`.

## UI Design

### Wizard review UX

The review screen for owner-aware rows must show:

- the submitted source snapshot
- the resolved employee target
- blocking errors for unresolved rows
- non-blocking warnings for name/team mismatches
- editable owner field controls for IT correction

Successful import feedback must report:

- total imported `Available` rows
- total imported `Laptop` rows
- total skipped or unresolved `Laptop` rows left for manual follow-up

### Import-result expectation

After import, IT can compare the successful import total with the source file and manually correct any unresolved employee rows outside the import flow if needed.

## Data Model Summary

The implementation will likely require:

- staged owner-resolution fields on `asset_import_rows`
- read APIs that return owner review state to the React wizard
- employee-query enrichment with derived laptop/computer-name values

The implementation must avoid duplicating the ownership source of truth into long-lived employee dynamic fields.

## Validation and Testing

Required verification for this slice:

- workbook inspection still works for current generic asset import files
- `Available` rows import as in-stock serialized assets without owner data
- `Laptop` rows resolve owner records from the employee master
- mismatched name/team rows stay reviewable with warnings
- unresolved rows do not import successfully
- approved `Laptop` rows create active `asset_loans`
- employee table shows derived comma-separated multi-laptop `Computer Name` values
- employee search can still find laptop-derived computer names

## Explicit Defers

- assign report per employee
- assign report per team
- bulk reassignment workflows
- receive/reconcile workflows for mismatched existing assets
- forced rewriting of the underlying `employees.computername` storage column
