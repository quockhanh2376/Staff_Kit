# EE List sync staging table: design (2026-11-23)

## Intent / why
- Today the EE List sync (HR MSSQL to local `employees`, group `employee_list`) is blocked when it hits one conflict. Approve is disabled. The preview lives only in memory (`PENDING_EMPLOYEE_LIST_SYNCS`), so it is lost when the app closes.
- The owner (IT) wants **update** to always load the HR list into a **staging table inside the app**. There they review and fix every New / Changed / Missing / Conflict row, then write the result to the real employee table in one step.
- **Why:** to avoid conflicts and bad writes in the database. Data should be fixed by hand in the app instead of the whole sync being blocked.

## Goals
- Conflicts never block staging. Every row can be decided on its own.
- Each staged row can be edited before Approve: full name, nick name, email, Azure AD account.
- Staging is stored in the database. It survives an app restart, so work can be finished later.
- Approve writes exactly what was reviewed, in one transaction. A mandatory safety snapshot is taken first, as today.
- Rows whose local employee changed after staging are flagged, never silently overwritten.

## Non-goals
- No change to the MSSQL connection, the read-only account, or Query B.
- No change to the Excel employee import.
- No sync for groups other than EE List, beyond the group moves below.
- No multi-user locking of a staging batch. One admin works on it at a time.

## Anti-goals (failure even if it "works")
- Staged data shows up in any real screen (employee list, borrow, export, counts) before Approve.
- Approve overwrites a local edit made after staging without the owner seeing it.
- A floating employee, or one still holding assets, is offboarded without the owner seeing it.

## Decisions made with the owner
- **Conflict row:** the HR staff code exists locally in another group (Onboarding / Offboarding / Internal movement). The row offers **Move to EE List** (default, ticked) or **Skip**, chosen per row. The editable fields can be changed before writing. Moving keeps the employee's record and loans and updates their details from HR (after edits).
- **Missing row:** an EE List employee who is not in HR. The row offers **Move to Offboarding**. It is **ticked by default**, and the owner unticks the rows to keep.
- **Approach:** a persisted staging table (option 1 of 3). Option 2, an in-memory preview with per-row choices, loses edits on close. Option 3, a "pending" flag on the real `employees` table, pollutes real data. Both were rejected.

## Claude's calls (proposed, shown to the owner)
- New and Changed rows also get a **Skip** tick. Default: apply.
- **Floating** employees (`is_floating = 1`) on a Missing row default to **not** ticked, because they are usually absent from HR on purpose.
- A Missing row shows how many assets the employee currently holds. The row stays ticked, and the count lets IT see who must return machines.
- Group moves reuse the existing semantics in `move_employees_group_conn` (`employee.rs:546`):
  - to EE List: lifecycle `active`, or `rehired` if coming from Offboarding, and `contract_end_date` is cleared when coming from Offboarding;
  - to Offboarding: lifecycle `resigned`, `is_floating = 0`.
- Who can use it: admin and super admin (`require_admin`), the same gate as today's preview and apply.
- **One open batch at a time.**
  - Pressing **update** while a batch is open asks whether to discard it and load a fresh one, or to continue the open one.
  - **Owner decision: nothing is left behind.** After Approve, and after Skip (discard), the batch and all of its staged rows are deleted from the database. Only the one-line audit summary remains.
- **Drift check at Approve.**
  - Each staged row stores a fingerprint of the local employee row it was built from (full name, nick, email, Azure AD, staff_group, `updated_at`).
  - At Approve, a row whose local employee no longer matches is **not written**. It is marked `stale` with the note `Changed in the app after this list was loaded — review again`, and Approve stops so the owner can re-check it.
  - HR is **not** re-fetched at Approve. The staged values (after edits) are what the owner approved.
- Edited values are trimmed, and the email is lowercased, the same normalisation as today. The full name is required for any row that will be written.
- **A staff code that appears twice in the HR source** becomes one staged row of kind `conflict` with the note `Duplicate Staff Code in HR source`. Its default is Skip, so it is never applied twice. (Claude's call; today's code would process both.)

## Owner decisions from the builder check
- **Stale rows block the whole batch (all-or-nothing).** Approve writes nothing while any row is `stale`.
  - Each stale row has a **"Reload from app"** action (`refresh_employee_sync_row`). It re-reads the current local row, updates `local_json` and the fingerprint, recomputes the kind and default action if the kind changed, keeps the owner's edits, and sets the state back to `pending`.
- **Confirm before offboarding asset holders.** When Approve would move people who still hold assets to Offboarding, the UI first shows a confirm, e.g. `3 people being offboarded still hold 5 assets. Continue?`. The Approve call proceeds only on yes.

## Claude's calls from the builder check
- Group moves:
  - `move_employees_group_conn` is split into a `_tx` variant that takes `&Transaction`. The old function stays as a thin wrapper. The move SQL is never copied.
  - A conflict row that moves to EE List gets `is_floating = 0`, as today.
  - The move also writes the HR values over name, nick, email and Azure AD, with the owner's edits taking priority, the same as New/Changed rows.
