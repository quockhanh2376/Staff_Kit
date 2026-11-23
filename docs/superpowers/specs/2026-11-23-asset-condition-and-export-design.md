# Asset Condition (New / Spare / Broken) + shared Export — design (2026-11-23)

## Intent / why
IT keeps laptops, monitors, phones… in **Kho IT**. The owner's inventory Excel already carries a `Status` column with **New / Spare / Broken**. They want Staff Kit to:
1. find usable machines quickly to hand to new staff,
2. track broken machines to repair or dispose of,
3. report counts: how many machines in total, how many are with staff, and how the stock splits into New / Spare / Broken,
4. export those lists (whole report or one slice) to Excel / CSV.

Today an asset has only a loan status (`in_stock` / `assigned`). The import ignores the `Status` column, so every imported machine becomes `in_stock`, and the LAN borrow page (`search_in_stock_assets_conn`) lets staff pick a broken machine.

## Goals
- A **condition** per serialized asset: `new`, `spare`, `broken` (display New / Spare / Broken). Separate from loan status.
- **New and Spare can be handed out; Broken cannot.** Staff must not find a Broken machine on the LAN borrow page, and an admin cannot approve a loan for one.
- Automatic transitions:
  - asset becomes `assigned` → condition `new` becomes `spare` (first hand-out ends "new"); `spare` stays `spare`.
  - asset returns to `in_stock` → condition becomes `spare`, **unless it is `broken`** (an IT mark made while it was out is kept).
- **Manual change** of condition by **admin or super admin** via a quick selector on the asset row; every change audit-logged. IT marks Broken when a machine fails, and sets it back to Spare after repair.
- **Import** reads the file's status column into condition.
- **Asset Dashboard**: Condition column, Condition filter, and count cards for New / Spare / Broken in stock (plus total and assigned).
- **Export** of assets (admin + super admin), with scope / category / team filters, to Excel or CSV, built on **one shared export module** that the employee export also uses.

## Non-goals
- No other conditions (no "repairing", "disposed").
- Condition is not tracked for quantity (stock_items) assets.
- No change to who can edit the descriptive fields: Edit Asset drawer stays super-admin only.
- No new export for other screens beyond Employees + Assets (the shared module just makes that easy later).

## Anti-goals (failure even if it "works")
- A Broken machine reachable by staff or approvable by an admin through **any** path.
- Changing loan logic (`in_stock`/`assigned`) semantics or loan history.
- Two export code paths that format CSV/XLSX differently.
- Breaking the existing employee export (same columns, same file names, existing tests keep passing).

## Decisions made with the owner
- IT decides every condition and loan state. Staff only receive and return exactly what was handed out. Machines held by staff are assumed usable; Broken is set by IT.
- Condition values: only New, Spare, Broken.
- Who changes condition: admin **and** super admin, through a small per-row selector (option 1). Descriptive fields stay super-admin only.
- New → Spare automatically after first assignment; return → Spare automatically unless Broken.
- Existing assets after upgrade: all **Spare** (keeps today's behaviour: everything assignable).
- Import: status cell `new` / `spare` / `broken`, case-insensitive, trimmed. Blank or unknown → **Spare**, and the import preview flags unknown values per row.
- Export:
  - Button on Asset Dashboard opens a small dialog: **Scope**, **Category**, **Team**, **Format**.
  - Scopes: All assets · Assignable (New + Spare, in stock) · Spare only · New only · Broken · Assigned to staff · Current table filter.
  - Category: All or one category. Team: All or one team, matched like the employee list filter (the team or its direct child teams, `employee.rs:951`), on the **holder's** team; in-stock machines have no team and are excluded when a team is chosen. *(Claude's call.)*
  - Always exports from the full data set (not the current page).
  - Excel: sheet **Summary** (counts for the exported rows: total, assigned, in stock New / Spare / Broken, broken down by category) + sheet **Assets**. CSV: Assets rows only.
  - Assets columns: Asset code, Category, Name, Brand, Model, Serial, Computer name, Location, Loan status (Assigned / In stock), Condition, Holder staff ID, Holder name, Holder team. For the "Assigned to staff" scope the holder columns come first.
  - File name: `StaffKit_Assets_<Scope>_<YYYY-MM-DD>.<xlsx|csv>`.

## Approaches considered
1. **New `condition` column on `assets` (picked).** Clean filter, count, and guard; loan logic untouched.
2. Put `broken`/`new` into the existing `status` column — rejected: loan code assumes two values and would mis-handle returns and counts.
3. Free text in Notes — rejected: cannot filter, count or block reliably.

Export:
1. **One shared frontend module (picked):** move the generic parts of `src/features/employees/employeeListExport.ts` (CSV escape + BOM, XLSX build via `write-excel-file`, save dialog + `staffApi.writeExportFile`) to `src/lib/tableExport.ts`; employee and asset exports become thin row builders on top of it.
2. Backend-generated files — rejected: duplicates formatting in Rust and the frontend already has everything.

## Design

### Data
- Column: `condition TEXT NOT NULL DEFAULT 'spare' CHECK (condition IN ('new','spare','broken'))` in the `CREATE TABLE` (fresh installs) **and** an `ensure_asset_condition_column` step that adds it only when missing (column-exists check, like `ensure_team_columns`). Raising `SCHEMA_USER_VERSION` (now 1, `mod.rs:540`) re-runs `BASE_SCHEMA_SQL` and every `ensure_*`, so a bare `ALTER TABLE` would fail on fresh DBs; it also triggers the mandatory safety snapshot on existing DBs, which is expected. Old DBs get `spare`.

