# Asset Import Wizard Design

## Goal
Replace the temporary `Asset Seed Utility` textarea flow with a desktop-first asset import workflow that supports CSV and Excel files, stages imported rows in SQLite for review, and only writes reviewed valid rows into the main `assets` table.

## Scope
- Apply to `Staff_Kit` desktop app only.
- Support `csv`, `xlsx`, and `xls` file import.
- Stage imported rows in a persistent temp batch before committing to the main asset database.
- Support partial import: valid rows can be imported while invalid rows remain in the batch for later review.
- Keep a fast manual path for IT to add one asset directly when needed.

## Non-Goals
- Do not import asset `status` from the source file in the first version.
- Do not merge multiple Excel sheets in one import run.
- Do not add advanced bulk-edit tooling beyond direct inline cell editing.
- Do not automatically roll back rows that were already imported successfully.

## Design

### 1. Import Entry Points
- Replace the current seed textarea with two clearer actions:
  - `Import Assets`
  - `Add Asset Manually`
- `Import Assets` opens the batch import flow.
- `Add Asset Manually` stays available for one-off fixes and records that were skipped or rejected from file import.

### 2. Import Flow
- The flow is split into three stages:
  - `Choose File`
  - `Map Columns`
  - `Review Batch`
- `Choose File` accepts `csv`, `xlsx`, and `xls`.
- `Map Columns` is only shown when automatic header detection is incomplete or ambiguous.
- `Review Batch` is the primary working screen where IT edits rows, resolves errors, and imports valid rows into the main asset table.

### 3. Header Detection And Mapping
- The importer should recognize multiple header variants for each supported field.
- Required fields:
  - `assetCode`
  - `assetType`
  - `displayName`
- Optional fields:
  - `model`
  - `serialNumber`
  - `notes`
- Header matching should be driven by alias lists so the recognition rules can be extended later without redesigning the flow.
- If the detected mapping is not reliable enough, IT can manually map source columns to target fields before continuing.

### 4. Parsing And Batch Persistence
- Import parsing runs in two layers:
  - raw file read
  - normalization of headers and row values
- For Excel files, the first version should import from one selected sheet only.
- The importer stores a persistent batch in SQLite before touching the main `assets` table.
- Batch metadata should include:
  - source file name
  - source file type
  - imported sheet name when relevant
  - import timestamp
  - original headers
  - chosen field mapping
  - total parsed row count
- Each temp row should store both:
  - the raw source row
  - normalized editable fields used for validation and final import
- Inline edits update normalized values only, so the original raw row remains available for traceability.

### 5. Validation Rules
- A row is invalid when:
  - `assetCode`, `assetType`, or `displayName` is missing
  - `assetCode` is duplicated inside the same import batch
  - `assetCode` already exists in the main asset database
- Empty optional fields do not make a row invalid.
- Validation runs when rows are first parsed and again after each inline edit.

### 6. Review Lifecycle
- Each staged row has one of these statuses:
  - `valid`
  - `error`
  - `imported`
  - `skipped`
- IT can edit rows inline in the review grid.
- IT can skip rows that should not be imported yet while keeping them in the batch for traceability.
- When IT clicks `Import Valid Rows`, only rows currently marked `valid` are inserted into the main `assets` table.
- Rows with errors remain in the batch and can be fixed later.
- Rows already imported become read-only to avoid drifting history.
- A batch remains `pending_review` until there are no more actionable rows left to process.

### 7. Review Screen Behavior
- The `Review Batch` screen should prioritize speed and clarity over a heavy wizard feel.
- Recommended layout:
  - batch summary header
  - action bar
  - editable grid
  - focused error panel for the selected row
- The summary header should show:
  - file name
  - import time
  - total rows
  - counts for `valid`, `error`, `imported`, and `skipped`
- The action bar should include:
  - `Import Valid Rows`
  - `Show Errors Only`
  - `Show Pending Only`
  - `Delete Batch`
- The grid should support inline edits for:
  - `assetCode`
  - `assetType`
  - `displayName`
  - `model`
  - `serialNumber`
  - `notes`
- Error indication should happen at the cell level so IT can immediately see and fix the wrong value.

### 8. Module Boundaries
- Keep asset import separate from employee import logic.
- Recommended module split:
  - asset import parser
  - asset import batch store
  - asset import review service
  - asset import commit service
- The main `assets` table remains the source of truth for imported assets.
- Temp batches are operational review state only and must not be treated as official stock records.

### 9. Verification
- Import a fully valid file.
- Import a mixed file where some rows are valid and some rows fail validation.
- Reopen the app and confirm pending batches still exist.
- Fix an invalid row inline and import it successfully.
- Confirm duplicate `assetCode` values are blocked both within the batch and against the main database.
- Confirm already imported rows cannot be edited again.
- Confirm deleting a temp batch does not affect rows already imported into the main asset table.
