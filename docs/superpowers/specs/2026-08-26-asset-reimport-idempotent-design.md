# Idempotent Asset Re-import Design

## Goal

Make serialized Asset master imports additive and idempotent by default: exact
Assets already in SQLite are previewed as `Existing`/`Skip`, genuinely new
Assets remain importable, and canonical code/serial conflicts remain blocking
errors.

## Scope and constraints

- Change only the Asset import preview/classification, approval, and regression
  coverage.
- Preserve owner-aware import behavior, canonical Asset identity, existing
  database uniqueness constraints, and canonical loan/status behavior.
- Do not update metadata for an existing Asset and do not add update-existing
  mode.
- Do not change Employee import or the database schema unless implementation
  evidence proves it unavoidable.
- Automated tests use isolated in-memory/test connections or temporary copies;
  the real `00_ExSource/AssetList.xlsx` is read-only acceptance input and the
  production database is never opened for writes.

## Classification model

The backend will resolve each serialized import row against the current
`assets` table using normalized canonical `asset_code` and optional normalized
`serial_number`.

1. `Existing`/`Skip`: the incoming code matches the existing code and the
   serial is absent on either side or equal. This row is not an error and is
   excluded from approval writes.
2. `New`: no existing code or serial identity conflicts with the incoming row.
   This row is eligible for the existing asset-create path.
3. `Error`/`Conflict`: the incoming code resolves to one asset while its
   serial resolves to a different asset, or any other canonical code/serial
   disagreement is found. The row remains excluded from writes.

The same classification will also detect duplicate identities within the
incoming batch, so two new rows cannot both pass and later rely on a database
constraint to resolve the ambiguity. Classification errors carry an explicit
conflict message in `validation_errors` and the preview error collection.

## Data flow and commit behavior

Batch creation and direct preview will use a shared read-only classifier over
the parsed normalized rows. Batch rows will store the existing row status
using the current schema-compatible status values: `skipped` for Existing rows
and `valid` for New rows; the row/API presentation will expose the reason as
Existing/Skip versus Ready/New without changing the meaning of blocking
`error` rows. Summary counts will report Existing/Skipped separately from
importable New/Ready rows.

Before approval inserts, the existing transaction will revalidate the batch
against the live `assets` table. New rows that have become exact existing rows
will be converted to skipped inside that transaction; newly discovered
conflicts will become errors and prevent their insert. Only rows still
classified as New are passed to `create_asset_tx`. Existing rows never call an
asset write function, so active loans, holder/status, and metadata remain
untouched. Approving a batch containing only Existing rows commits no Asset
inserts and reports zero imported rows.

The direct import command will use the same classifier and insert only New
rows, retaining its existing skipped/error report semantics. This keeps both
the staged wizard and direct path idempotent.

## UI behavior

The Asset preview will show status labels that distinguish `Existing`/`Skip`
from `New`/ready rows and will include Existing rows in the total without
showing them as red errors. Approval remains disabled only when there are no
New/Ready rows or when blocking errors remain. The current 10-row workbook is
expected to preview as Total 10, New 4, Existing/Skipped 6, Errors 0, and
approval inserts only the four Samsung rows.

## Testing strategy

Rust Asset import tests will cover exact existing code with same/absent serial,
repeated import, mixed six-existing/four-new batch, code/serial cross-conflict,
active loan/status preservation, zero-write Existing-only approval, and a
second import after success. A fixture representing the current workbook rows
will assert the 6/4/0 classification and final Asset count change of 37 to 41
in an isolated test database. Existing owner-resolution and canonical-loan
tests will remain unchanged and continue to run.

## Files expected to change

- `src-tauri/src/db/asset_import.rs`: shared identity classification, batch
  revalidation/commit filtering, direct import behavior, and Rust regression
  tests.
- `src/features/assets/AssetImportWizard.tsx` and/or adjacent Asset import
  copy/status helpers: visible New/Existing/Skip labels and summary counts,
  only if the backend status contract requires presentation changes.
- `src/types/staff.ts`: typed preview/status fields only if required by the
  actual API shape.

No Employee, schema, loan, or production-data files are part of the design.
