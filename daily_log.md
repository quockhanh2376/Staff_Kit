# Daily Log - 2026-03-30

## Objective
Rename the remaining wizard batch-section header so it matches the new staged-import vocabulary without widening the current copy cleanup scope.

## Work Completed
- Added a tiny shared copy helper for the wizard batch list title.
- Renamed the wizard section header from `Existing Staged Batches` to `Staged Import Batches`.
- Kept the slice intentionally narrow:
  - no behavior changes
  - no state changes
  - no batch summary logic changes

## Validation
- Added the failing assertion first in `scripts/asset-import-copy.test.ts`
- Verified the red step by running `node --experimental-strip-types scripts/asset-import-copy.test.ts` before the new export existed
- Re-ran `node --experimental-strip-types scripts/asset-import-copy.test.ts`
- Ran `npm run check:quality`
- Result: passed
  - copy helper script test: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Git History Added
- `a928dc8` - `style: align import batch section title`

## Current State
The staged-batch section now uses the same import-focused vocabulary across its title, empty state, and Settings summary references. The asset-import wording cluster is more internally consistent without touching behavior.

## Next Suggested Focus
- Decide whether `Active Import Batch` should stay as-is or be renamed to `Current Staged Import Batch` for full vocabulary alignment.
- Decide whether asset-import helper strings should remain centralized in one file or be split into settings/wizard groups once behavior work resumes.

# Daily Log - 2026-03-30

## Objective
Align the remaining asset-import batch empty-state wording so Settings and the wizard describe staged batches with the same vocabulary.

## Work Completed
- Added shared copy helpers for:
  - staged import batch empty state
  - staged import batch count summary
- Updated the Settings import card summary rail to use the shared helper for both the zero-state and the review-count message.
- Updated the wizard `Existing Staged Batches` empty state to match the same `staged import batches` wording.
- Kept scope copy-only:
  - no import behavior changes
  - no batch-state changes
  - no layout changes

## Validation
- Added the failing assertions first in `scripts/asset-import-copy.test.ts`
- Verified the red step by running `node --experimental-strip-types scripts/asset-import-copy.test.ts` before the new exports existed
- Re-ran `node --experimental-strip-types scripts/asset-import-copy.test.ts`
- Ran `npm run check:quality`
- Result: passed
  - copy helper script test: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Git History Added
- `6ef1639` - `style: align import batch empty states`

## Current State
The batch-summary copy now reads consistently across both Settings and the wizard. IT sees the same `staged import batches` wording whether there are zero batches or a small review queue waiting.

## Next Suggested Focus
- Decide whether `Existing Staged Batches` should also be renamed to a more neutral label like `Staged Import Batches`.
- Decide whether the `Active Import Batch` summary card should move into the shared copy helper too, or stay inline until its behavior changes.

# Daily Log - 2026-03-30

## Objective
Finish the last two hardcoded Settings import strings so the import copy cluster is fully routed through the shared helper before the next demo pass.

## Work Completed
- Added two Settings-specific asset import copy helpers:
  - entry description for the staged import flow
  - secondary manual action label for opening the serialized add drawer
- Updated the Settings import card to use the shared helper instead of local hardcoded strings.
- Kept the scope intentionally tiny:
  - no behavior changes
  - no state changes
  - no import pipeline changes

## Validation
- Added the failing assertions first in `scripts/asset-import-copy.test.ts`
- Verified the red step by running `node --experimental-strip-types scripts/asset-import-copy.test.ts` before the new exports existed
- Re-ran `node --experimental-strip-types scripts/asset-import-copy.test.ts`
- Ran `npm run check:quality`
- Result: passed
  - copy helper script test: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Git History Added
- `30bf402` - `style: extract settings import copy helpers`

## Current State
The remaining Settings import copy now comes from the same helper module as the rest of the serialized manual-path wording. The asset import entry card is easier to maintain because its CTA, body copy, and secondary manual action label all live behind one shared copy surface.

## Next Suggested Focus
- Show a fresh demo pass of the current Settings/import UI after the copy cleanup.
- Decide whether any remaining asset-import vocabulary in non-primary flows should move into the shared helper too, or stay local until behavior changes again.

# Daily Log - 2026-03-30

