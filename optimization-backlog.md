# Staff_Kit Optimization Backlog

- **Created:** 2026-08-05
- **Base:** `origin/main` @ `fc437937e840aefff9c4188f17d4720051a351fa` (v2.1.0 post-SEC-001 merge)
- **Sources reconciled:** `optimise.md` (2026-07-16 audit on `release/v2.0.11`), `zoptimize.md` (2026-08-01 audit on `origin/main` v2.1.0)
- **Status:** SEC-001 complete and merged; this is the single canonical forward-looking backlog

---

## Summary

| Classification | Count |
|---|---|
| **DONE** | 6 |
| **STILL VALID** (active) | 19 |
| **PARTIALLY FIXED** | 2 |
| **OBSOLETE / SUPERSEDED** | 2 |
| **DEFERRED** | 3 |
| **NEEDS RUNTIME VERIFICATION** | 1 |
| **Total reconciled** | 33 |

### Active findings by severity
- **Critical:** 1 (SEC-002)
- **High:** 4 (SEC-003, SEC-004-path-confinement, SEC-007-MSSQL-TLS, BUG-001)
- **Medium:** 9 (BUG-002, BUG-003, BUG-004, BUG-007, PERF-001, PERF-002, PERF-003, MAINT-001, ARCH-001)
- **Low:** 7 (BUG-005, BUG-006, BUG-008, BUG-009, PERF-004, PERF-005, PERF-006, PERF-007, MAINT-003, MAINT-004, MAINT-005, SEC-008, DEP-001, DEP-002, DEP-003)

### Recommended next target
**SEC-002** — Secure the LAN borrow server (token auth, peer IP, dedup, rate-limit). It is the only remaining Critical finding and the highest-risk unaddressed security gap.

---

## DONE

### SEC-001 — Backend Tauri command authorization ✅
- **Supersedes:** optimise.md SEC-001, zoptimize.md SEC-001
- **Resolution:** Squash-merged as `fff32be` on `main`. 75 registered IPC commands classified: 9 public, 22 authenticated, 36 admin, 8 super_admin. Backend-owned in-memory `SessionStore` with 128-bit opaque tokens. `set_active_local_account` and `run_auto_backup_if_due` removed from IPC. `SessionContext` actor attribution for borrow/seed operations. 284 tests.
- **Do not weaken or replace the merged session/authorization design.**

### SEC-004 (authorization) — Restore/move DB commands now super_admin-guarded ✅
- **Supersedes:** zoptimize.md SEC-004 (authorization aspect)
- **Resolution:** `restore_database_from_file`, `restore_history_snapshot`, `move_database_to` all now guarded with `require_super_admin`. Session invalidation (`invalidate_all`) fires post-success.
- **Note:** Path confinement (SEC-004-path) and audit logging (SEC-006) are still open — see below.

### SEC-005 — `update_borrow_lan_settings` now admin-guarded ✅
- **Resolution:** Command wrapper calls `require_admin`; uses `update_borrow_lan_settings_with_actor` with `SessionContext` actor.

### BUG-010 — `write_export_file` now admin-guarded ✅
- **Resolution:** Command wrapper calls `require_admin`. Path confinement is SEC-004-path (still open).

### MAINT-002 / TEST-001 (optimise.md) — Frontend component tests exist ✅
- **Resolution:** Vitest + `@testing-library/react` + jsdom added. 13 test files in `src/`, 130 frontend tests pass. Some behavioral gaps remain (TEST-005).

### optimise.md SEC-004 / TEST-001 — LAN XSS fixed + quality gate expanded ✅
- **Resolution:** LAN borrow page uses `textContent`/`createTextNode` (no `innerHTML`). `check:quality` now runs frontend tests.

---

## STILL VALID (Active)

### SEC-002 — LAN borrow server: unauthenticated, IDOR, spoofable IP, no replay/rate-limit
- **Severity:** Critical
- **Priority:** 1 (next implementation target)
- **Files/symbols:** `src-tauri/src/lan_server.rs:83` (bind `0.0.0.0`), `:110-117` (no auth middleware), `:151-159` (submit endpoint); `src-tauri/src/db/borrow.rs:195-202` (`submit_source_ip` from JSON body)
- **Current evidence:** `grep -c '0.0.0.0' lan_server.rs` = 1; `grep -c 'Authorization' lan_server.rs` = 0; `submit_source_ip` taken from client JSON (15 references in borrow.rs). `axum::serve` does not call `into_make_service_with_connect_info`. No nonce/idempotency/rate-limit.
- **Impact:** Anyone on the LAN can file borrow/return requests in any employee's name, frame any audit IP, and flood the pending queue.
- **Dependencies:** None (SEC-001 session infra can be reused for token generation).
- **Effort:** M | **Risk:** Medium
- **Acceptance criteria:** POST without token → 401; spoofed IP ignored (peer IP authoritative); duplicate within window → 409/429; rate-limited by peer.
- **Tests:** Rust integration: 401 without token, dedup, rate-limit, peer-IP capture. Manual: QR round-trip from a phone.
- **Do not implement SEC-002 LAN token in the same branch as other work.**

