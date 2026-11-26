# Asset import staging: design (2026-11-26)

## Intent / why
- Today the serialized Asset Import (`AssetImportWizard`, "Approve Serialized Assets") previews a file in memory, and Approve stays disabled while any row has an error. With `00_ExSource/Asset_November.xlsx` (156 rows), 7 serial mismatches block all 136 new laptops.
- The owner wants the file loaded into a **staging table in the app**, fixed there until it is right, and only then written to the database.
- **Why:** to stop bad writes. On 2026-11-25, a direct import wrote 377 junk assets.

## Goals
- Load a file into a persisted staging batch that survives an app restart.
- Edit rows before Approve and skip rows one by one. Problem rows are flagged with their reason.
- **The file's serial is the source of truth.** An existing laptop (same asset code) whose serial differs gets its serial updated on Approve. This is not an error.
- Serial comparison ignores dashes, spaces and case (`PF-5MBXRW` = `PF5MBXRW`).
- Approve writes in one transaction after a mandatory safety snapshot. The batch is then deleted, and only the audit row remains.
- Discard deletes the batch and its rows.

## Non-goals
- No change to quantity (stock) imports, the Historical Assignment (monitor) importer, or holders / loans of existing assets.
- Other fields of an existing asset (model, location, name) are not overwritten. Only serial and condition are updated, and condition is updated as today.
- No multi-user locking.

## Anti-goals
- Staged rows appear in any real screen before Approve.
- Approve writes rows the owner has not seen, such as rows that became invalid after loading. The commit revalidates inside the transaction, and any row that turns into an error aborts the whole Approve.

## Approaches
- **A (picked): a UI on the existing batch backend.** `asset_import_batches` / `asset_import_rows` already have create, edit field, skip, commit (one transaction, snapshot, revalidation) and delete. Only the screen and the serial rules are missing.
- B (rejected): make the in-memory preview editable. Edits are lost on close.
- C (rejected): new tables copying EE List sync. They would duplicate A's tables.

## Decisions made with the owner
- Serial mismatch on an existing code: **the file wins** (2026-11-26).
- Use approach A.

## Claude's calls
- **Approve rule:** Approve is enabled only when no row has status `error`. Skipped rows are ignored.
- **Serial rules:**
  - A row whose code and serial point to two *different* existing assets stays an error, because the file serial already belongs to another laptop. The owner must edit or skip it.
  - Its review note shows `Serial will update: <old> → <new>` (English, like the other import notes).
- **Editable columns:** asset code, name, computer name, brand, model, serial, location, condition, note. These reuse `update_asset_import_row` field keys.
- **One open batch per file.** Opening Asset Import with a pending batch offers *Continue* or *Discard and load again*.
- After Approve, the batch is deleted, and its row ids and codes go into the existing `asset_import.import_valid_rows` audit payload.
- Loading a file into staging deletes every older `pending_review` batch, so the 11 stale Aug/Oct test batches go with the first load. (Changed while building: this needs no migration.)
- An existing asset with a blank serial gets the file serial (the note shows `(none) → …`).
- Light and dark mode both checked. Errors start with `Invalid` / `Not found` / `Cannot`.

## Left to the builder
- Exact table layout. Follow `EmployeeSyncDrawer.tsx`.
- Whether to show the old and new serial side by side or as a note.
