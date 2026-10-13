# Remote EE Sync Credential Cleanup and Account Approval Plan

> **For agentic workers:** Implement inline with focused checkpoints; do not merge or push.

**Goal:** Remove legacy HRMS account references from Staff Kit runtime configuration and support explicit, super_admin-controlled changes to the exact SQL login allowlist.

**Architecture:** Store the current approved login in encrypted `app_settings`, seeded to `user_reader`. Rust loads it for saved and environment profile checks before network/authentication. A dedicated confirmation command requires super_admin. When an exact legacy HRMS username is found in Staff Kit EE settings, clear that username and delete only the fixed Staff Kit Credential Manager target; do not read the secret. Account changes delete that same target and invalidate pending sync previews; every SQL use retains TLS and the existing read-only permission check before fixed Query B.

**Tech Stack:** Rust/Tauri, React/TypeScript, SQLite settings, Windows Credential Manager.

## Global Constraints

- No HRMS credential access or modification; no secrets in logs, source, settings or tests.
- Only exact currently approved username may reach SQL authentication; prohibited HRMS login is never approved.
- TLS, Query B, preview/approve flow, and fail-closed read-only permission verification stay unchanged.
- No live SQL or Staff Kit DB access/writes, merge, or push.

---

### Task 1: Regression tests for persisted allowlist and legacy cleanup

**Files:** `src-tauri/src/db/mssql_import.rs` tests.

- [ ] Add in-memory tests for initial `user_reader`, exact matching for saved/environment inputs, rejection before network, and rejecting the prohibited account even if proposed for approval.
- [ ] Add tests that exact legacy profile is cleared and requests deletion only of the Staff Kit credential reference; other names leave profile/secret untouched.
- [ ] Add tests that account change requires explicit confirmation and clears pending preview state.
- [ ] Run focused tests first and confirm failure before implementation.

### Task 2: Rust approved-account storage, authorization and cleanup

**Files:** `src-tauri/src/db/mssql_import.rs`, `src-tauri/src/lib.rs`.

- [ ] Replace the hard-coded allowlist with a persisted approved username seeded from `user_reader`; enforce it for settings, saved/environment profile resolution, and final `connect()` before TCP/TLS.
- [ ] Add a dedicated Tauri command guarded by `require_super_admin`, with typed confirmation, validated username, persisted actor/time metadata, and generic safe errors.
- [ ] On legacy profile match only, clear the prohibited username and delete only `staffkit.remote-ee-sync.sql-password.v1` without reading its blob. Never access other Credential Manager targets.
- [ ] On approved-account change, delete only the app-owned SQL secret target and clear pending previews so old credentials/previews cannot be reused.
- [ ] Preserve TLS, read-only permission query, and Query B unchanged; test all gates.

### Task 3: Settings UI, API and documentation

**Files:** `src/features/settings/RemoteEeSyncSettings.tsx`, `src/services/staff-api.ts`, `src/types/staff.ts`, sync spec.

- [ ] Show the active approved account and a separate change/confirmation control only to super_admin users; backend remains authoritative.
- [ ] Surface safe authorization/cleanup status; do not display prohibited account values or any password.
- [ ] Test the confirmation and visibility behavior; update the spec to describe the persisted account change process.

### Task 4: Verification, Qeen review, commit

- [ ] Run focused Rust/frontend tests, full Tauri tests, quality gate, format and diff checks.
- [ ] Have Qeen review exact commit read-only; verify no HRMS credential/data access and no live DB connection/writes.
- [ ] Commit only intended files on `feat/employee-mssql-preview-sync`; preserve pre-existing worktree changes and stop before merge/push.