- Conflict rows carry full local data. The staging snapshot also loads the full local row for codes in other groups (id, group, name, nick, email, Azure AD, `updated_at`), so conflict rows have a Local name, a `local_group` and a fingerprint.
- Drift check by row kind:
  - **New** row: stale if that staff code now exists locally in any group. This prevents a duplicate insert.
  - **Missing** or **Conflict** row: stale if the employee was deleted or moved to a different group since staging.
- Approve is disabled when no row will write. A batch with nothing to write is closed with Skip (discard).
- A duplicate HR code is stored as one row holding both HR records in `source_json`. It allows Skip only and takes priority over the normal kind. The owner fixes it in HR and loads a new batch.
- Scope and old commands:
  - One PR. The old preview and apply commands and `PENDING_EMPLOYEE_LIST_SYNCS` are removed.
  - The settings-change cleanup at `mssql_import.rs:839` deletes any open batch instead, because a different HR source invalidates it.

## Design

### Data
- New table `employee_sync_batches`:
  - `id`
  - `created_at`
  - `created_by` (actor ref)
  - (no status column: a batch exists only while it is open)
  - `source_row_count`
- New table `employee_sync_rows`:
  - `id`
  - `batch_id`
  - `staff_code`
  - `kind`: `new` | `changed` | `unchanged` | `missing` | `conflict`
  - `local_employee_id` (nullable)
  - `local_group` (nullable)
  - `source_json` (the HR values)
  - `local_json` (local values plus the fingerprint)
  - `edited_json` (the owner's field edits, nullable)
  - `action`: `apply` | `skip` | `move_to_employee_list` | `move_to_offboarding` | `none`
  - `notes_json`
  - `state`: `pending` | `stale`
- Both tables are created through the normal migration path. `SCHEMA_USER_VERSION` goes from 3 to 4, and the tables are added to the base schema.
- `unchanged` rows are stored only as a count. They are not shown as editable rows.

### Backend (`src-tauri/src/db/mssql_import.rs` plus a small new module if it grows)
- `stage_employee_list_sync` replaces the in-memory preview. It:
  - fetches HR (Query B);
  - reads the local snapshot;
  - runs `compare_employee_list`;
  - writes the batch and its rows with their default actions;
  - returns the batch summary and rows.
- `get_open_employee_sync_batch` returns the open batch, if any, so the UI can resume it after a restart.
- `update_employee_sync_row(row_id, action?, edits?)` validates the action against the row kind and validates the edits.
- `discard_employee_sync_batch(batch_id)`.
- `apply_employee_sync_batch(batch_id)`:
  1. Take the mandatory safety snapshot.
  2. In one transaction: run the drift check on every row that will be written. If any row is stale, mark it, roll back the writes, and return a business error starting `Cannot apply: N rows changed in the app — review them` (the stale marks are saved).
  3. Insert or update in the EE List, using the edited values or else the source values.
  4. Move conflict rows to EE List and update their details.
  5. Move ticked Missing rows to Offboarding.
  6. Write one audit row `business.import.mssql` with the counts per action, and delete the batch and its rows in the same transaction.
- All commands use `require_admin`, and the lib.rs command-classification test lists them.
- Errors start with `Invalid`, `Not found` or `Cannot`.
- The old `preview_employee_list_mssql_sync` / `apply_employee_list_mssql_sync` commands and the in-memory store are removed once the UI uses the new ones.

### Frontend (`src/features/employees`)
- **update** opens a large drawer, **EE List sync**, in place of the small popover. The drawer shows:
  - counts (New / Changed / Missing / Conflicts / Unchanged);
  - a filter by kind;
  - a table with one row per staged row: Staff code, Kind, Local name, Source name, editable fields (inline inputs), and the action (a checkbox or select, depending on kind);
  - notes such as the assets held and the stale reason.
- Each row is saved when it changes, through `update_employee_sync_row`, so a restart keeps the edits.
- The **Approve** button is enabled when at least one row will write. It shows the per-action totals, for example "Add 4 · Update 9 · Move 6 to EE List · Offboard 2".
- **Skip** (discard) asks for confirmation.
- After Approve: a green notice (`setGlobalNotice`) and the employee list refreshes.
- Light and dark mode both use theme tokens.

## Tests (minimum)
- **Rust:**
  - staging with conflicts creates a batch;
  - default actions: conflict → move_to_employee_list; missing → move_to_offboarding; floating missing → none;
  - edits are applied on Approve;
  - a conflict move keeps the employee id and its loans, and sets lifecycle;
  - a missing move sets lifecycle `resigned`;
  - a stale row blocks Approve and writes nothing;
  - a skipped row writes nothing;
  - discard, and a successful Approve, leave both staging tables empty;
  - migration v3 → v4;
  - a duplicate HR code becomes one row, defaulting to skip.
- **Vitest:**
  - the drawer renders kinds and defaults;
  - toggling an action calls `update_employee_sync_row`;
  - the Approve totals text;
  - a stale error shows on its row;
  - resume of an open batch.

## Left to the builder
- Exact drawer layout and column widths.
- Whether rows save on blur or debounced.
- The module split inside `mssql_import.rs`.