### SEC-003 — Shared SQLCipher key + default credentials in source
- **Severity:** High
- **Priority:** 2
- **Files/symbols:** `src-tauri/src/db/schema.rs:20` (`Aswhite2026`), `:21` (`SK-RECOVERY-2026`), `:28` (`SK-AES256-staffkit-2026-...`), `:22` (`Welcome!`); `auth.rs:807-898` (`ensure_default_admin_account`)
- **Current evidence:** All constants verified present on main. `ensure_default_admin_account` force-promotes `adman` to `super_admin` on migration.
- **Impact:** Reverse-engineering one binary or reading public source yields the key for every installation's DB + backups, plus a known super-admin credential.
- **Dependencies:** None.
- **Effort:** M | **Risk:** High (key migration must not brick existing DBs)
- **Acceptance criteria:** No compiled key/credential opens a fresh install's DB; first-run enrollment replaces defaults.

### SEC-004-path — Restore/move DB: no path confinement + no audit log
- **Severity:** High (downgraded from original — guards are in place via SEC-001)
- **Priority:** 3
- **Files/symbols:** `src-tauri/src/db/backup.rs:179-221` (`move_database_to`), `:345-389` (`restore_database_from_file`); `backup.rs` has zero `audit::insert_audit_log_*` calls.
- **Current evidence:** Commands now require `super_admin` (SEC-001) and invalidate sessions (Phase D), but still accept arbitrary absolute paths with no confinement. No audit trail for the most destructive operations.
- **Impact:** A super_admin can write the encrypted DB to an attacker-chosen location or load a tampered DB, with no forensic trail.
- **Dependencies:** None.
- **Effort:** M | **Risk:** Medium
- **Acceptance criteria:** Source/target restricted to app data dir or native-dialog path; audit row written before restore/move; `before_restore` snapshot failure is a hard error.

### SEC-006 — No audit logging on password reset/change, DB restore/move
- **Severity:** Medium
- **Priority:** 4
- **Files/symbols:** `src-tauri/src/db/auth.rs` (0 `audit::insert` calls); `src-tauri/src/db/backup.rs` (0 `audit::insert` calls)
- **Current evidence:** Audit call-sites exist only in `asset_import.rs`, `borrow.rs`, `employee_asset_seed.rs`. Password mutations and DB lifecycle operations leave no audit fingerprint.
- **Impact:** Security-sensitive events cannot be investigated.
- **Dependencies:** None.
- **Effort:** S | **Risk:** Low
- **Acceptance criteria:** Each sensitive op produces an `audit_logs` row with actor + target + timestamp.

### SEC-007 — MSSQL `TrustServerCertificate=yes`
- **Severity:** Medium
- **Priority:** 5
- **Files/symbols:** `src-tauri/src/db/mssql_import.rs:58-62`
- **Current evidence:** `encrypt=true;TrustServerCertificate=yes;` still present.
- **Impact:** TLS active but server identity unverified — MITM-susceptible on untrusted networks.
- **Dependencies:** Production CA confirmation (ops).
- **Effort:** S (code) / M (CA ops) | **Risk:** Medium
- **Acceptance criteria:** MSSQL cert validated in prod; `TrustServerCertificate=no` after CA confirmed.
- **Status:** DEFERRED until production CA is confirmed.

### SEC-008 — `format!`-built SQL in asset-import row update (safe today, fragile)
- **Severity:** Low
- **Priority:** 6
- **Files/symbols:** `src-tauri/src/db/asset_import.rs:882-888`
- **Current evidence:** Column name from `match` of string literals; validated against allow-list. Not exploitable today.
- **Impact:** Latent injection if allow-list/match drift.
- **Effort:** S | **Risk:** Low
- **Acceptance criteria:** No user-influenced text reaches SQL string.

### BUG-001 — Vietnamese NFC/NFD diacritics never normalized
- **Severity:** High
- **Files/symbols:** `src-tauri/src/db/mod.rs:1111-1120`, `import.rs:1288-1294`, `employee.rs:1053-1100`
- **Current evidence:** No `unicode-normalization` crate; normalizers only filter alphanumeric + lowercase.
- **Impact:** Vietnamese headers in NFD silently misclassified; two-laptop merge creates duplicate slots.
- **Effort:** S | **Risk:** Low
- **Acceptance criteria:** NFC/NFD forms compare equal in import + dedup.