### Backend
- Transitions live in **one place**, `asset::set_asset_status_tx`, and apply only to the `assigned` and `in_stock` targets (`retired`/`disposed`/`lost` in `borrow.rs:31-33` leave condition alone): when setting `assigned`, also `condition = CASE WHEN condition = 'new' THEN 'spare' ELSE condition END`; when setting `in_stock`, `condition = CASE WHEN condition = 'broken' THEN 'broken' ELSE 'spare' END`.
- Every other code path that writes `status = 'assigned'` directly must go through that helper or apply the same rule. Known ones: `historical_assignment_import.rs:929` (transfer) and `:1212` (`apply_single_historical_assignment_tx`, main path); `asset_import.rs:2551` goes through the helper.
- Hand-out guards (all must reject `broken`):
  - `search_in_stock_assets_conn` (LAN borrow search) adds `AND a.condition <> 'broken'`.
  - LAN borrow submit (`borrow.rs:671`) and admin approval (`borrow.rs:1297`) reject a broken asset with `Cannot assign broken asset <code>`.
- New command `update_asset_condition(asset_id, condition)` with `require_admin`. It validates the value (`Invalid condition`), returns `Not found …` for a missing asset, writes nothing when unchanged, and writes audit `asset.update_condition` with before/after.
- Dashboard row (`AssetDashboardSerializedRecord`) gains `condition` and `holder_team_name` (join employee → team). The summary gains in-stock counts per condition.
- Import (serialized wizard, `asset_import.rs`):
  - Add a status/condition alias set (`status`, `condition`, `tinhtrang`, `trangthai`) to `AssetImportFieldMapping`, and a `condition` value on each `asset_import_rows` row so the preview can show and edit it.
  - New assets get the parsed condition.
  - **Rows matching an existing asset** (today marked `skipped`, ~`asset_import.rs:3052-3080`) instead update **only the condition** from the file. A blank cell keeps the current value. Other fields are never touched by this path. *(Owner's decision.)*
  - **Highlight for IT review** in the preview (owner's decision), each with a short reason. IT can edit the condition in the preview before approving:
    - existing asset whose condition will change (`Spare → Broken`, …);
    - unknown or blank status value (imported as Spare);
    - a row with an owner (assigned on import) whose file condition is **Broken** (imported and kept Broken) or **New** (becomes Spare by the hand-out rule).
- Owner-aware import (`asset_import.rs:2551`) and historical assignment import (`historical_assignment_import.rs:929, :1212`) **do not refuse** Broken or New-with-owner rows. They record reality and highlight those rows in their preview for IT to correct. *(Owner's decision: staff are assumed to hold working machines; only IT decides condition.)*

### Frontend
- `src/lib/tableExport.ts`: `escapeCsvCell`, `buildCsv(headers, rows)`, `buildXlsxBlob(sheets)`, `saveExport(blob, filename, ext)`. The employee export is rewritten on top of it with identical output; its tests move or stay green.
- Asset Dashboard:
  - Condition column.
  - Condition filter next to the category filter.
  - Count cards: Total, Assigned, New, Spare, Broken.
  - Per-row condition selector for admins (New / Spare / Broken), disabled while saving, errors shown inline. **New** is offered only for in-stock assets that are already New (it cannot be set back from Spare, and never on an assigned asset); Spare ↔ Broken is always allowed. *(Claude's call: keeps "first hand-out ends New".)*
  - Count cards Total / Assigned / New / Spare / Broken sit **beside** the existing per-category cards (none removed), counted over the whole data set. *(Claude's call.)*
  - Manual create form (`create_asset_manually`) gets a Condition picker, New or Spare, default **New** (a hand-entered machine is usually a new purchase). `upsert_assets` keeps the column default. *(Claude's call.)*
  - **Export** button → dialog (scope, category, team, format) → builds rows from the full serialized list in memory (already loaded by the dashboard) → `tableExport`.
- Edit Asset drawer shows the condition read-only. It is changed with the row selector so there is one way to do it.

### Fixes to existing export code found during review
- **CSV formula injection:** cells beginning with `=`, `+`, `-`, `@`, tab or CR are written as-is, so a crafted name or note opened in Excel runs as a formula. The shared `escapeCsvCell` prefixes such cells with `'`, **except** plain numbers / phone numbers (`^[+-]?[0-9][0-9 ().-]*$`, e.g. `+84 912…`, `-5`), which stay unchanged. CSV only; XLSX cells are typed strings and already safe. This is the one intended change to the employee CSV output. *(Claude's call.)*
- **No success feedback:** after saving, the employee export shows nothing. The shared `saveExport` returns the saved path (or `null` when cancelled) so both screens can show "Saved to …".
- **Numbers written as text in XLSX:** fine for IDs, but Summary counts must be numbers. The shared XLSX builder accepts typed cells.
- **Known, not fixed here:** PERF-19 (export bytes cross IPC as a JSON number array). Acceptable for these file sizes; it stays on the backlog.

## Tests (minimum)
- Rust:
  - migration sets `spare` on an old DB;
  - transitions on approve/return, including broken kept on return and new → spare;
  - LAN search excludes broken;
  - approval of broken rejected;
  - `update_asset_condition` audit, no-op, invalid value, not found, plain user rejected;
  - import parses New/Spare/Broken/blank/unknown;
  - command-classification list updated.
- Vitest:
  - `tableExport`: CSV escaping, including the formula-injection guard;
  - employee export output unchanged;
  - asset export row builder per scope, team and category filter;
  - Summary counts;
  - condition selector visible for admin, hidden for plain user;
  - dashboard filter and cards.

## Left to the builder
Exact UI layout of the dialog, the cards and the selector; the audit payload shape (follow `asset.update_details`); the alias list beyond the ones above; whether the condition filter and the export "current table filter" share state.