## Objective
Clean up the remaining serialized-only manual-path copy and make the Settings entry CTA more mode-neutral without changing any import behavior.

## Work Completed
- Added a focused asset-import copy helper for labels and user-facing manual serialized messages.
- Updated the manual drawer title from a generic manual-add label to `Add Serialized Asset`.
- Updated the serialized mode card description to say `One serialized asset per row`.
- Updated the manual panel title/body/button copy:
  - `Quick Serialized Add`
  - serialized-only fallback wording for one-off serialized assets
  - `Create Serialized Asset`
- Updated the manual serialized validation and success messages in state:
  - required fields now use UI labels `asset code, category, and asset name`
  - success message now describes the result as a borrow-ready serialized asset instead of a raw table insert
- Changed the Settings entry CTA from `Import Assets` to the more neutral `Open Import Wizard`.

## Validation
- Wrote the new helper rail first in `scripts/asset-import-copy.test.ts`
- Ran `node --experimental-strip-types scripts/asset-import-copy.test.ts`
- Ran `npm run check:quality`
- Result: passed
  - copy helper script test: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Git History Added
- `5588869` - `style: clean serialized import copy`

## Current State
The import branch is now more consistent about the serialized manual path. The drawer title, CTA labels, mode description, and manual add messages all point to the same mental model: this path creates a serialized borrow-ready asset, while the main import wizard still handles both quantity and serialized flows.

## Next Suggested Focus
- Decide whether `Serialized Assets` in mode badges should also become `Serialized (Borrow-Ready)` for complete consistency.
- Decide whether hidden seed-panel copy should be updated too, or left alone until that panel is either removed or restored.

# Daily Log - 2026-03-30

## Objective
Align wizard and Settings wording with the current import model so `quantity` clearly means stock-only updates and `serialized` clearly means borrow-ready asset records.

## Work Completed
- Tightened the asset import message helper contract:
  - quantity success message now says rows were committed into stock records only
  - serialized success message now says rows were committed into borrow-ready assets
  - serialized delete warning now keeps the same borrow-ready wording
  - added a dedicated CTA label helper for mode-aware import buttons
- Updated the Review Batch primary action button to use mode-aware wording:
  - `Import Stock Rows`
  - `Import Serialized Assets`
- Updated Settings copy around the import entry point:
  - clarified the difference between quantity and serialized outcomes
  - renamed the manual action button to `Add Serialized Asset`
  - renamed the batch summary card to `Active Import Batch`
  - surfaced the active batch mode in the summary line

## Validation
- Wrote the message-contract assertions first in `scripts/asset-import-messages.test.ts`
- Ran `node --experimental-strip-types scripts/asset-import-messages.test.ts`
- Ran `npm run check:quality`
- Result: passed
  - message helper script test: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Git History Added
- `24bc723` - `style: align import outcome wording`

## Current State
The mainline import branch now speaks more clearly at the point of action. IT can see, from both Settings and the wizard CTA/success copy, whether a staged batch is headed into stock records or into borrow-ready serialized assets.

## Next Suggested Focus
- Decide whether Settings should also show mode-aware copy in the staged-batch count/empty state for even quicker scanning.
- Decide whether manual serialized add should surface category metadata like prefix or QR expectations once a category is selected.

# Daily Log - 2026-03-30

## Objective
Tighten the asset import wizard around the seeded category master so manual add and review edits stop relying on raw free-text category input when an active category already exists.

## Work Completed
- Added a focused category-option helper for the import wizard:
  - filter category choices by `trackingMode`
  - include only active category master records
  - canonicalize current values by matching either `categoryCode` or `categoryName`
  - preserve legacy current values when they do not match an active category
- Added a reusable category input component for asset import surfaces.
- Updated the review grid so the `Category` column uses category-master choices instead of a generic text input when category data is available.
- Updated the manual asset panel so serialized manual add also uses the same category-master-driven input.

## Validation
- Wrote the new red/green rail first in `scripts/asset-import-category-options.test.ts`
- Ran `node --experimental-strip-types scripts/asset-import-category-options.test.ts`
- Ran `npm run check:quality`
- Result: passed
  - category-option script test: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Git History Added
- `35d0c6e` - `feat: use category master in asset import inputs`

