# Mainline QR Return Cleanup Design

## Goal
Restore `origin/main` to a clean, verifiable baseline after the archived QR-return branches introduced duplicate merge hunks into the shared Borrow/Return flow.

## Current Problem
- `npm run check:quality` fails on `src/types/staff.ts` because `BorrowRequestRecord.requestType` is declared twice.
- `src-tauri/src/db/borrow.rs` contains duplicate field declarations, duplicated local bindings, and a duplicated `load_request_state_tx` signature/body fragment.
- `src-tauri/src/db/schema.rs` declares `borrow_requests.request_type` twice.
- `src-tauri/src/db/mod.rs` contains a malformed `ensure_borrow_request_columns` merge that duplicates the `PRAGMA table_info` setup.

These are merge-artifact failures, not intentional product behavior.

## Scope
- Keep the current product direction on `main`: LAN host detection, Borrow/Return mode on the phone page, assigned-asset search, return submit endpoint, and request-type-aware approval flow.
- Remove only the accidental duplicate hunks and inconsistent pieces that make the branch fail quality checks or risk incorrect runtime behavior.
- Leave follow-up product polishing for later slices.

## Approach
1. Use `6b70aed` as the last known clean mainline baseline.
2. Use the archived QR-return branch changes only as a source for intended return-flow logic.
3. Manually resolve the six changed files so each contains one coherent implementation:
   - `src/types/staff.ts`
   - `src-tauri/src/db/schema.rs`
   - `src-tauri/src/db/mod.rs`
   - `src-tauri/src/db/borrow.rs`
   - `src-tauri/src/lan_server.rs`
   - `daily_log.md` only if needed for consistency
4. Verify with `npm run check:quality`, then run targeted Rust tests around borrow/return request flows.

## Non-Goals
- No new review UX semantics yet.
- No broad refactor of borrow/return modules.
- No dev-reset work in this slice.

## Validation
- `npm run check:quality`
- Targeted Rust tests covering request submission and approval/rejection behavior