### BUG-002 — Employee search no debounce/cancellation
- **Severity:** High
- **Files/symbols:** `src/features/employees/useEmployeeState.ts:95-139`
- **Current evidence:** `grep -c 'debounce\|AbortController' = 0`; `searchTerm` is a direct effect dep.
- **Impact:** Stale results overwrite newer queries after fast typing.
- **Effort:** S | **Risk:** Low
- **Acceptance criteria:** Final rows match final query; no setState after unmount.

### BUG-003 — `useTableEdit` silently skips missing rows on save
- **Severity:** High
- **Files/symbols:** `src/features/employees/useTableEdit.ts:234-235`
- **Current evidence:** `if (!employee) continue` still present.
- **Impact:** Silent edit loss when a reload overwrites `employees`.
- **Effort:** S | **Risk:** Low

### BUG-004 — HighlightText diacritic offset
- **Severity:** Medium
- **Files/symbols:** `src/features/employees/EmployeeView.tsx:39-65`
- **Effort:** S | **Risk:** Low

### BUG-005 — `preview_import_excel` swallows DB errors
- **Severity:** Medium
- **Files/symbols:** `src-tauri/src/db/import.rs:684-700`
- **Effort:** S | **Risk:** Low

### BUG-006 — `handleCheckDuplicates` swallows errors
- **Severity:** Medium
- **Files/symbols:** `src/features/employees/EmployeeView.tsx:199-227`
- **Effort:** S | **Risk:** Low

### BUG-007 — Asset dashboard reload race
- **Severity:** Medium
- **Files/symbols:** `src/features/assets/useAssetDashboardState.ts:40-96`
- **Effort:** S | **Risk:** Low

### BUG-008 — Cross-page selection drift on staffGroupFilter
- **Severity:** Medium
- **Files/symbols:** `src/features/employees/useTableEdit.ts:256-297`; `EmployeeView.tsx:162-164`
- **Effort:** S | **Risk:** Low

### BUG-009 — Date parsing dd/mm vs mm/dd by order
- **Severity:** Low
- **Files/symbols:** `src-tauri/src/db/import.rs:1461-1470`
- **Effort:** S | **Risk:** Low

### PERF-001 — Inline asset-import cell edit UPDATE storm
- **Severity:** Medium
- **Files/symbols:** `src-tauri/src/db/asset_import.rs:2617-2692`
- **Effort:** M | **Risk:** Low

### PERF-002 — Asset dashboard no LIMIT
- **Severity:** Medium
- **Files/symbols:** `src-tauri/src/db/asset.rs:1366-1433` (`list_asset_dashboard_serialized_conn` — no LIMIT/OFFSET)
- **Current evidence:** LIMITs in asset.rs are for single-row lookups (`LIMIT 1`) and search (`LIMIT ?`), not for dashboard pagination.
- **Effort:** S | **Risk:** Low

### PERF-003 — Import N+1 lookups
- **Severity:** Medium
- **Files/symbols:** `src-tauri/src/db/import.rs:294-304`
- **Effort:** S | **Risk:** Low

### PERF-004 — Batch detail loads all rows
- **Severity:** Low
- **Files/symbols:** `src-tauri/src/db/asset_import.rs:2920-2949`
- **Effort:** S | **Risk:** Low

### PERF-005 — Asset status/loan write race
- **Severity:** Low
- **Files/symbols:** `src-tauri/src/db/asset.rs:210-227`
- **Effort:** S | **Risk:** Low

### PERF-006 — LIKE `%q%` defeats indexes
- **Severity:** Low
- **Files/symbols:** `src-tauri/src/db/employee.rs:772-790`
- **Effort:** S | **Risk:** Low

### PERF-007 — Non-atomic COUNT+SELECT
- **Severity:** Low
- **Files/symbols:** `src-tauri/src/db/employee.rs:832-849`
- **Effort:** S | **Risk:** Low

### MAINT-001 / ARCH-001 — Large files
- **Severity:** Medium
- **Files/symbols:** `asset_import.rs` (4,580), `asset.rs` (2,554), `EmployeeView.tsx` (863), `lib.rs` (~950+), `AssetDashboard.tsx` (1,846)
- **Effort:** M/file | **Risk:** Low

