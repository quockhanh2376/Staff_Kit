# Daily Log - 2026-03-15

## Objective
Refocus `Staff_Kit` as a native desktop-only application and fully separate it from the `AssetDesk-Pro` web migration track.

## Work Completed
- Reviewed the full `Staff_Kit` workspace and related Markdown documentation.
- Confirmed the active app remains a native desktop app built with Tauri, Rust, React, Vite, and SQLite.
- Verified there is no direct runtime dependency from `Staff_Kit` to `E:\AssetDesk-Pro`.
- Removed mixed web-project artifacts from the `Staff_Kit` repo:
  - `web/`
  - `openspec/`
  - `ConvertWEB.md`
  - `docs/business-notes.md`
- Rewrote internal project guidance to match desktop-native direction:
  - `.agent/project-context.md`
  - `.agent/workflows/implement-feature.md`
  - `.agent/workflows/brainstorm.md`
- Added cleanup documentation:
  - `docs/desktop-separation-report.md`
- Pruned internal agent/skill content that was web-focused, Stitch-focused, or unrelated to desktop-native development.
- Updated ESLint config to ignore `.worktrees/**` so quality checks stop scanning unrelated worktree artifacts.

## Validation
- Ran `npm run check:quality`
- Result: passed
  - ESLint: passed
  - TypeScript build/typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Git History Added
- `5c6fbe1` - `chore(repo): remove web migration artifacts from desktop workspace`
- `a6490ec` - `chore(agent): prune non-desktop skills and ignore worktrees`

## Current State
`Staff_Kit` is now cleaner and aligned as an independent desktop-native project, with web migration/spec baggage removed from the main workspace.

## Next Suggested Focus
- Continue feature work only against the desktop stack (`src/` + `src-tauri/`)
- Keep docs and internal workflows desktop-only
- Avoid reintroducing `AssetDesk-Pro` web planning into this repo

# Daily Log - 2026-03-21

## Objective
Stabilize Rust verification for the `codex/asset-import-wizard` worktree so feature work can continue with reproducible `cargo check` and `cargo test` results instead of long native-build timeouts.

## Work Completed
- Re-investigated the earlier `openssl-sys` timeout and confirmed the problem was not a Rust-source deadlock.
- Verified the worktree was compiling into its own isolated `src-tauri/target`, which forced repeated cold builds of the native `sqlcipher` and OpenSSL stack.
- Fixed real compile issues in `src-tauri/src/db/asset_import.rs` that surfaced once the native build completed:
  - generic error formatting for `calamine`
  - transaction borrow lifetime before `tx.commit()`
  - moved-value handling in field mapping resolution
- Replaced the temporary worktree-only Cargo target config with a portable wrapper script:
  - `scripts/run-with-shared-cargo-target.mjs`
- Updated package scripts so Rust verification reuses the shared Cargo target:
  - `check:tauri`
  - `test:tauri`
- Kept `tauri`, `tauri:dev`, and `tauri:build` on their stable existing commands.

## Root Cause Confirmed
- The earlier verification issue came from two combined factors:
  - worktree-local Cargo target directories caused very expensive native rebuilds
  - `asset_import.rs` still had compile errors that were previously hidden by long native build times

## Validation
- Ran `cargo check --manifest-path src-tauri/Cargo.toml`
- Ran `cargo test --manifest-path src-tauri/Cargo.toml`
- Ran `npm run check:quality`
- Ran `npm run test:tauri`
- Result: passed
  - Rust `cargo check`: passed
  - Rust `cargo test`: passed
  - Rust tests: `19 passed, 0 failed`
  - Frontend lint/typecheck/build: passed
  - Tauri quality verification: passed

## Git History Added
- `dbef0fb` - `fix: stabilize rust verification for asset import`
- `853629e` - `build: share cargo target in worktree scripts`

## Current State
The `codex/asset-import-wizard` branch now has stable and repeatable Rust verification, and the branch is ready for the next Asset Import Wizard implementation chunk without carrying unresolved build uncertainty.

# Daily Log - 2026-03-16

## Objective
Ship `Staff_Kit` `2.0.1` as `project ST` with a LAN-only fixed-QR borrow flow, using `project ASP` only as business-flow reference and keeping all code changes isolated to ST.

## Business Scope Locked
- `project ST` = `Staff_Kit`
- `project ASP` = `AssetDesk-Pro`
- ASP was reviewed read-only to capture the receive/approval behavior. No ASP code was modified.
- `2.0.1` scope is limited to:
  - fixed QR that points to the ST machine over LAN
  - mobile employee submit flow
  - desktop IT approval flow
  - stock mutation only after approval
  - active employee asset-loan record creation on approval
- `return flow` is intentionally deferred.

## Work Completed
- Added borrow-related schema and version bump to `2.0.1`.
- Added database services for:
  - asset upsert/seed utility
  - borrow submit, queue, detail, approve, reject
  - audit logs for borrow actions and LAN setting changes
- Added Tauri commands and TypeScript API/types for:
  - borrow LAN settings
  - asset seed utility
  - pending borrow queue
  - request detail
  - approve/reject actions
- Added local LAN HTTP server in Rust with:
  - `/borrow` mobile page
  - `/api/assets` in-stock asset search
  - `/api/borrow-requests` pending request submit
- Added desktop borrow admin UI with:
  - QR preview
  - borrow URL display
  - pending request queue
  - request detail
  - approve/reject controls
- Added Settings support for:
  - LAN host/port
  - asset seed input

## Verification
- Ran targeted Rust DB tests for borrow services.
- Ran targeted Rust LAN server tests.
- Ran `npm run check:quality`
- Result: passed
  - ESLint: passed
  - TypeScript build/typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Release Notes Draft
- Version target: `2.0.1`
- Limitation: LAN-only. Phone and ST machine must be on the same network.
- Limitation: if the LAN port changes, restart ST before testing the new QR URL.
- Approval rule: only IT/admin approval mutates stock and creates active loan records.

## Git History Added
- `7f096fc` - `feat: add borrow schema foundation`
- `1d16707` - `feat: add borrow approval services`
- `7f9612a` - `feat: expose borrow admin commands`
- `a046edd` - `feat: add lan borrow server`
- `b16fdfe` - `feat: add desktop borrow approval ui`
