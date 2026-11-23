# Staff_Kit Optimization Backlog

- **Revised:** 2026-11-19 (replaces the 2026-08-05 revision)
- **Base:** `main` @ `d1d91ac` (v2.2.1)
- **Sources reconciled:** previous revision of this file (2026-08-05), `TotalSpec.md` (2026-10-27), and a full read-through audit of `src/` and `src-tauri/src/` on 2026-11-19.
- **Status:** this is the single canonical forward-looking backlog. `TotalSpec.md` stays as the design/spec reference; its section 3 status column is superseded by this file.

How to read the "Checked" column:
- **verified**: the code path was read end to end and the failing case confirmed.
- **traced**: found by the audit and read in the code, but not independently re-checked.
- **plausible**: the code allows it; the failing case depends on timing or runtime state that was not reproduced.

Nothing in this revision was reproduced by running the app. All statuses come from reading code.

---

## Summary

| Group | Count |
|---|---|
| New findings (2026-11-19 audit) | 27 |
| Carried over, still open | 25 |
| Carried over, partially fixed | 13 |
| Closed since the previous revisions | 19 |
| Fixed after this audit (PR #51, #53, #54) | 6 |

New findings by severity: High 5, Medium 13, Low 9. Still open after the fixes: High 0, Medium 13, Low 7 (SEC-016, BUG-22, BUG-23, REL-005, SEC-019, BUG-28, SEC-015 closed — see section 4).

### Recommended next targets

1. ~~**SEC-015**: export can overwrite `db_settings.json`.~~ Fixed.
2. **BUG-24, BUG-27, BUG-25, BUG-34**: small, independent data-loss/crash fixes.
3. **BUG-32, BUG-31, BUG-33**: borrow and reset flows that dead-end.
4. Follow-up from REL-005: a corrupt `db_settings.json` still falls back to the local DB silently.

---

## 1. New findings (2026-11-19)

### 1.1 High

| ID | Type | Location | Problem (concrete case) | Smallest fix | Checked |
|---|---|---|---|---|---|

### 1.2 Medium

| ID | Type | Location | Problem (concrete case) | Smallest fix | Checked |
|---|---|---|---|---|---|
| BUG-24 | data-loss | `src/features/employees/useTableEdit.ts:86-94` | After 2 minutes without input the inactivity timer clears all table-edit drafts and leaves edit mode with no warning. An admin who edits several cells and steps away loses them. | Do not auto-exit (or re-arm the timer) while drafts exist. | verified |
| BUG-27 | bug | `src/features/settings/AuditLogViewer.tsx:318-320` | `JSON.parse(selectedLog.payloadJson)` runs during render and `src/` has no error boundary. One audit row with a non-JSON payload unmounts the whole app when opened. | Parse in try/catch and fall back to the raw string. | verified (code); not checked whether the backend ever writes non-JSON |
| BUG-32 | bug | `src-tauri/src/db/mod.rs:196-205` (`reset_all_data`), `db/schema.rs:400,439` | `DELETE FROM employees` runs without first deleting `borrow_requests` / `asset_loans`, which reference employees with `ON DELETE RESTRICT`. Once any borrow history exists the reset fails, and each attempt still leaves a `before_reset` snapshot. | Delete loans, claims and requests first in the same transaction, or refuse up front with a clear message. | verified |
| SEC-014 | security | `src/features/auth/useAuthState.ts:433,439`, `src/features/settings/useSettingsState.ts:829,941` | Resetting a user's password, changing the security policy and revoking recovery codes ask for the new password and the Super Admin password through `window.prompt`, which shows them unmasked. | Reuse the existing `RecoveryPasswordModal`. | verified |
| SEC-013 | security | `src/App.tsx:167-171`, `src/features/settings/useSettingsState.ts:76-79,112`, `SettingsView.tsx:1405` | If the session expires while the recovery-codes modal is open, logout does not reset `pendingRecoveryBatch`, `recoveryCodesModalOpen`, `lanToken` or `activeView`. The next admin who logs in lands on Settings with the 8 plaintext codes on screen. | Add a settings reset called from `resetAppStateOnLogout`; gate the modal on `auth.isSuperAdminAccount`. | traced |
| SEC-017 | security | `src-tauri/src/db/auth.rs:861-954`, `recovery_ledger/reconciler.rs:71` | The recovery gate checks ledger integrity only, not ledger-vs-DB agreement. Restoring a snapshot taken before a code rotation revives burned or revoked codes and invalidates the newly printed ones. | In the gate, require the account's `batch_uuid` to match its last `BatchActivated` block and the code not to appear in a `CodeBurned` block. | traced |
| SEC-018 | security | `src-tauri/src/db/auth.rs:758,1040,1126`, `auth_session.rs:411` | `token_epoch` is incremented on password reset but never read. Sessions live in process memory, so with a shared DB a reset, demotion or deletion done on PC A leaves that account's session on PC B valid with its old role for up to 12 hours. | Store the epoch in `SessionEntry` and compare it (and the role) in the admin and super_admin guards. | traced |
| REL-006 | bug | `src-tauri/src/db/backup.rs:346-420`, `recovery_ledger/verification_gate.rs:50` | A COPY-mode `move_database_to` does not copy `ledger.rev` / `recovery.journal`. The ledger is looked up beside the new DB, so every recovery-code reset fails with "ledger is missing" until codes are regenerated. | Copy both files with the DB. | traced |
| REL-007 | bug | `src-tauri/src/db/backup.rs:115-135`, `recovery_ledger/activation.rs:263,313`, `verification_gate.rs:133-166` | The USB ledger mirror is synced only on ledger events and errors are dropped. Enabling USB backup after codes exist, or activating/revoking with the USB unplugged, leaves the mirror missing or behind, and the gate stays Disabled even with the USB inserted. | Call `sync_to_usb_mirror` in `configure_usb_backup_destination` and in each USB backup sync. | traced |
| BUG-30 | bug | `src-tauri/src/db/auth.rs:1052-1073` | If `ledger.append_event` fails after `tx.commit()`, the user sees an error although the password already changed and the code is burned; old sessions are not invalidated. A torn append also leaves the ledger permanently unverifiable. | Invalidate sessions and unlock right after commit; treat a ledger failure as a warning that the startup reconciler completes. | traced |
| BUG-31 | bug + missing test | `src-tauri/src/db/borrow.rs:1316-1333`, `db/schema.rs:439` | A LAN borrow with an unknown staff ID is accepted with a NULL `employee_id_fk`, but approval inserts that NULL into `asset_loans.employee_id_fk NOT NULL`. The request can only be rejected and holds the asset claim meanwhile. Tests cover manual returns only. | Block approval with a clear "link an employee first" error; add the test. | traced |
| BUG-25 | bug | `src/features/auth/LoginPage.tsx:137,225-230`, `useAuthState.ts:259-289` | After a failed login, forgot-password errors go to `globalError`, which only renders when `loginError` is empty. The user keeps seeing "Username or password is incorrect" instead of "wrong recovery code". | Call `auth.clearLoginError()` in the "Forgot password" and "Back to login" handlers. | traced |
| BUG-26 | bug | `src/features/borrow/useBorrowState.ts:61-81,165-191` | Clicking request A then B quickly can let A's detail resolve last. The queue highlights B while Approve/Reject act on A (`selectedRequest.id`). | In `loadRequestDetail`, drop the result when `selectedRequestIdRef.current !== requestId`. | plausible |

### 1.3 Low

| ID | Type | Location | Problem (concrete case) | Smallest fix | Checked |
|---|---|---|---|---|---|
| BUG-34 | bug | `src-tauri/src/db/backup.rs:2046-2060` | A stray file such as `snap_a_b_c.sqlite3` in the history folder makes `&date[..4]` panic in snapshot listing and in pruning after every snapshot. | Fall back to the raw name unless the parts are 8 and 6 ASCII digits. | verified |
| BUG-33 | bug | `src-tauri/src/db/borrow.rs:689-696` | Return requests skip the pending-claim check and the page creates a new `requestId` per tap. A retry after a lost response queues a second return that can never be approved. | Reject a return when a pending return item already exists for the asset. | traced |
| BUG-29 | bug | `src/features/settings/AuditLogViewer.tsx:37-58` | Switching category tabs quickly lets a slower earlier response overwrite the list under the newer tab. | Add a `cancelled` flag in the effect cleanup. | plausible |
| PERF-22 | scale | `src/features/employees/EmployeeView.tsx:658-829`, `useEmployeeState.ts:43` | With 500 rows per page every search or cell-edit keystroke re-renders about 500 × visible-columns unmemoized cells, because `searchTerm` and drafts live in App-level hooks. Not measured. | Extract a `React.memo` row component; highlight with the debounced term. | plausible |
| PERF-23 | scale | `src-tauri/src/lan_server.rs:571-830` | LAN handlers do blocking DB work on the async runtime. A burst of submits during a long desktop write parks one runtime worker per request for up to the 5 s busy timeout, stalling async commands and the backup timer. | Wrap handler DB work in `spawn_blocking`. | plausible |
| MAINT-007 | lean | `package.json`, `src/data/staff-data.ts`, `src/services/staff-api.ts`, `useSettingsState.ts`, `vite.config.ts:16` | Unused dependencies: `class-variance-authority`, `clsx`, `tailwind-merge`, `zod` (no import in `src/`, `scripts/` or root configs). Dead file `src/data/staff-data.ts` (134 lines, no importer). Hook code never wired to UI: `handleRevokeRecoveryCodes`, `assetSeedText` / `seededAssets` / `handleSeedAssets`. `staffApi` methods with no production caller: `probeLanServer`, `createEmployee`, and the asset-import batch family. `emptyOutDir: false` leaves stale chunks in `dist/` (a 577 KB chunk from October is still there). | Delete them; set `emptyOutDir: true` unless something depends on it. | verified (deps, dead file); traced (rest) |
| MAINT-008 | lean | `src-tauri/Cargo.toml`, `db/mssql_import.rs:1159`, `db/audit.rs:201` | `futures` is unused (cargo warns); `tower` is used only in tests. `validate_remote_settings` and `prune_audit_logs_older_than` are called only from tests, so `audit_logs` is never pruned and grows without bound. Cargo reports about 21 unused functions in total; most were not checked. | Drop `futures`, move `tower` to dev-dependencies, wire or delete the two functions. | traced |

---

## 2. Carried over, still open

Line numbers are current as of `d1d91ac`.

### 2.1 Performance

| ID | Severity | Location | What remains |
|---|---|---|---|
| PERF-20 | Medium | `src-tauri/src/lib.rs:1722-1737`, `db/backup.rs:521` | App close still runs a synchronous `VACUUM INTO` snapshot plus `block_on(server.stop())`, so close time grows with DB size. |
| PERF-19 | Medium | `src/features/employees/employeeListExport.ts:56-57`, `lib.rs:104-109` | Export bytes still cross IPC as a JSON number array (`Array.from(Uint8Array)` → `Vec<u8>`). |
| PERF-02 | Medium | `src-tauri/src/db/asset.rs:1496` | Backend query still has no LIMIT/OFFSET; commit `b0ec365` only paginated the frontend. |
| PERF-03 | Low | `src-tauri/src/db/import.rs:362-374` | Per-row employee lookups during import (N+1). |
| PERF-01 | Low | `src-tauri/src/db/asset_import.rs:870-921` | One UPDATE per inline cell edit. |
| PERF-04 | Low | `src-tauri/src/db/asset_import.rs:3309` | Batch detail loads all rows. |
| PERF-06 | Low | `src-tauri/src/db/employee.rs:924,936` | FTS match is still OR'd with several `LIKE '%q%'` scans. |
| PERF-07 | Low | `src-tauri/src/db/employee.rs:996-1006` | COUNT then SELECT without a read transaction. |
| PERF-05 | Low | `src-tauri/src/db/asset.rs:681,701,1629` | Deferred transactions on asset status/loan writes; whether a real race remains was not traced. |

### 2.2 Bugs

| ID | Severity | Location | What remains |
|---|---|---|---|
| BUG-003 | Medium | `src/features/employees/useTableEdit.ts:244-245` | `if (!employee) continue`, then drafts are cleared: edits for rows no longer loaded are dropped silently. |
| BUG-007 | Medium | `src/features/assets/useAssetDashboardState.ts:60-68,96-106` | No request id or disposed guard on dashboard reload. |
| BUG-008 | Low | `src/features/employees/EmployeeView.tsx:195-197` | Selection is not cleared when `staffGroupFilter` changes. |
| BUG-006 | Low | `src/features/employees/EmployeeView.tsx:236,239,278-279` | `handleCheckDuplicates` still swallows errors. |
| BUG-005 | Low | `src-tauri/src/db/import.rs:~807-818` | `preview_import_excel` turns DB lookup errors into "new row". |
| BUG-009 | Low | `src-tauri/src/db/import.rs:1661-1670` | Ambiguous dates are parsed as dd/mm first with no warning or locale setting. |

### 2.3 Security

| ID | Severity | Location | What remains |
|---|---|---|---|
| SEC-012 | Low | `src-tauri/src/lib.rs:407`, `db/auth.rs:552` | `list_login_account_hints` returns all usernames before login; no setting to hide them. |
| SEC-008 | Low | `src-tauri/src/db/asset_import.rs:898-921` | UPDATE is still built with `format!` on a column name. Safe today (the name comes from a literal `match`); `TotalSpec.md` marked this closed, which is not accurate. |

### 2.4 Architecture, maintenance, DevOps

| ID | Severity | Location | What remains |
|---|---|---|---|
| ARCH-001 | Medium | Rust: `asset_import.rs` 5856, `backup.rs` 3770, `borrow.rs` 3677, `asset.rs` 3560, `mssql_import.rs` 3399, `auth.rs` 3212 lines. Frontend: `AssetDashboard.tsx` 1950, `SettingsView.tsx` 1411, `EmployeeView.tsx` 1217, `useSettingsState.ts` 1093. | No splits; Rust tests still inline. |
| ARCH-002 | Medium | `src/App.tsx` (618 lines, 11 feature hooks at `:84-232`) | No providers/context; props drilled at `:519-555`. |
| ARCH-003 | Low | `src-tauri/src/lib.rs` (2325 lines, 109 `require_*` calls) | No `commands/` module or guard helper; larger than when first reported. |
| MAINT-006 | Low | `asset_import.rs:1382,1413`, `auth.rs:1497`, `backup.rs:1573-1662`, `mod.rs:145,356,370` | `eprintln!` in production paths; no `log`/`tracing`. |
| MAINT-005 / DEP-001 | Low | `package.json` | `react-router-dom` still unused; `caniuse-lite` still in `dependencies`. Remove together with MAINT-007. |
| MAINT-004 | Low | `src/App.tsx:479-480` | `window.__noteTimer`; use `useRef`. |
| DEP-002 | Low | `.github/workflows/staffkit-ci.yml` | No `cargo audit` / `npm audit` step. |
| HYGIENE-001 / DEP-003 | Low | repo root | `__tmp_trim.pdb` (1.2 MB) still tracked; no `.gitignore` rule. |

---

## 3. Carried over, partially fixed

| ID | Severity | Done | What remains |
|---|---|---|---|
| SEC-003 | High | Default account is created with `force_password_reset = 1`; the default recovery code is rejected (`auth.rs:845-856`). | The SQLCipher key and default password are still compiled in (`db/schema.rs:20-28`); `adman` is still forced to `super_admin` (`auth.rs:1839-1844`). No per-machine key or rekey. |
| REL-003 | High | LINK mode adds integrity check, restore lock, cache invalidation and audit (`backup.rs:313-345`). | No network-path detection for the DB location; WAL is unconditional (`mod.rs:426`); migrations do not run on link. Related: REL-005. |
| SEC-004-path | Medium | Audit and mandatory `before_restore` snapshot (`backup.rs:256-311, 989-991`). | No path confinement: restore checks the extension only (`:982`), move accepts any folder (`:238`). |
| SEC-009 | Medium | `validate_export_path` whitelists extensions, rejects `..`, blocks system and DB folders, audits failures. | No dialog-issued path; the system-folder check is substring matching. See SEC-015 for the concrete hole. |
| REL-002 | Medium | `SCHEMA_USER_VERSION = 1` gate (`mod.rs:521-528`) with a lifecycle test. | One all-or-nothing gate, no sequential per-version steps; `ensure_recovery_codes_table` still runs on every open (`:554`). |
| SEC-011 | Low | Minimum length 8 (Standard) / 12 (Strict) (`auth_session.rs:47-52`). | Standard is below the ≥10 target; no check found that blocks reusing the default password. |
| MAINT-003 | Low | Data migration for `internal_movent` (`mod.rs:1487`). | The legacy CASE remains in `employee.rs:360,380,961,967,1453` and `schema.rs:155`. |
| PERF-16 | Low | 8 lazy views; dynamic `write-excel-file`; `manualChunks`; `AssetDashboard` lazy inside Settings (`perf/startup-polish`: SettingsView chunk 158 KB → 67 KB, AssetDashboard 95 KB own chunk). | `qrcode.react` is a static import. |
| BUG-004 | Low | Highlight uses NFD stripping. | Offsets are correct for NFC text only (`EmployeeView.tsx:34-66`, `AssetDashboard.tsx:924-946`). |
| TEST-002 | Medium | 34 test files use RTL / `renderHook`. | 16 tests still assert on source text (`readFileSync` / `?raw`). Count is from grep, not from reading each test. |
| TEST-005 | Medium | Search debounce and stale-race tests exist. | No `useTableEdit`, `HighlightText`/Unicode or dashboard reload-race tests. |

---

## 4. Closed since the previous revisions

Closed on `main` after reading the implementing code (not from commit messages):

| ID | Evidence |
|---|---|
| SEC-015 | `lib.rs` `validate_export_path` now blocks both the DB folder and `app_local_data_dir`, canonicalizes the parent folder of a not-yet-existing file, and rejects the file name `db_settings.json` anywhere. Tests `validate_export_path_blocks_app_data_dir_for_new_files`, `..._blocks_real_app_file_names_anywhere`. |
| SEC-002 | `lan_server.rs:483` bearer check, `:210` connect-info, `:573-743` peer rate limits, replay check. Still binds `0.0.0.0` by design. |
| SEC-006 | 14 audit calls in `db/auth.rs`, 13 in `backup.rs`, reset at `mod.rs:221`. |
| SEC-007 | `mssql_import.rs:1575` `EncryptionLevel::Required`; no `TrustServerCertificate`. |
| SEC-010 | Lockout in `db/auth.rs:434,833`, `auth_session.rs:560,598`. In-memory only (resets on restart). |
| REL-001 | Staged restore with rollback (`backup.rs:664,1009-1016`). |
| REL-004 | Per-category snapshot quotas (`backup.rs:1965-2023`). |
| BUG-20 | Auto-backup scheduler (`lib.rs:1574-1585`, `backup.rs:1561`). |
| BUG-001 / TEST-006 | NFC normalization at import, search and header keys; Rust test `mod.rs:2031`. |
| BUG-002 / PERF-18 | 250 ms debounce and request-id guard (`useEmployeeState.ts:59-68,128,146`). The backend FTS half is still open as PERF-06. |
| BUG-21 | Hook dependencies fixed (`useSettingsState.ts:297`). |
| PERF-10 | 97 `#[tauri::command(async)]`; only `get_app_version`, `ping`, `show_main_window` are sync. |
| PERF-11 | Connection pool and cached DB path (`db/mod.rs:71-73,255,380`). |
| PERF-12 / PERF-13 | `user_version` gate; single connection in `init_database`. |
| PERF-14 | Window shown once after first paint (`src/lib/windowReady.ts`, `main.tsx:27`). |
| PERF-21 | `[profile.release]` in `Cargo.toml:51-54`. |
| PERF-15 | Font is the Windows system font (`Segoe UI Variable Text`, `Segoe UI`) instead of an unbundled Inter; per-user theme applied in `useLayoutEffect` before paint (`perf/startup-polish`). |
| PERF-17 | `useTeamState` gated by `enabled: activeView === "teams"`, with a test (`perf/startup-polish`). Login call count not re-measured. |
| DEVOPS-001 | `.github/workflows/staffkit-ci.yml`. |
| SEC-016 | Fixed in PR #51 — `change_local_account_password` only for the caller's own account, wrong current password counts toward lockout. |
| SEC-019 | Fixed in PR #51 — `inspect_asset_import_file` requires admin; guard/classification test added in `lib.rs`. |
| BUG-22 | Fixed in PR #51 — `moveEmployeesGroup` forwards `action`; UI sends `set_floating`. |
| BUG-23 | Fixed in PR #51 — select-all query keeps `isFloating`. |
| REL-005 | Fixed in PR #53 — unreachable custom DB folder returns an error; failed init no longer cached (`init_once`). |
| BUG-28 | Fixed in PR #54 — Select All and MSSQL preview refresh failures shown to the user. |

Closed in earlier revisions and unchanged: SEC-001, SEC-004 (authorization), SEC-005, BUG-010, MAINT-002.

---

## 5. Ordered roadmap

Each row is one small branch/PR.

| Order | Target | Why first | Effort |
|---|---|---|---|
| ~~1~~ | ~~SEC-016, SEC-019~~ (done, #51) | Guard fixes of a few lines each; SEC-016 bypasses login lockout. | S |
| ~~2~~ | ~~BUG-22, BUG-23, BUG-28~~ (done, #51, #54) | Floating actions are broken today and "select all" can move the wrong employees. | S |
| ~~3~~ | ~~REL-005~~ (done, #53) | Silent data split when the shared folder is offline. | S |
| ~~4~~ | ~~SEC-015~~ (rest of SEC-009 / SEC-004-path still open) | Export can redirect the app to another DB. | M |
| 5 | BUG-24, BUG-27, BUG-25, BUG-34 | Small, independent data-loss/crash fixes. | S |
| 6 | BUG-32, BUG-31, BUG-33 | Borrow and reset flows that currently dead-end. | S–M |
| 7 | SEC-013, SEC-014 | Recovery codes and passwords shown on screen. | S |
| 8 | SEC-017, SEC-018, REL-006, REL-007, BUG-30 | Recovery-ledger consistency; do together, with tests. | M |
| 9 | BUG-003, BUG-007, BUG-008, BUG-26, BUG-29 + TEST-005 | Frontend races and silent drops, with behaviour tests. | M |
| 10 | PERF-20, PERF-19, PERF-02 | Remaining user-visible slowness. | M |
| 11 | MAINT-007, MAINT-008, MAINT-005, HYGIENE-001, DEP-002 | Deletions and CI audit step. | S |
| 12 | REL-002, REL-003, MAINT-003 | Versioned migration steps, network-path handling. | L |
| 13 | SEC-003 | Per-machine key and rekey; highest risk, do last and alone. | L |
| 14 | ARCH-001/002/003, MAINT-006, TEST-002 | Move-only refactors, one file per PR. | M each |

---

## 6. Not checked in this revision

- Nothing was reproduced at runtime; no finding has a failing test yet.
- Backend files not read by the new-issue audit: `asset.rs`, `asset_import.rs`, `employee.rs`, `import.rs`, `mssql_import.rs`, `historical_assignment_import.rs`, `employee_asset_seed.rs`, `asset_metadata_reimport.rs`, `import_detector.rs`, `team.rs`, `column.rs`, `storage_safety.rs`, `audit_journal.rs` internals, ledger `block.rs` / `key_derivation.rs`, `lan_assets.rs`, `backup.rs:1227-1400`, the bodies of `lib.rs:700-1530`, and all test modules.
- Frontend files not read by the new-issue audit: `AssetDashboard.tsx` and the rest of `src/features/assets/` except `useAssetDashboardState.ts`, `src/features/import/`, `src/features/columns/`, `TeamView.tsx`, `useTeamState.ts`, most modals, `src/lib/`, and all test files.
- `web/` (AssetDesk-Pro) is out of scope.
- SEC-002: that the stored `submit_source_ip` comes from the peer address rather than the request body was not traced.

*`optimise.md` and `TotalSpec.md` are kept unchanged as the audit and design trail.*
