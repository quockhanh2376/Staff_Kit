# Mainline QR Return Cleanup Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore `origin/main` to a green baseline without backing out the merged Borrow/Return product direction.

**Architecture:** Start from the broken `origin/main` worktree, compare against clean mainline `6b70aed` plus the intended QR-return branch logic, and hand-resolve the six affected files. Keep the cleanup narrow: remove duplicate merge artifacts, preserve valid request-type-aware behavior, and verify with the existing quality/test rails.

**Tech Stack:** React, TypeScript, Rust, Tauri, SQLite, ESLint, TypeScript compiler

---

## Chunk 1: Lock Scope And Restore Type/Schema Consistency

### Task 1: Remove obvious duplicate merge artifacts in frontend/shared schema files

**Files:**
- Modify: `src/types/staff.ts`
- Modify: `src-tauri/src/db/schema.rs`
- Test: `npm run check:quality`

- [ ] **Step 1: Confirm the failing frontend typecheck**

Run: `npm run check:quality`
Expected: FAIL on duplicate `requestType` declarations in `src/types/staff.ts`

- [ ] **Step 2: Make the minimal type/schema cleanup**

Implementation:
- Keep exactly one `requestType` field in `BorrowRequestRecord`
- Keep exactly one `request_type` column in the `borrow_requests` schema

- [ ] **Step 3: Re-run frontend quality gate**

Run: `npm run check:quality`
Expected: Frontend moves past the duplicate-type failure, or the next cleanup target becomes visible

## Chunk 2: Restore Coherent Rust Borrow/Return Flow

### Task 2: Clean `mod.rs` migration helper

**Files:**
- Modify: `src-tauri/src/db/mod.rs`
- Test: `cargo check --locked --message-format short`

- [ ] **Step 1: Keep one coherent `ensure_borrow_request_columns` implementation**

Implementation:
- Remove duplicated `PRAGMA table_info` setup
- Preserve the migration that adds `borrow_requests.request_type` when missing
- Preserve the migration test that upgrades legacy `borrow_requests`

- [ ] **Step 2: Re-run Rust compile check**

Run: `cargo check --locked --message-format short`
Expected: Any remaining errors are now isolated to `borrow.rs` or adjacent flow files

### Task 3: Clean `borrow.rs` while preserving intended return behavior

**Files:**
- Modify: `src-tauri/src/db/borrow.rs`
- Test: `cargo check --locked --message-format short`
- Test: `cargo test submit --lib`

- [ ] **Step 1: Remove duplicated merge hunks**

Implementation:
- Keep one `request_type` field in `BorrowRequestRecord`
- Keep one `load_request_state_tx` function
- Keep one set of local bindings in approve/reject flows
- Keep coherent `generate_request_key_tx` behavior

- [ ] **Step 2: Preserve intended Borrow/Return semantics**

Implementation:
- Borrow submit still accepts `requestType` and validates stock by request type
- Return submit endpoint still exists as a distinct helper when needed
- Approval flow remains request-type-aware

- [ ] **Step 3: Re-run compile/test rails**

Run: `cargo check --locked --message-format short`
Expected: PASS

Run: `cargo test borrow --lib`
Expected: Borrow/return request tests pass

## Chunk 3: Final Verification And Follow-Up Handoff

### Task 4: Verify the cleaned branch and record follow-up slices

**Files:**
- Modify: `daily_log.md` (only if consistency update is needed)

- [ ] **Step 1: Run full quality verification**

Run: `npm run check:quality`
Expected: PASS

- [ ] **Step 2: Summarize deferred follow-ups**

Implementation:
- Note that QR Return admin/review semantics stay in the next slice
- Note that dev reset auto-seed stays as third priority