### MAINT-003 — `'internal_movent'` typo in SQL
- **Severity:** Low
- **Files/symbols:** `employee.rs:254,808,1223`
- **Effort:** S | **Risk:** Low

### MAINT-004 — `window.__noteTimer`
- **Severity:** Low
- **Files/symbols:** `src/App.tsx:392-393`
- **Effort:** S | **Risk:** Low

### MAINT-005 / DEP-001 — `react-router-dom` unused dependency
- **Severity:** Low (supply-chain)
- **Files/symbols:** `package.json:36`
- **Current evidence:** Still present; 2 high npm advisories (incl. RCE via turbo-stream). Not reachable (no router usage in `src/`).
- **Effort:** S | **Risk:** Low

### DEP-002 — No audit in CI
- **Severity:** Low
- **Effort:** S | **Risk:** Low

### DEP-003 / HYGIENE-001 — `__tmp_trim.pdb` committed
- **Severity:** Low
- **Effort:** S | **Risk:** Low

### REL-001 — DB restore non-atomic
- **Severity:** High
- **Files/symbols:** `src-tauri/src/db/backup.rs:305,328,337`
- **Effort:** L | **Risk:** High

### REL-002 — Unversioned migrations
- **Severity:** Medium
- **Files/symbols:** `src-tauri/src/db/mod.rs:295`
- **Effort:** L | **Risk:** High

### DEVOPS-001 — No CI for Staff Kit
- **Severity:** Medium
- **Effort:** M | **Risk:** Low

### TEST-002 — Frontend tests mostly source-text assertions
- **Severity:** Medium
- **Effort:** M | **Risk:** Low

### TEST-005 — Component-test gaps (search race, table-edit, highlight, dashboard)
- **Severity:** Medium
- **Effort:** M | **Risk:** Low

### TEST-006 — No Unicode normalization tests
- **Severity:** Low
- **Effort:** S | **Risk:** Low

---

## PARTIALLY FIXED

### SEC-007 (optimise.md) / SEC-007 (zoptimize.md) — LAN API error masking
- **Status:** Internal errors no longer exposed in HTTP 500 (generic message); correlation ID/structured logging deferred.
- **Remaining:** Add request-scoped correlation IDs when an observability policy is agreed.

### BUG-008 — Cross-page selection drift
- **Status:** v2.1.0 clears selection on 3 filter changes (`searchTerm`, `teamFilter`, `startDateFilter`) but not `staffGroupFilter`.
- **Remaining:** Clear on `staffGroupFilter` change.

---

## OBSOLETE / SUPERSEDED

### optimise.md SEC-004 (LAN XSS) → Superseded by zoptimize.md confirmation + SEC-001 merge
### optimise.md MAINT-002 / TEST-001 (no component tests) → Obsolete; Vitest+RTL exists with 130 tests

---

## DEFERRED

### SEC-007 (MSSQL TLS) — Deferred until production CA is confirmed
### DEP-001/HYGIENE-001 — Deferred pending owner confirmation that `__tmp_trim.pdb` is not a release artifact
### optimise.md SEC-006 — Deferred (same as SEC-007 MSSQL TLS)

---

## NEEDS RUNTIME VERIFICATION

### optimise.md PERF-001 (same as PERF-002 here) — Asset dashboard at 5k-row scale
- **Status:** No LIMIT confirmed by source audit. Actual UX impact at 5k assets needs runtime profiling. The query returns all rows on every load.
- **Action:** Profile with a 5k-row dataset; add pagination only if the load time exceeds 2s.

---

## Ordered Roadmap

| Phase | Target | Severity | Effort |
|---|---|---|---|
| **1 (next)** | **SEC-002: LAN borrow server auth** | Critical | M |
| 2 | SEC-003: Per-install key + credential rotation | High | M |
| 3 | SEC-004-path: DB path confinement + audit | High | M |
| 4 | SEC-006: Audit logging on password/DB ops | Medium | S |
| 5 | BUG-001: Vietnamese NFC/NFD normalization | High | S |
| 6 | BUG-002/003: Search race + silent edit skip | High | S |
| 7 | PERF-001/002/003: Asset dashboard + import perf | Medium | M |
| 8 | MAINT-001: File splits | Medium | M |
| 9 | MAINT-005/DEP-001: Remove unused `react-router-dom` | Low | S |
| 10 | REL-001/002: DB restore atomicity + migration versioning | High | L |
| 11 | DEVOPS-001: CI pipeline | Medium | M |
| 12 | DEP-002/003: Audit CI + remove `__tmp_trim.pdb` | Low | S |

---

*Historical audit documents `optimise.md` and `zoptimize.md` are preserved unchanged as evidence of the audit trail.*
