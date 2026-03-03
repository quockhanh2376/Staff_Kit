# Employee Manager App - Planning & Workflow cho AI IDE

Date: 2026-02-26

## 1) Product Scope

- Data source: Excel master (employee_id, full_name, start_date, computername, team, email, notes).
- Core features:
  - Search employee/team.
  - Import/update from Excel.
  - Manual CRUD.
  - Export CSV.
- Platforms:
  - Desktop: Windows, macOS, Linux.
  - Mobile: Android, iOS.
- Storage:
  - Local SQLite first.
  - Optional OneDrive sync for 2 users (phase 2).

## 1.1) Dev Data Source Convention (ExSource)

- In development environment, folder `ExSource/` is the master input source.
- Expected flow:
  - `ExSource/*.xlsx` (master files) -> import/normalize/validate -> SQLite.
  - UI does not query Excel files directly.
  - UI reads only from database for list/search/detail/report.
- Reports (CSV/UI views) are generated from current DB state after import.

## 2) MVP Goals

- Offline-first app with local DB.
- Import `.xlsx` and upsert data correctly.
- Fast search by name/email/team.
- Employee + team management UI.
- Export filtered data to CSV.

## 3) Locked UI Baseline

- Default: Dark mode.
- Theme toggle in header: `Dark | Light`.
- Main screens:
  - Employee List.
  - Import Excel Drawer.
  - Add/Edit Employee Drawer.
  - Teams Management.

## 4) Database Schema (SQLite + FTS5)

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS teams (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS employees (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  employee_id TEXT NOT NULL UNIQUE,   -- ma NV
  full_name TEXT NOT NULL,
  start_date TEXT,                    -- ISO date: YYYY-MM-DD
  computername TEXT,
  team_id INTEGER REFERENCES teams(id) ON UPDATE CASCADE ON DELETE SET NULL,
  email TEXT UNIQUE,
  notes TEXT,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE VIRTUAL TABLE IF NOT EXISTS employees_fts USING fts5(
  employee_id,
  full_name,
  email,
  notes,
  team_name,
  content=''
);

CREATE TRIGGER IF NOT EXISTS trg_employees_ai AFTER INSERT ON employees BEGIN
  INSERT INTO employees_fts(rowid, employee_id, full_name, email, notes, team_name)
  VALUES (
    NEW.id,
    NEW.employee_id,
    NEW.full_name,
    COALESCE(NEW.email, ''),
    COALESCE(NEW.notes, ''),
    COALESCE((SELECT name FROM teams WHERE id = NEW.team_id), '')
  );
END;

CREATE TRIGGER IF NOT EXISTS trg_employees_au AFTER UPDATE ON employees BEGIN
  DELETE FROM employees_fts WHERE rowid = OLD.id;
  INSERT INTO employees_fts(rowid, employee_id, full_name, email, notes, team_name)
  VALUES (
    NEW.id,
    NEW.employee_id,
    NEW.full_name,
    COALESCE(NEW.email, ''),
    COALESCE(NEW.notes, ''),
    COALESCE((SELECT name FROM teams WHERE id = NEW.team_id), '')
  );
END;

CREATE TRIGGER IF NOT EXISTS trg_employees_ad AFTER DELETE ON employees BEGIN
  DELETE FROM employees_fts WHERE rowid = OLD.id;
END;
```

## 5) Excel Column Mapping

- `employee_id` <- ma NV (required)
- `full_name` <- ho ten (required)
- `start_date` <- ngay bat dau (optional)
- `computername` <- ten may (optional)
- `team` <- ten team (required)
- `email` <- email (optional, unique if provided)
- `notes` <- ghi chu (optional)

## 6) Workflow - Import/Update Excel

1. Source file comes from `ExSource/` in dev (or user file picker in app).
2. Backend reads file with `calamine`.
3. Validate required columns and row-level formats.
4. Normalize values (trim, uppercase employee_id, parse date).
5. Upsert teams by `name`.
6. Upsert employees by `employee_id`.
7. Return report:
   - `inserted`
   - `updated`
   - `skipped`
   - `failed`
   - detailed error list.

## 7) Workflow - Search

1. User enters keyword.
2. Backend query:
   - `employees_fts MATCH ?`
   - join employees + teams for output model.
3. Apply optional filters: team, status, date range.
4. Return paginated result (`limit`, `offset`).

## 8) Workflow - Manual CRUD

- Create employee:
  - Validate required fields.
  - Enforce unique `employee_id` and unique `email` if not null.
- Update employee:
  - Update by `id`.
  - FTS sync via triggers.
- Delete employee:
  - MVP uses hard delete.
- Teams management:
  - Add/update/delete team.
  - Warn if team is in use before delete.

## 9) Workflow - Export CSV

1. User selects filters and output path.
2. Backend queries data with active filters.
3. Backend writes CSV with fixed headers.
4. Return success/failure status.

## 10) Optional OneDrive Sync (Phase 2)

- MVP is local-only, no real-time sync.
- Phase 2:
  - snapshot sync or CSV delta sync via OneDrive.
  - conflict policy: `last_write_wins` + merge warning log.

## 11) Tauri Command Contract

- `search_employees(query, team, limit, offset)`
- `import_excel(file_path, mode)`
- `create_employee(payload)`
- `update_employee(id, payload)`
- `delete_employee(id)`
- `list_teams()`
- `upsert_team(payload)`
- `delete_team(id)`
- `export_csv(filters, output_path)`

## 12) AI IDE Execution Plan

1. Setup DB layer + migration bootstrap.
2. Implement team/employee CRUD commands.
3. Implement Excel importer + validator + import report.
4. Implement FTS search + pagination query.
5. Wire UI table/filter/drawer/import/export.
6. Add tests for import, search, unique constraints.
7. Build desktop package and verify mobile config.

## 13) Acceptance Criteria

- Import one Excel file and upsert correctly with report.
- Search returns correct result by name/email/team.
- CRUD works without breaking unique constraints.
- CSV export follows selected filters and schema.
- Query/report views read from DB (not direct Excel read path).
- Desktop build passes with `cargo tauri build`.

## 14) Non-Functional Rules

- Do not commit `ExSource/`, runtime DB files, or secrets.
- Validate all file inputs and DB writes.
- Use parameterized queries only.
- Keep logs free of sensitive data.