## Current State
The mainline quantity-import branch now reads from the seeded category master in the wizard UI instead of leaving category edits fully free-form. Existing legacy values are still preserved safely, but the normal path now nudges IT toward valid active categories for the selected import mode.

## Next Suggested Focus
- Align import success and Settings summary copy so `quantity` clearly means stock-only updates while `serialized` remains the borrow-ready asset path.
- Decide whether the manual serialized add path should also surface category metadata like prefix or QR expectations once a category is chosen.

# Daily Log - 2026-03-23

## Objective
Stabilize the live demo path on `codex/asset-import-wizard`, sync the latest serialized-import hardening work, and keep project tracking aligned across git, Beads, and NotebookLM before moving on to quantity commit work.

## Work Completed
- Fixed a real startup blocker for the live demo on legacy local databases:
  - the app crashed before launch because `BASE_SCHEMA_SQL` tried to create `idx_assets_category_id` before old `assets` tables had a `category_id` column
  - moved the category index responsibility to the post-upgrade migration path so legacy DBs can add the column first and only then create the index
- Verified the live desktop demo actually runs after the migration fix:
  - Tauri app opened successfully as `Staff Kit`
  - LAN borrow server listened on port `8787`
  - mobile borrow page served correctly at `/borrow`
- Updated the mobile borrow submit button styling:
  - changed from yellow to green to match the app action-button family
  - added a slightly darker green hover state
  - kept dark text for readability
- Hardened serialized import validation in the asset import wizard backend:
  - normalize `serialNumber` to uppercase during staging
  - normalize `serialNumber` to uppercase during inline edit
  - reject duplicate `serialNumber` values inside the same serialized batch
  - reject duplicate `serialNumber` values against existing assets
  - made the duplicate check case-insensitive against legacy lower/mixed-case stored serial data

## Validation
- Wrote failing Rust tests first for:
  - legacy asset-schema upgrade before category index creation
  - green borrow submit button theme
  - duplicate serialized `serialNumber` inside the same batch
  - duplicate serialized `serialNumber` against existing assets
  - uppercase normalization of `serialNumber` during inline edit
- Re-ran targeted tests after each fix and confirmed they passed.
- Ran `npm run check:quality`
- Ran `npm run test:tauri`
- Result: passed
  - Frontend lint/build + Tauri `cargo check`: passed
  - Rust tests: `34 passed, 0 failed`

## Git History Added
- `8a77755` - `fix: migrate legacy asset schema before category index`
- `578cb11` - `style: switch borrow submit button to green`
- `8fd10f9` - `fix: validate serialized import serial numbers`

## Current State
The branch is now demo-safe for the current desktop + LAN borrow flow and stronger on the serialized-import path. Legacy local databases upgrade cleanly, the live borrow page matches the app action color language better, and serialized staging no longer lets duplicate/case-variant serials slip through.

## Next Suggested Focus
- Track the next slice in Beads before implementation begins.
- Start `quantity` batch confirm into `stock_items`
- Keep the slice narrow:
  - commit valid quantity rows into `stock_items`
  - preserve staged review/history behavior
  - do not expand into return flow or QR phase-2 work

# Daily Log - 2026-03-22

## Objective
Re-baseline the `codex/asset-import-wizard` branch against the NotebookLM business source of truth, then land the first implementation slice without breaking borrow `2.0.1`.

## Work Completed
- Re-read these NotebookLM sources and compared them against the current branch direction:
  - `business_rules_master_spec_st_adp.md`
  - `Staff_Kit-All-Docs.md`
  - `staff_kit_asset_import_summary.md`
  - canonical `DAILY_LOG.md`
- Wrote an updated execution plan for the current feature branch:
  - `docs/superpowers/plans/2026-03-22-asset-import-rebaseline-execution.md`
- Chose the compatibility boundary for this phase:
  - keep `assets` as the serialized asset store for borrow `2.0.1`
  - add explicit category and quantity-stock structures instead of forcing quantity items into `assets`
- Implemented Chunk 1 of the rebaseline:
  - added `asset_categories`
  - added `stock_items`
  - seeded default category master records with tracking mode and prefix rules
  - exposed `list_asset_categories` through Tauri + TypeScript bridge
  - aligned borrow approval result from `borrowed` to `assigned`
  - kept borrow search behavior limited to serialized `in_stock` assets
