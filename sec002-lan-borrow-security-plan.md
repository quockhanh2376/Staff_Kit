# SEC-002 Implementation Design — LAN Borrow Server Authentication

> **Scope:** Planning only. Do not modify application source. This document designs a backend-owned LAN authorization mechanism for the axum borrow server.
>
> **Parent audit:** `optimization-backlog.md`, finding **SEC-002 (Critical)**.
> **Baseline:** branch `security/lan-borrow-authentication` @ `2a13d315d835e32eb099de62f963cacb91953432` (== `origin/main`).
> **Threat model:** LAN, guest Wi-Fi, VPN, BYOD, and browsers are untrusted.
>
> **Revision:** 2026-08-05 (final) — reflects approved product decisions, token-lifecycle/design/HTTP-risk corrections, and Phase-C schema/migration/index corrections.

---

## 0. Approved Product Decisions

| Decision | Resolution |
|---|---|
| QR code | One shared QR code for both Borrow and Return. After scan, user chooses mode in the page UI. |
| Token expiry | No time-based expiry. Valid only for the current LAN-server lifecycle. |
| Token persistence | None. Memory-only. Not persisted to DB, config, filesystem, logs, URLs outside the displayed QR, or frontend storage. |
| Token invalidation | App restart, LAN server restart, Stop Server, or Regenerate QR immediately invalidates the old token. |
| Server lifecycle | Must support Start, Stop, and manual Regenerate QR/token. |
| Concurrency | ~5 users now, potentially 10 later. No hard concurrent-user limit. |
| Per-submission identity | Each browser flow and submission uses an independent `clientSessionId`/`requestId`. |
| Assets per request | One borrow/return request may contain one or multiple assets. |
| Staff ID lookup | Authoritative when found: backend fills employee data from DB. |
| Unknown Staff ID | Proceeds with manually entered details; `employee_id_fk` nullable + `manual_entry` flag. |
| Offboarded employees | Cannot borrow. May return assets. |
| Onboarding employees | May borrow. No automatic status change. |
| Borrow flow | Always Pending. Requires IT approval. Optional auto-approve (IT-controlled, default off). |
| Return flow | Same QR. Creates Pending Return. Requires IT confirmation. |
| On-behalf-of return | Another employee may return on behalf of the borrower. Both IDs preserved. |
| IT direct actions | IT may create or complete borrow/return directly. |
| Asset validation | Server-side: existence, eligibility, availability, lifecycle, current borrower, competing claims. |
| Concurrent claims | Transactionally protected via `asset_pending_claims` reservation table + `BEGIN IMMEDIATE`. |
| Trust boundary | Never trust client-supplied employee names, asset state, source IP, role, or approval state. |
| PIN verification | Not required. |

---

## 1. Current Attack Paths

(unchanged — see prior revisions for the 7 confirmed attack paths with exact evidence.)

---

## 2. Proposed Architecture

### 2.1 Token design — opaque, memory-only, no expiry

- **256-bit random** from `OsRng`, stored as `[u8; 32]`.
- **Encoded** as base64url-no-pad (43 chars) for QR embedding.
- **No content:** no identity, role, expiry, claims, or signature inside the token.
- **Verification:** decode to `[u8; 32]`, constant-time XOR comparison against the stored bytes.
- **Storage:** single active token in `LanTokenStore` (`Mutex<Option<[u8; 32]>>`).
- **No persistence:** DB, config, filesystem, logs, frontend storage — none.
- **Lifecycle:** valid until Stop Server / Regenerate / app restart.
- **No HMAC, no `sha2`/`hmac` dependency.**

### 2.2 QR URL format

```
http://<host>:<port>/borrow#t=<token>
```

Uses a **URL fragment** (`#t=`), not a query parameter (`?t=`). Rationale:
- Fragments are NOT sent to the server in HTTP requests — the token never appears in server access logs, proxy logs, or `Referer` headers.
- The `/borrow` page is served as static HTML (no server-side token processing needed for the page itself).
- Browser JS reads the fragment via `location.hash`, stores the token in memory, removes it from the visible URL with `history.replaceState`, and sends it only via `Authorization: Bearer` headers on API calls.

### 2.3–2.10

(unchanged from prior revision — endpoint matrix, peer IP, replay/rate-limit, DTO, errors, audit.)

---

## 3. HTTP Residual Risk Statement

**HTTP is an explicitly accepted residual risk for SEC-002.**
- Bearer token traverses the network in cleartext.
- A captured token remains reusable until Stop/Regenerate/restart.
- Required mitigations: private/domain firewall scope, token redaction from logs/errors, immediate manual regeneration.
- TLS is a separate backlog item. Interception risk is not eliminated.

---

## 4. Borrow/Return Business Rules

### 4.1 Unknown Staff ID — schema approach

**Do NOT create a sentinel/UNKNOWN employee row.** Instead:

- Make `borrow_requests.employee_id_fk` **nullable** (from `NOT NULL`).
- Add columns:
  - `manual_entry INTEGER NOT NULL DEFAULT 0`
  - `manual_employee_id TEXT`
  - `manual_employee_name TEXT`
  - `manual_employee_email TEXT`
  - `manual_employee_team TEXT`
- When Staff ID found: `employee_id_fk` = real row, `manual_entry = 0`.
- When Staff ID not found: `employee_id_fk = NULL`, `manual_entry = 1`, manual fields filled from client input.

**Migration requirement (Phase C):** Changing `employee_id_fk` from `NOT NULL` to nullable requires a **transactional SQLite table-rebuild migration** (SQLite cannot `ALTER COLUMN` in-place). Existing rows must be preserved byte-for-byte in business meaning. This is a non-trivial migration — it must rebuild the table with the new schema, copy all rows, swap, and verify. Do NOT perform this migration in Phase A.

### 4.2 Concurrent asset claims — reservation table

**Do NOT use a partial index with EXISTS/subquery** (invalid SQLite DDL).

Instead, use a dedicated **`asset_pending_claims`** reservation table:

```sql
CREATE TABLE IF NOT EXISTS asset_pending_claims (
  asset_id INTEGER PRIMARY KEY REFERENCES assets(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  borrow_request_id INTEGER NOT NULL UNIQUE REFERENCES borrow_requests(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Behavior:**
- A Pending Borrow atomically inserts claims for **every** asset in the request inside a `BEGIN IMMEDIATE` transaction.
- `asset_id PRIMARY KEY` ensures only one pending claim per asset — a duplicate insert fails with a constraint violation.
- If **any** asset claim insert fails, the **entire** multi-asset request rolls back (transactional atomicity).
- Claims are **released** on reject/cancel (delete rows).
- Claims **transition** correctly on approval (delete rows; create `asset_loans`).
- Claims are released on **timeout/cleanup** if a request is abandoned (future enhancement).

### 4.3–4.5

(unchanged — employee lookup matrix, state machine, IT direct actions.)

---

## 5–10

(unchanged from prior revision — token lifecycle, Rust structures, module boundaries, persistence decision, implementation phases, rollback plan.)

**Phase C updated:** includes the table-rebuild migration for `employee_id_fk` nullable + `asset_pending_claims` table creation.

---

## 11–14

(unchanged from prior revision — automated tests, manual matrix, acceptance criteria, unresolved decisions.)

---

*Historical audit documents `optimise.md` and `zoptimize.md` are preserved unchanged. SEC-001 session/authorization design is not weakened or replaced.*
