# Assets CRUD + Preload UI Design

## Goal
Turn `/assets` into the main admin workstation for asset master data with list/search/filter, full create/edit form, and batch preload by CSV/XLSX upload.

## Scope
- Build one operational `/assets` page for admins.
- Keep asset master-data editing separate from assignment workflows.
- Support single-asset create/edit and batch preload upload.
- Reuse existing asset APIs and service layer where possible.

## Design

### 1. Workstation Model
- `/assets` stays the main working page.
- The page includes:
  - filter bar
  - results table
  - `New Asset` action
  - `Preload Assets` action
- Create/edit uses a modal, not a separate route.
- Preload uses a dedicated upload modal, not a separate route.

### 2. Assets Table
- Initial columns:
  - `Asset Code`
  - `Name`
  - `Asset Type`
  - `Status`
  - `Owning Unit`
  - `Managing Unit`
  - `Serial Number`
  - `Current Holder`
  - `Recorded At`
  - `Actions`
- Search applies to `assetCode` and `name`.
- Filters apply to `status` and `assetType`.
- Sorting is out of scope for this phase.
- `Current Holder` is read-only and comes from the active assignment if present.
- Row action is `Edit`.

### 3. Create/Edit Modal
- The form includes the full asset master-data set:
  - `assetCode`
  - `name`
  - `assetType`
  - `status`
  - `recordedAt`
  - `owningUnit`
  - `managingUnit`
  - `serialNumber`
  - `brand`
  - `modelName`
  - `notes`
  - `retiredAt`
  - `disposedAt`
- `assetCode` behavior:
  - create: editable, optional for backend-generated code
  - edit: read-only
- `retiredAt` is enabled only when `status = RETIRED`.
- `disposedAt` is enabled only when `status = DISPOSED`.
- When the status is not compatible, those fields should be cleared or disabled.
- Successful create/edit closes the modal, refreshes the list, and shows a short success message.

### 4. Preload Modal
- Phase 1 supports:
  - `CSV`
  - `XLSX`
- Upload flow:
  - user uploads file
  - client parses and validates rows
  - modal shows summary
  - user submits batch preload
  - backend performs `upsert`
- Result summary shows:
  - total rows
  - valid rows
  - invalid rows
  - created count
  - updated count

### 5. Preload File Contract
- Accepted columns:
  - `assetCode`
  - `name`
  - `assetType`
  - `status`
  - `recordedAt`
  - `owningUnit`
  - `managingUnit`
  - `serialNumber`
  - `brand`
  - `modelName`
  - `notes`
  - `retiredAt`
  - `disposedAt`
- Header names are fixed in this phase.
- Column-mapping UI is out of scope.
- Optional columns may be omitted, but any provided header must match the contract exactly.

### 6. Validation Rules

#### Single asset create/edit
- `name` is required.
- `assetType` is required.
- `status` must be a valid enum value.
- `retiredAt` is valid only for `RETIRED`.
- `disposedAt` is valid only for `DISPOSED`.

#### Batch preload
- `assetCode` is required for every row.
- `name` is required.
- `assetType` is required.
- `status` must be valid.
- date columns must parse into valid dates when provided.
- duplicate `assetCode` values within the same upload file are rejected.
- invalid rows block submit until the file is corrected and re-uploaded.

### 7. Business Boundaries
- `/assets` only edits asset master data.
- It does not create, edit, or close assignments.
- `Current Holder` is informational only.
- Batch preload uses `assetCode` as the business identity and performs `upsert`.
- Preload must not mutate assignment data.

### 8. Error Handling
- Three error groups must be visible to the user:
  - file parse errors
  - row validation errors
  - server preload/create/update errors
- Row validation must show row number, field, and reason.
- Server-side validation remains authoritative even if client validation passes.

### 9. Architecture
- Break the UI into focused units:
  - `AssetsPageShell`
  - `AssetsTable`
  - `AssetFormModal`
  - `AssetPreloadModal`
- Keep API/service reuse:
  - `GET /api/assets`
  - `POST /api/assets`
  - `PATCH /api/assets/[assetCode]`
  - `POST /api/assets/preload`
- Successful mutations revalidate `/assets`.

### 10. Testing Strategy
- Service/API tests:
  - create asset with full form data
  - update asset with full form data
  - reject invalid status/date combinations
  - preload valid CSV/XLSX
  - reject duplicated asset codes in one file
  - return correct preload summary
- UI tests:
  - search/filter list
  - open create modal
  - create asset and refresh list
  - edit asset and refresh list
  - upload preload file, preview, and submit successfully
- Browser smoke:
  - login
  - open `/assets`
  - create asset
  - edit asset
  - preload sample file
  - verify summary and updated list

## Out of Scope
- Asset detail page
- Assignment timeline/history on the asset screen
- Upload column-mapping wizard
- Bulk edit outside preload