- Cleaned up accidental Rust formatting-only noise from unrelated files before commit so the feature diff stayed focused.

## Validation
- Wrote failing Rust tests first for:
  - category/stock schema presence
  - seeded category master rules
  - borrow approval status alignment
- Re-ran those targeted tests after implementation and confirmed they passed.
- Ran `npm run typecheck`
- Ran `npm run check:quality`
- Ran `npm run test:tauri`
- Result: passed
  - TypeScript typecheck: passed
  - Frontend lint/build + Tauri `cargo check`: passed
  - Rust tests: `20 passed, 0 failed`

## Git History Added
- `559abc0` - `docs: add asset import rebaseline execution plan`
- `9e914af` - `feat: rebaseline asset model for import wizard`

## Current State
The `codex/asset-import-wizard` branch now has the first business-aligned asset model rebaseline in place. The branch is clean, pushed, and ready for the next slice: turning the generic import wizard into a mode-aware `quantity` vs `serialized` preview flow.

## Next Suggested Focus
- Start Chunk 2: mode-aware preview flow in `useAssetImportState` and `AssetImportWizard`
- Add import-type choice and mode-specific required columns
- Keep all preview/review data reading from SQLite staging rows only
- Continue deferring return flow, QR comparison, maintenance lifecycle, and bulk QR printing

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

## Next Checklist Locked
- Replace the temporary Settings asset-seed entry path with `Import Assets` and `Add Asset Manually`.
- Add a dedicated `useAssetImportState` hook for file pick, file inspection, batch staging, mapping, review state, inline row actions, and manual add.
- Add the first desktop skeleton of `AssetImportWizard` with:
  - `Choose File`
  - `Map Columns`
  - `Review Batch`
  - existing staged batch list
  - manual add panel
- Keep the first UI slice focused on structure and state wiring, then finish richer row-review UX in the next chunk.

# Daily Log - 2026-03-21

## Objective
Move the Asset Import Wizard checklist from NotebookLM into the actual desktop implementation on `codex/asset-import-wizard`.

## Work Completed
- Wired `useAssetImportState` into the app shell so the asset import flow can open from Settings and share app-level reload/error handling.
- Added `AssetImportWizard` drawer skeleton with:
  - `Choose File`
  - `Map Columns`
  - `Review Batch`
  - staged batch list
  - quick manual add panel
- Added new Settings entry points for:
  - `Import Assets`
  - `Add Asset Manually`
- Added active-batch resume support from Settings so IT can jump back into the current staged batch.
- Hid the old `Asset Seed Utility` path from the Settings screen so the staged import flow is now the primary desktop entry point.

## Validation
- Ran `npm run lint`
- Ran `npm run typecheck`
- Ran `npm run check:quality`
- Result: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Current State
Chunk 3 now has its first usable UI skeleton in place. The next pass should focus on tightening the review grid behavior, visual error feedback, and manual smoke testing with a real CSV or Excel file.

## Preparation Checklist Before Coding Next
- Re-baseline the current branch from a generic staged asset import prototype into a mode-aware import design with `quantity` and `serialized` as separate business flows.
- Add category master requirements into the next implementation slice:
  - `tracking_mode`
  - `prefix_code`
  - `qr_required`
  - `is_active`
- Decide whether the current `assets` table becomes the serialized asset store or remains a temporary compatibility layer for borrow `2.0.1`.
- Align the wizard input model with the NotebookLM templates:
  - quantity: `item_name, category, brand, model, quantity, warehouse, note`
  - serialized: `category, asset_name, brand, model, serial_number, warehouse, note`
- Treat the current staged review flow as the desktop equivalent of `preview`, and treat `Import Valid Rows` as the equivalent of `confirm`.
- Keep the next coding slice focused on P0 only:
  - category mode split
  - serialized asset-code generation
  - mode-aware preview/review flow
  - batch history for IT traceability
  - borrow alignment for `in_stock` assets
- Explicitly defer phase-2 scope:
  - two-QR comparison
  - physical QR verification logs
  - full return and repair lifecycle
  - bulk QR printing
  - advanced duplicate heuristics
- No runtime code changed in this preparation step. This checklist was captured from NotebookLM review so the next coding pass starts from the correct business model.

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
