# Remote EE Sync Secure Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver secure Remote EE Sync settings, verify source SQL permissions before Query B, and keep the existing preview → approve → local EE List-only write boundary.

**Architecture:** Rust owns connection-profile resolution, Windows Credential Manager access, permission verification, fixed Query B execution, and guarded Tauri commands. SQLCipher `app_settings` stores non-secret profile metadata only. React exposes one admin Settings panel and calls commands without raw saved credentials. The legacy freeform MSSQL import surface is removed; Excel import remains unchanged.

**Tech Stack:** Rust/Tauri v2, rusqlite SQLCipher, Tiberius 0.12, Windows Credential Manager via windows-sys, React 19, TypeScript, Vitest.

## Global Constraints

- Work only in `feat/employee-mssql-preview-sync` worktree; no merge or push before Qeen validates the exact commit.
- Source SQL Server is read-only; reject if effective server/database/schema/object/role permissions include write/escalation access or cannot be verified.
- Source data is fetched only with the approved Query B; permission introspection may use read-only metadata SELECTs.
- No remote DML/DDL. Test Connection and preview never write either database; only explicit approval may write local EE List rows.
- Passwords never appear in source, logs, SQLite, exports, frontend state after save, or returned DTOs. Use Windows Credential Manager for persistent SQL passwords.
- Environment profile overrides saved profile only when a complete `STAFFKIT_MSSQL_*` configuration is present; partial override fails closed and is never persisted.
- Preserve `web/` and all existing untracked files.

### Task 1: Update security and configuration contract tests

**Files:** `src-tauri/src/db/mssql_import.rs`, `src-tauri/src/lib.rs`

- [ ] Add tests for safe settings DTO JSON excluding password, config override all-or-nothing, authentication-required fields, and Query B fixed/no caller SQL.
- [ ] Add tests for Test Connection result contract (PASS/FAIL, safe reason, count) and that permission failure prevents Query B.
- [ ] Add/adjust Tauri command registry tests that assert arbitrary MSSQL query/import commands are absent and settings/sync commands are present.
- [ ] Run focused Rust tests and observe the expected failure before production changes.

### Task 2: Add secret store and safe connection profile

**Files:** `src-tauri/Cargo.toml`, `src-tauri/src/db/mssql_import.rs`, `src-tauri/src/db/mod.rs`

- [ ] Add Windows-only `windows-sys` Credential Manager dependency/features after verifying the installed 0.61 API.
- [ ] Define a non-serializable internal credential/profile type, a serializable password-free Settings DTO, auth enum, fixed credential target, and environment variable names.
- [ ] Implement Windows Credential Manager read/write/delete and non-Windows fail-closed errors; ensure DTO/debug/error paths never include the secret.
- [ ] Store server/database/port/auth type/username and secret reference using SQLCipher settings helpers; do not persist environment overrides.
- [ ] Add guarded get/save/remove secret commands with explicit password status and tests for no password serialization/storage.
- [ ] Verify focused tests and Windows Rust compilation.

### Task 3: Make all MSSQL access use the profile and fixed Query B

**Files:** `src-tauri/src/db/mssql_import.rs`, `src-tauri/src/lib.rs`

- [ ] Resolve either a complete environment override or saved profile plus credential; never combine a partial environment override with persisted settings.
- [ ] Build Tiberius config using server, port, database, auth type, and secret internally. Support SQL authentication and Windows Integrated authentication on Windows only.
- [ ] Keep the effective-permission preflight fail closed, including inherited role permissions; use configured DB context and do not accept credentials or SQL through a generic import command.
- [ ] Implement Test Connection as permission gate + Query B fetch; return status, safe reason, and row count, never employee rows or raw driver detail.
- [ ] Change sync preview/apply to resolve credentials internally. Apply rechecks permissions and refetches fixed Query B before any local write.
- [ ] Remove legacy custom-query preview/import Tauri commands and any SQL command that takes user-provided SQL or target group; retain pure upsert helpers only if used by tests or fixed local apply.
- [ ] Test that permission rejection blocks Query B, connection errors are sanitized, and the only source data SQL constant is Query B.

### Task 4: Add Settings → Remote EE Sync UI

**Files:** `src/features/settings/RemoteEeSyncSettings.tsx` (new), `src/features/settings/SettingsView.tsx`, `src/types/staff.ts`, `src/services/staff-api.ts`, relevant frontend tests

- [ ] Add types/API for saved profile, password-free status, save/remove, and Test Connection result.
- [ ] Build form fields Server, Database, Auth type, Username, Password. Password stays blank on load; show only “saved securely” status.
- [ ] Save non-secret profile and password to distinct backend APIs, preserve password on blank save, and offer a separately explicit remove-password action.
- [ ] Show Test Connection PASS/FAIL reason and Query B row count; no settings view prints credentials.
- [ ] Replace Settings’ “Import from MSSQL” button with Remote EE Sync panel entry or section. Remove MSSQL tab from ImportDrawer and delete/freeform panel wiring; preserve Excel import.
- [ ] Add component/API tests for password non-prefill, save, fail-closed test status rendering, and no freeform SQL input.

### Task 5: Refactor EE List Update to configured profile

**Files:** `src/features/employees/useEmployeeState.ts`, `src/features/employees/useEmployeeState.mssql.test.tsx`, `src/services/staff-api.ts`, `src-tauri/src/lib.rs`

- [ ] Make Update call `previewEmployeeListMssqlSync()` with no host/user/password arguments.
- [ ] Map safe backend errors to actionable guidance mentioning Settings → Remote EE Sync; remove old credential checks in React.
- [ ] Keep explicit approval boundary, stale-snapshot check, local-only mapped apply, Missing report-only, and success refresh.
- [ ] Verify tests prove no secret enters frontend sync calls and prior no-write-before-approval behaviors still pass.

### Task 6: Full validation, Qeen review, and handoff

**Files:** `daily_log.md` and implementation files

- [ ] Review diff for secret leaks, arbitrary source SQL, permission gate bypasses, settings/export leakage, and unrelated Employee/loan/auth changes.
- [ ] Run focused Rust/frontend tests, `npm run test:unit`, `npm run test:tauri`, `npm run check:quality`, and `git diff --check`.
- [ ] Inspect only environment variable key presence, never print values. If configured, require read-only preflight before any Query B/direct mapping check; if not configured or unverifiable, report BLOCKED without DB writes.
- [ ] Append End evidence and commit feature branch only.
- [ ] Ask Qeen to independently inspect the exact commit, read-only permission gate, fixed Query B mapping/counts, and DB safety. Do not merge or push.
