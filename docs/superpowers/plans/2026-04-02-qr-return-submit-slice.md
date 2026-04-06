# QR Return Submit Slice Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first backend-only QR return slice so employees can submit pending return requests without changing official stock or loan state.

**Architecture:** Reuse the existing borrow request tables and DTOs, add `request_type = "return"` behavior in the Rust backend, and keep the current desktop review UI unchanged. This slice is guarded so the borrow-only approval path cannot accidentally process return requests before a later review slice lands.

**Tech Stack:** Tauri v2, Rust, SQLite (rusqlite), Axum LAN server, React/TypeScript type surface

---

## Chunk 1: Backend Request Model and Service Rules

### Task 1: Add failing Rust service tests for return submit

**Files:**
- Modify: `src-tauri/src/db/borrow.rs`

- [ ] **Step 1: Write the failing tests**

Add tests for:
- valid return submit creates a pending request with `request_type = "return"`
- return submit rejects unknown employees
- return submit rejects duplicate asset codes
- return submit rejects asset codes not actively loaned to the submitted employee
- borrow approve/reject actions reject requests whose `request_type` is not `borrow`
- pending borrow queue excludes hidden return requests
- return submit writes `return_request.submit` audit logging

- [ ] **Step 2: Run targeted tests to verify they fail**

Run: `node ./scripts/run-with-shared-cargo-target.mjs cargo test --manifest-path src-tauri/Cargo.toml borrow -- --nocapture`

Expected: the new return-submit tests fail for missing behavior.

- [ ] **Step 3: Implement the minimal backend code**

In `src-tauri/src/db/borrow.rs`:
- add a return-submit input/flow alongside the existing borrow-submit flow
- resolve active-loan eligibility from `asset_loans` where `returned_at IS NULL`
- insert pending requests with `request_type = "return"`
- add guards so borrow approve/reject actions reject non-borrow request types
- keep `list_pending_borrow_requests` filtered to borrow-only during this hidden slice
- emit `return_request.submit` audit logs

- [ ] **Step 4: Re-run targeted tests**

Run: `node ./scripts/run-with-shared-cargo-target.mjs cargo test --manifest-path src-tauri/Cargo.toml borrow -- --nocapture`

Expected: new and existing borrow tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/borrow.rs
git commit -m "feat: add pending qr return submit flow"
```

## Chunk 2: LAN Endpoint Coverage

### Task 2: Add return-submit LAN endpoint and tests

**Files:**
- Modify: `src-tauri/src/lan_server.rs`
- Test: `src-tauri/src/lan_server.rs`

- [ ] **Step 1: Write the failing LAN tests**

Add tests for:
- valid `/api/return-requests` submit creates a pending return request
- invalid payload or ineligible assets are rejected

- [ ] **Step 2: Run targeted LAN tests to verify they fail**

Run: `node ./scripts/run-with-shared-cargo-target.mjs cargo test --manifest-path src-tauri/Cargo.toml lan_server -- --nocapture`

Expected: the new return endpoint tests fail because the route does not exist yet.

- [ ] **Step 3: Implement the minimal route**

In `src-tauri/src/lan_server.rs`:
- add `POST /api/return-requests`
- parse the same narrow employee submit payload shape
- call the new return-submit backend function

- [ ] **Step 4: Re-run targeted LAN tests**

Run: `node ./scripts/run-with-shared-cargo-target.mjs cargo test --manifest-path src-tauri/Cargo.toml lan_server -- --nocapture`

Expected: return endpoint tests pass and existing borrow endpoint tests stay green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lan_server.rs
git commit -m "feat: add lan qr return submit endpoint"
```

## Chunk 3: TS Contract and Full Verification

### Task 3: Expose typed request contracts and verify the slice

**Files:**
- Modify: docs only if implementation realities changed from the original slice spec

- [ ] **Step 1: Update docs or contracts only if the slice actually needed them**

For this hidden backend-first slice, only change TS contracts if implementation required them. Otherwise, keep this chunk focused on final verification plus doc alignment.

- [ ] **Step 2: Run type/quality verification**

Run: `npm run check:quality`

Expected: quality checks still pass after the backend-only slice.

- [ ] **Step 3: Implement only the minimal follow-up changes still needed**

Examples:
- doc alignment for queue/guard behavior
- minimal TS surface update only if a backend helper was intentionally exposed

- [ ] **Step 4: Run full verification**

Run:
- `npm run check:quality`
- `npm run test:tauri`

Expected:
- frontend lint/typecheck/build pass
- Rust/Tauri tests pass, including new return-submit coverage

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-04-02-qr-return-submit-slice-design.md docs/superpowers/plans/2026-04-02-qr-return-submit-slice.md
git commit -m "docs: align qr return submit plan with implementation"
```
