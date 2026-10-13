# EE List MSSQL Preview Sync Implementation Plan

> REQUIRED SUB-SKILL: use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace EE List Update's immediate local upsert with a fixed Query B fetch, local read-only comparison preview, explicit approval, and transactional local EE List-only apply.

**Architecture:** Add a dedicated sync lifecycle in the Rust MSSQL module. It owns a fixed SELECT (no caller SQL), normalizes/deduplicates source Staff Codes, reads local EE List rows without migrations or writes, and stores preview snapshots in managed in-memory state. Approval re-fetches the source and verifies the snapshot, then applies only New/Changed rows to `employee_list` in one SQLite transaction. Tauri exposes admin-guarded preview/apply commands; React owns review, confirmation, result, and error state. The legacy generic import command remains separate and is not called from EE List Update.

**Tech Stack:** Rust, Tauri v2, rusqlite/SQLCipher, tiberius, React 19, TypeScript, Vitest.

## Global Constraints

- Work only in `feat/employee-mssql-preview-sync` worktree.
- Do not merge/push; Qeen must validate the exact final commit first.
- Never write to the live MSSQL source. Before any MSSQL query, check current login roles and effective database/table permissions; fail closed when write permission is present or permission evidence is unknown. The EE List sync path can execute only the fixed Query B SELECT.
- Preview must not write local SQLite, run migrations, enable WAL, or update last-sync storage. Use a SQLite read-only connection and `PRAGMA query_only=ON`.
- Only explicit approval may write local SQLite, in one transaction, and only to EE List records/mapped fields. Missing rows are report-only. Other groups, Employee master data, loans, assets, and auth are untouched.
- Do not print credentials, employee data, or source query rows in logs. No direct source DB validation is possible until read-only credentials are configured.

---

### Task 1: Add failing Rust tests for fixed Query B normalization and preview diff

**Files:** `src-tauri/src/db/mssql_import.rs`

1. Add tests that assert the EE List sync query has INNER JOIN `AspNetUsers`, `Resigned = 0`, the approved five fields including `COALESCE`, and no `TOP` cap; assert sync accepts no arbitrary SQL argument.
2. Add pure tests for normalized Staff Code matching and New/Changed/Unchanged/Missing classifications, including duplicate codes and non-EE-List collisions.
3. Run the focused Rust test filter and confirm the new tests fail because the dedicated sync API/diff logic does not exist.

### Task 2: Implement source query and compare model

**Files:** `src-tauri/src/db/mssql_import.rs`

1. Define dedicated serializable DTOs for preview rows, category counts, conflicts, preview ID, and apply result; do not reuse generic arbitrary-query DTOs where that weakens the fixed-query boundary.
2. Add a fail-closed read-only permission preflight before all MSSQL query paths. Reject write-capable fixed roles, effective server/database permissions outside an explicit read-only allowlist, and DML/DDL/EXECUTE permissions on schemas, tables, views, procedures, functions, database users/roles, or server logins. Treat SQL errors, NULL permission checks, missing required CONNECT permissions, and unknown permission results as unverifiable and block the query.
3. Add a fixed Query B constant: `INNER JOIN [AssetManagement].[dbo].[AspNetUsers]`, `WHERE Resigned = 0`, `ROW_NUMBER` deduplication by normalized Code with deterministic `S.Id` ordering, exact approved mappings, and no TOP limit.
4. Normalize Staff Code by trim + uppercase for matching. Preserve the source display Code. Compare records against local `employee_list` by normalized Code and classify New, Changed, Unchanged, Missing. Detect codes owned by another staff group and block those rows rather than moving/updating them.
5. Run focused Rust tests and confirm they pass.

### Task 3: Implement a genuinely read-only preview snapshot

**Files:** `src-tauri/src/db/mssql_import.rs`, `src-tauri/src/db/mod.rs`

1. Add a dedicated local SQLite open helper for preview using SQLCipher key and `SQLITE_OPEN_READ_ONLY`; set `PRAGMA query_only=ON`. It must not call `ensure_database_ready`, migrations, WAL configuration, or `open_runtime_connection`.
2. Read only required EE List fields and relevant group-collision codes. Do not persist preview rows or status.
3. Fetch Query B using the configured SQL login, store the exact source/local comparison snapshot in managed process memory with an opaque ID and creation time, and return only preview DTO data.
4. On connection/query/local-read error, return safe actionable errors without logging secrets or row contents.
5. Add tests proving preview does not mutate the local database and failed source/local reads leave rows unchanged.

### Task 4: Implement approval with stale-preview guard and scoped transaction

**Files:** `src-tauri/src/db/mssql_import.rs`

1. Add an apply function that accepts only a preview ID, re-fetches the same fixed Query B, re-reads the current local snapshot, and rejects if either differs from preview.
2. On a current snapshot, start one local transaction; insert New rows and update only mapped fields for Changed `employee_list` rows. Never delete Missing rows or modify other groups.
3. Roll back all changes on any apply error. Return counts and completion timestamp only after commit.
4. Test approval gating, stale source/local rejection, transactional rollback, missing preservation, and non-EE-List preservation.

### Task 5: Add guarded Tauri IPC and typed frontend API

**Files:** `src-tauri/src/lib.rs`, `src/services/staff-api.ts`, `src/types/staff.ts`

1. Add admin-authorized preview/apply commands. Preview command takes configured credentials only and no SQL/query string. Apply command takes only the preview ID.
2. Register commands and capabilities consistently with existing admin guards.
3. Add TypeScript DTOs and typed API methods for preview/apply; keep legacy generic import API available only to its existing separate workflow.
4. Add/adjust command tests that assert the sync IPC has no arbitrary SQL input and approval is required for apply.

### Task 6: Replace EE List Update UI with preview/review/approve lifecycle

**Files:** `src/features/employees/useEmployeeState.ts`, `src/features/employees/EmployeeView.tsx`, `src/features/employees/useEmployeeState.mssql.test.tsx`

1. Change Update to request and retain a preview; do not call `testMssqlConnection` followed by `importMssqlStaff`.
2. Show New/Changed/Unchanged/Missing counts and reviewable row details, plus conflicts/duplicates and the fixed-query inner-join scope. Missing is informational only.
3. Require an explicit approval action before calling apply. On apply success, refresh EE List and show completion time/counts. On preview/apply failure, show an actionable safe error and leave state reviewable.
4. Test preview-before-apply, no apply before approval, cancel/dismiss, failure feedback, successful refresh, and missing rows not silently treated as deleted.

### Task 7: Verify security boundaries, run full checks, and record handoff

**Files:** implementation files above, `daily_log.md`

1. Review exact diff for no SQL write statements in the dedicated sync path, no arbitrary query parameter, no preview local writes, no changes to Employee master/loan/asset/auth logic, and no secret/employee row logging.
2. Run focused Rust and frontend tests, `npm run test:unit`, `npm run test:tauri`, and `npm run check:quality`.
3. Record that live MSSQL direct count/mapping comparison is pending until source read-only credentials are supplied; do not substitute fixtures for that evidence.
4. Append daily-log End record with commit SHA, checks, and DB safety. Commit feature branch only; request Qeen independent validation of the exact commit.
