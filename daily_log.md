# Daily Log - 2026-10-23

## 2026-10-23 — Staff Kit Verified Historical Asset Assignment Import (Start)
- Authorized by Zon/Cheki.
- Lead / Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  1. Implement dedicated Historical Assignment Import workflow for existing assets without altering existing Asset Import semantics.
  2. Parse Staff ID, Employee Name, and multiple Monitor codes in Tại CTY / Tại NHÀ columns from `Asigned_Asset.xlsx`.
  3. Validate rows and generate read-only Preview categories: New Assignment, Already Assigned, Missing Asset, Missing Employee, Conflict (identity mismatches).
  4. Block conflicting identities (`ASWVN214`, `ASWVN1261`) without guessing. Require canonical matching on Staff ID and Asset Code.
  5. On explicit Approve: atomically create active loans, update asset status to assigned, update usage location safely without overwriting unrelated data.
  6. Idempotent reimport: existing loans preserved, no duplicate loans or automatic returns.
  7. Maintain explicit distinction between inventory import and signature status (`pending_signature` vs `signed`). Zero fabricated signatures or dates.
  8. Safety & Quality: Isolated test DB copy verification, zero writes on Preview/Cancel, full quality gate.

## 2026-10-23 — Staff Kit Verified Historical Asset Assignment Import (End)
- Implementation (Victor):
  * Schema & Migrations: Added `signature_status` ('pending_signature' | 'signed'), `imported_at`, and `handover_date` to `borrow_requests`; added `imported_at` and `handover_date` to `asset_loans`. Idempotent SQLite migrations applied in `src-tauri/src/db/mod.rs` and `schema.rs`.
  * Domain Engine (`src-tauri/src/db/historical_assignment_import.rs`): Dedicated parser and import pipeline for `Asigned_Asset.xlsx`.
    - Parses 157 rows into 200 monitor assignments across "Tại CTY" and "Tại NHÀ" locations.
    - Read-only preview detects New Assignment, Already Assigned, Missing Asset, Missing Employee, and Conflict (intra-file and identity mismatches like ASWVN214 and ASWVN1261).
    - Transactional approval hard-blocks on any conflict; creates approved `borrow_requests` with `signature_status = 'pending_signature'`, `submit_source_ip = NULL`, `handover_date = NULL`, updates asset status to `assigned` and `usage_location`, and inserts `asset_loans`.
    - Genuine signature path `sign_historical_borrow_request_conn` transitions `signature_status` to `'signed'` with stored confirmation.
  * Queries & UI:
    - Updated `get_employee_assigned_assets_conn` and `list_asset_dashboard_serialized_conn` to surface `signature_status`.
    - Registered IPC commands `preview_historical_assignment_import`, `apply_historical_assignment_import`, `sign_historical_borrow_request` in `src-tauri/src/lib.rs`.
    - Updated `AssignedAssetsModal.tsx` with `Pending Signature` / `Signed` status badges.
- QA Validation (Qeen):
  * Tested with Zon's `00_ExSource/Asigned_Asset.xlsx`: 200 monitor assignment items parsed correctly.
  * Detected identity conflicts ASWVN214 (LÊ MINH DUY vs TRẦN NHẬT PHONG) and ASWVN1261 (LÊ TUẤN ANH vs NGUYỄN CHÍ THÀNH).
  * Verified transactional safety: conflicts strictly block import approval with zero partial writes.
  * Verified signature invariants: new assignments start with `pending_signature`, zero fabricated signatures or handover dates (`submit_source_ip` preserved as NULL).
  * Preserved original test DB (`data/test_db/staff_kit.sqlite3`) and production DB.
  * Backend tests: `npm run test:tauri` (364 passed, 0 failed).
  * Frontend tests: `npm run test:unit` (288 passed, 0 failed).
  * Quality gate: `npm run check:quality` (0 errors across lint, typecheck, build, and cargo check).
- Verdict: PASS.

## 2026-10-23 — Staff Kit Tauri Window Not Visible (Start)
- Authorized by Zon/Cheki.
- Lead / Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  1. Investigate why Staff Kit window is not visible after `npm run tauri:dev` reaches `Running target\debug\staff-kit.exe`.
  2. Root cause analysis: inspect window station and thread desktop inheritance, `visible: false` in `tauri.conf.json`, WebView2 initialization and window show timing.
  3. Implement fix:
     - Add robust window show, unminimize, and focus fallback retry loop in Tauri backend.
     - Restore `show_main_window` public Tauri command and hook into React frontend on mount and theme ready.
  4. Verify actual visible, interactive window on user's foreground Windows desktop session.
  5. Protect test/production DB. Focused TDD + full quality gate (`npm run check:quality`).

## 2026-10-23 — Staff Kit Tauri Window Not Visible (End)
- Implementation (Victor):
  * Root Cause:
    1. In `src-tauri/tauri.conf.json`, `visible: false` keeps the main window hidden initially to eliminate white/black startup flash.
    2. Commit `9bcb573` had removed the `show_main_window` IPC command and frontend trigger, replacing it with a single one-shot `std::thread::sleep(500ms)` followed by `window.show()`. If WebView2 initialization or the Vite dev server took longer than 500ms, or if Tao dropped the early show call without unminimize/focus, the window remained permanently hidden (`Style: 0x04CF0000`, `Vis: False`).
  * Fix:
    1. `src-tauri/src/lib.rs`: Restored public `show_main_window` command calling `window.show()`, `window.unminimize()`, and `window.set_focus()`. Registered in `invoke_handler!` and categorized in `PUBLIC_COMMANDS`.
    2. `src-tauri/src/lib.rs`: Updated background window shower in `setup` to retry every 250ms for up to 5 seconds with `unminimize()` and `set_focus()` until visibility is confirmed.
    3. `src/services/staff-api.ts`: Restored `staffApi.showMainWindow()` public IPC method.
    4. `src/services/staff-api.test.ts`: Added unit test confirming `showMainWindow` is public and invokes `show_main_window`.
    5. `src/App.tsx`: Triggered `staffApi.showMainWindow()` upon initial theme mount and database ready.
- Verification (Qeen):
  * Window visibility verified: HWND verified with `Vis: True`, `Style: 0x14CF0000` (`WS_VISIBLE`), class `Tauri Window`, title `"Staff Kit"`.
  * Unit tests: `npm run test:unit -- src/services/staff-api.test.ts` (19/19 PASS).
  * Tauri backend tests: `npm run test:tauri` (360/360 PASS, including `every_registered_command_is_classified` and `classification_counts_are_non_empty_and_disjoint`).
  * Full quality gate: `npm run check:quality` (lint, typecheck, build, cargo check — 0 errors PASS).
  * Database integrity: Zero writes or modifications to test/production DB.

# Daily Log - 2026-10-22

## 2026-10-22 — Staff Kit Auto-Create Asset Category During Import (Start)
- Authorized by Zon/Cheki.
- Lead / Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  1. During Asset Import Preview, detect Category values missing from the local database.
  2. Show "New Category: H Phone (4 assets)" and similar summaries in preview (read-only).
  3. On user Approve, atomically create missing Categories, then import their assets in the same transaction.
  4. For serialized rows, new Categories use tracking_mode = Serialized. Respect existing category configuration; never silently change tracking_mode.
  5. Match existing Categories case-insensitively and avoid duplicates. Generate unique category codes using existing conventions.
  6. Allow optional fields to remain empty. Require valid unique Asset Code and preserve duplicate/conflict safeguards.
  7. Any validation/import failure rolls back both newly created Categories and asset changes.
  8. Do not automatically assign employees, create loans, or overwrite existing assets.
- Validation: Focused unit tests (Rust + Vitest), full quality gate (`npm run check:quality`). DB safety: preserve test DB, validate against isolated test DB copy.

## 2026-10-22 — Staff Kit Auto-Create Asset Category During Import (End)
- Implementation (Victor):
  * Backend (`src-tauri/src/db/asset.rs`): Added `generate_unique_category_code_tx` and `ensure_category_exists_or_create_tx` to atomically look up or create missing asset categories within active transaction with unique code slugs and appropriate tracking modes.
  * Backend (`src-tauri/src/db/asset_import.rs`): Added `AssetDirectImportNewCategorySummary` struct and `new_categories` field to `AssetDirectImportPreview`. Extended preview inspection to count missing categories case-insensitively (`NOCASE`). Updated quantity and serialized import pipelines to atomically ensure categories exist. Added unit tests for preview detection, atomic creation, and transaction rollback.
  * Frontend (`src/types/staff.ts`): Added `AssetDirectImportNewCategorySummary` type and `newCategories` to `AssetDirectImportPreview`.
  * Frontend (`src/features/assets/AssetImportWizard.tsx`): Rendered compact preview summary cards for newly detected categories ("New Category: <Name>" with "<Count> assets").
  * Test suites: Added component test `AssetImportWizard.newCategory.test.tsx` and updated `useAssetDirectImportState.approval.test.tsx`.
- QA Validation (Qeen):
  * Verified "H Phone" missing creates exactly one category and imports 4 Samsung assets.
  * Verified reimport produces no duplicate category or assets.
  * Verified preview and cancel perform 0 writes (read-only).
  * Verified validation/import failures trigger full rollback of both category and assets.
  * Preserved original test DB (`data/test_db/staff_kit.sqlite3`).
  * Unit tests: Rust unit tests passed (355 passed); Vitest asset suite passed (14 passed across 6 test files).
  * Full quality gate (`npm run check:quality`): lint, typecheck, build, and cargo check all passed (0 errors).
- Verdict: PASS.

## 2026-10-22 — Staff Kit Approved Canonical Asset Display (Start)
- Authorized by Zon/Cheki (approved D1=A, D2=A, D3=A, D4=A).
- Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  * Phase 0: Mandatory legacy safety gate on isolated copy of test DB, baseline count verification for 92 legacy records, confirm 0 overwrites/deletions.
  * Phase 1: Canonical "Assigned Assets (N)" column in Employee List; click opens popover/drawer showing asset code, name, category, serial, usage location from assets + asset_categories + asset_loans (read-only).
  * Phase 2: Preserve COMPUTER NAME / COMPUTER (2) PC-only derivation (has_computer_name=1); keep legacy dynamic fields untouched; hide legacy asset columns by default for NEW column profiles only; use assets.usage_location as canonical.
- Validation: Focused unit tests (Rust + Vitest), full quality gate (`npm run check:quality`). DB safety: preserve test DB, validate against isolated test DB copy.

## 2026-10-22 — Staff Kit Approved Canonical Asset Display (End)
- Author: Victor (Implementation), Qeen (Independent QA).
- Outcome: Completed with focused TDD and strict legacy safety.
- Implementation Summary:
  * Phase 0 (Legacy Safety Gate): Verified 92 legacy records in employee_dynamic_values untouched (asset_name: 92, cateagory: 91, location: 92 across 442 employees). Zero overwrites/migrations.
  * Backend: Added `assigned_assets_count` join in `src-tauri/src/db/employee.rs` and `schema.rs`. Implemented `get_employee_assigned_assets` Tauri command and core column `assignedAssets` with deterministic ordering (`borrowed_at ASC, asset_code ASC`). Added IPC classification in `src-tauri/src/lib.rs`.
  * Frontend: Added `AssignedAssetsModal` component displaying canonical assets, category, serial, and usage location. Connected clickable badge in `EmployeeView.tsx`.
  * Business Rules Verified: D1=A (canonical assigned assets display), D2=A (legacy fields preserved untouched, hidden by default for NEW profiles only), D3=A (has_computer_name=1 restricted to PCs, no phone/monitor computer name derivation), D4=A (assets.usage_location is canonical, no two-way sync with employee LOCATION).
- Quality Gate Verification:
  * Backend: `cargo test` passed (357 passed, 0 failed).
  * Frontend: Vitest passed (283 passed across 38 files), `npm run lint` passed (0 errors), `npm run typecheck` passed (0 errors), `npm run build` passed.
  * Gate: Full `npm run check:quality` passed cleanly.
- Verdict: PASS.

## 2026-10-22 — Staff Kit Additional Approved Asset Business Rules (Start)
- Authorized by Zon/Cheki clarification.
- Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  * Verification: Confirm Borrow/Assign workflow handles unlimited devices (0/1/many per category, e.g. 3 laptops, 4 monitors) without artificial caps.
  * H Phone default location: Apply default usage_location "Tại nhà" on assignment when usage_location is NULL/empty; preserve existing explicit locations without overwriting; preserve locations on Return.
  * UI grouping: In AssignedAssetsModal, group active assigned assets by category (e.g. Laptop (3), Monitor (4), H Phone (1)) for high readability, displaying individual asset code, name, serial, and canonical usage location.
  * Safety: Preserve original test DB untouched (test on isolated copy); keep legacy dynamic fields and loans intact; full quality gate.

## 2026-10-22 — Staff Kit Additional Approved Asset Business Rules (End)
- Author: Victor (Implementation), Qeen (Independent QA).
- Outcome: Completed with focused TDD and strict legacy safety.
- Implementation Summary:
  * Backend (`src-tauri/src/db/borrow.rs` & `src-tauri/src/db/asset_import.rs`):
    - Added rule in borrow approval and asset import assignment: when H Phone is assigned with NULL/empty `usage_location`, default to "Tại nhà".
    - Preserved existing explicit locations without overwriting.
    - Verified Return retains the asset's existing location.
    - Verified employee can hold unlimited devices across categories (0/1/many: tested 3 laptops, 4 monitors with mixed locations, 2 H Phones simultaneously = 9 active loans) with zero artificial caps.
  * Frontend (`src/features/employees/AssignedAssetsModal.tsx`):
    - Grouped assigned assets by category (e.g. Laptop (1), H Phone (1), Monitor (2)) with category headers and count badges.
    - Cleaned table rows to display individual asset code, name, serial number, canonical usage location ("Tại nhà", "Tại công ty", etc.), and assigned date.
  * Tests:
    - Added Rust integration test `test_unlimited_devices_and_hphone_location_rules` in `src-tauri/src/db/borrow.rs`.
    - Added category grouping and mixed-locations unit tests in `src/features/employees/AssignedAssetsModal.test.tsx`.
- Quality Gate & QA Verification:
  * Backend: `cargo test` passed (358 passed, 0 failed).
  * Frontend: Vitest passed (all employee suites passed, 67 tests).
  * Gate: Full `npm run check:quality` passed cleanly (lint 0 errors, typecheck 0 errors, build successful, cargo check passed).
  * Safety: Original test DB `data/test_db/staff_kit.sqlite3` preserved untouched.
- Verdict: PASS.

## 2026-10-22 — Staff Kit Fix AssetList.xlsx Import Failure (Start)
- Authorized by Zon/Cheki.
- Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  1. Reproduce actual import failure using Zon's latest AssetList.xlsx (20 assets); capture failed stage and root error.
  2. Inspect sheet/header offset (headers start at column B), trailing blank/whitespace rows, numeric Samsung serials, and category resolution.
  3. Confirm Asset Tag → asset_code; Asset Name → display_name; Category → asset_categories; Brand/Model/Serial → canonical asset fields.
  4. Verify missing categories are auto-created ONLY on Approve, in the same transaction.
  5. Preserve valid blank serials for Monitors; H Phone must not derive a PC Computer Name.
  6. Distinguish New / Existing-Skipped / Error accurately. Reimport must be idempotent.
  7. Protect test DB (data/test_db) and production DB. No DB reset or direct import into original DB.
  8. Fix confirmed parser/UI/validation defects with focused TDD; do not relax asset identity safeguards.
- Validation: Focused tests + full quality gate; Qeen verification on isolated DB copy.

## 2026-10-22 — Staff Kit Fix AssetList.xlsx Import Failure (End)
- Author: Victor (Implementation), Qeen (Independent QA).
- Outcome: Completed with focused TDD and strict database protection.
- Investigation & Diagnostic Findings:
  * Workbook analysis (`00_ExSource/AssetList.xlsx`): Sheet `Asset`, 20 rows, headers `["Asset Tag", "Asset Name", "Category", "Brand", "Computer Name", "Model", "Serrial Number"]`.
  * Column mapping verified: `Asset Tag` → `asset_code`, `Asset Name` → `display_name`, `Category` → `asset_type`, `Serrial Number` → `serial_number`.
  * Database state: 6 Laptops already exist in DB (`VNLAP504`, `505`, `506`, `523`, `526`, `536`); 4 H Phones (`VNHPH001`–`004`) have numeric serials and category `H Phone` which is missing from DB; 10 Monitors (`VNMON761`, etc.) have empty/null serials.
  * Preview stage: 20 total rows recognized (0 phantom rows), 14 valid (new), 6 existing-skipped, 0 errors. Auto-detected 1 missing category: `H Phone (4 assets)`. Zero-writes invariant verified (0 DB changes on preview/cancel).
  * Approve stage: `H Phone` category created atomically in the same transaction (`tracking_mode = Serialized`, `has_computer_name = 0`). 14 assets imported cleanly with canonical types, models, brands. Samsung numeric serials preserved as strings (`359545150098862`), Monitor empty serials preserved as NULL.
  * Reimport stage: Idempotent (Total: 20, New: 0, Existing-Skipped: 20, Error: 0).
  * Column B offset & trailing whitespace: Added tests verifying headers offset to column B and trailing empty/whitespace rows are cleanly skipped without producing phantom rows or field misalignment.
- Tests & Verification:
  * Added `test_diagnose_zon_asset_list_import` in `src-tauri/src/db/asset_import.rs`.
  * Added `test_excel_header_offset_and_trailing_whitespace_rows` in `src-tauri/src/db/asset_import.rs`.
  * Ran full backend test suite: 360 passed, 0 failed.
  * Quality Gate: `npm run check:quality` (ESLint 0 errors, tsc 0 errors, build OK, cargo check OK).
  * Database safety: Original test DB `data/test_db/staff_kit.sqlite3` strictly untouched.
- Verdict: PASS.


# Daily Log - 2026-10-18

## 2026-10-18 — Staff Kit HR Lifecycle UI Label Cleanup (Start)
- Authorized by Zon/Cheki.
- Lead / Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  1. Remove visible "Rehired" badge beneath employee names in Employee List (desktop table and mobile card view). Preserve `lifecycle_status`, audit history, and rehire behavior in backend/DB.
  2. Rename dropdown option "Rehire to Employee List" to "Employee List".
  3. Keep existing group-transfer confirmation, permissions, and database logic unchanged.
  4. Update focused UI tests, run quality gate, commit and push.
- Validation: Focused unit tests (Vitest), full quality gate (`npm run check:quality`). DB safety: preserve test DB.

## 2026-10-18 — Staff Kit HR Lifecycle UI Label Cleanup (End)
- Implementation (Victor):
  * Removed desktop table and mobile card visible "Rehired" badges in `EmployeeView.tsx`. Backend lifecycle status, audit trail, and rehire handling remain untouched.
  * Renamed dropdown option in `EmployeeView.tsx` from "Rehire to Employee List" to "Employee List".
  * Retained action button rehire logic, confirmation dialogues, permissions, and database logic completely unchanged.
  * Added focused unit test suite `EmployeeView.lifecycleLabels.test.tsx` verifying absence of "Rehired" badge and proper dropdown option text/value.
- QA Validation (Qeen):
  * Verified unit tests: 59 passed across employee test suite (`EmployeeView.lifecycleLabels.test.tsx`, `MissingEmployeesModal.test.tsx`, etc.).
  * Full quality gate (`npm run check:quality`): ESLint, TypeScript (`tsc -b`), Vite build, and Cargo check passed with 0 errors.
  * Verified dark/light mode compatibility (uses standard CSS variables and tokens).
  * Confirmed zero DB schema, migration, or backend query changes.
- Verdict: PASS. Ready for git commit and push to `origin/main`.

## 2026-10-18 — Staff Kit Missing Drill-down & Bulk Move Group (Start)
- Authorized by Zon/Cheki.
- Lead / Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  1. In EE Sync Preview, make the Missing count badge clickable with cursor pointer and badge styling.
  2. Open compact detail modal/drawer listing only Missing employees with Staff ID, Full Name, Current Group, and checkbox selection.
  3. Support single selection, Select All, search filtering, Open Profile/Edit trigger, and bulk Move Group.
  4. Allow selecting valid target group adhering to HR lifecycle rules; reject invalid transitions.
  5. Confirmation step before save showing count, employee identities, current group → target group, and warning that Missing does not mean resigned.
  6. Explicit approval required before group changes; transactional batch application using existing employee group transfer functions.
  7. Refresh employee counts and EE Sync Preview after successful changes; recalculate Missing based on current EE List membership.
  8. Remote Query B strictly SELECT-only; Missing never causes automatic Offboarding; preserve Conflicts, Skip/Approve gating, and audit trails.
- Validation: Focused unit tests (Vitest), cargo check/tests, full quality gate (`npm run check:quality`). DB safety: preserve test DB.

## 2026-10-18 — Staff Kit Missing Drill-down & Bulk Move Group (End)
- Implementation (Victor):
  * Added `MissingEmployeesModal.tsx` for drilling down Missing preview employees with search, single/bulk selection, Select All, individual edit trigger, and bulk group transfer.
  * In `EmployeeView.tsx`: transformed Missing badge into interactive clickable button when missing > 0; hooked modal open/close, single edit via `enterTableEditModeForEmployee`, and transactional `handleBulkMoveMissing`.
  * In `useEmployeeState.ts`: added `refreshMssqlSyncPreview` to dynamically re-evaluate sync preview against current database state without full manual trigger.
  * Preserved HR lifecycle rules, transactional rollback on error, canonical employee IDs, asset loan audit history, conflict blocking, and theme/accessibility styling.
- QA Validation (Qeen):
  * Tested MissingEmployeesModal: 7/7 tests passing (render, search filter, single & select all, confirmation dialog, rejection/cancellation zero-write behavior, error handling).
  * Tested employee MSSQL flow: 20/20 tests passing.
  * Full quality gate (`npm run check:quality`): ESLint, TypeScript typecheck (`tsc -b`), Vite build, and Cargo check passed cleanly with 0 errors.
  * Backend test suite (`npm run test:tauri`): 353 Rust tests passed cleanly.
  * DB Safety: Test DB preserved; zero remote MSSQL writes; zero unintended local DB mutations.
- Verdict: PASS. Ready for git commit and push to `origin/main`.

## 2026-10-18 — Staff Kit Compact EE Sync Status UI (Start)
- Authorized by Zon/Cheki.
- Lead / Implementer: Victor (Senior Developer). QA Validator: Qeen (Independent QA Validator).
- Scope:
  1. Remove long inline preview message next to Update in Employee List; keep compact preview popover.
  2. In Config Sync, show "✓ Connected" with green success styling on successful connection test, hide redundant PASS banner, reset to "Test Connection" on settings change or test failure.
  3. After successful Approve, show compact success notification with accurate imported/updated counts, auto-hide after 10 seconds. Never auto-hide failures.
  4. Preserve dark/light themes, accessibility, DB isolation, no backend changes or permission modifications.
- Validation: Focused unit tests (Vitest) and full quality gate (`npm run check:quality`).

## 2026-10-18 — Staff Kit Compact EE Sync Status UI (End)
- Implementation (Victor):
  * Removed long inline preview message in `useEmployeeState.ts`; preview popover remains compact and fully functional.
  * In `RemoteEeSyncSettings.tsx`: Test Connection button displays `✓ Connected` in emerald green upon successful test; hides redundant PASS banner; resets to `Test Connection` if settings or credentials change or if connection test fails.
  * Formatted approval success to compact notification (e.g. `✓ Imported 1, updated 2 employees`) and added 10-second auto-dismiss timer. Failures remain visible indefinitely.
- QA Validation (Qeen):
  * Focused unit tests: 31 passed (`src/features/employees/useEmployeeState.mssql.test.tsx`, `src/features/settings/RemoteEeSyncSettings.test.tsx`).
  * Quality gate passed: ESLint, TypeScript (`tsc -b`), Vite build, Cargo check (`npm run check:quality`).
  * DB Safety: Test DB isolated; zero modifications to backend SQL, permissions, or production databases.
- Verdict: PASS. Ready for commit and push to `origin/main`.

## 2026-10-18 — Staff Kit v2.2.1 Official GitHub Release Publication (Complete)
- Approved Commit: `36b338206929f5d018f969ac7353da14bb01c3eb` (merged to `main` via `--ff-only`, pushed to `origin/main`).
- Annotated Tag: `v2.2.1` pushed to `origin`.
- Release URL: https://github.com/quockhanh2376/Staff_Kit/releases/tag/v2.2.1
- Uploaded Assets & Hashes:
  * `staff-kit.exe` (20,978,688 bytes | SHA256: `204A7AC7C7DB049B35A02682A203679FF4177E8012CD608A9FB4AA5BA3B7682E`)
  * `Staff Kit_2.2.1_x64_en-US.msi` (7,749,632 bytes | SHA256: `30A77E8F05DD53CDA929CD8D1FDFDA895604D187475A25B53B8510293CC82A00`)
  * `Staff Kit_2.2.1_x64-setup.exe` (5,578,040 bytes | SHA256: `0EA2289C7CF903F4357181442D06E98F1D0C816C7CADC42EB4976FC76DB4BDBF`)
  * `SHA256SUMS.txt` (272 bytes | SHA256: `D50BD083DA20196218E9C669F725027C8FE5980FC8E9D2C14F2A18894B8B2969`)
- Verification: Qeen downloaded assets from GitHub; SHA256 checksums matched approved build bit-for-bit.
- Database Safety: Shared DB (`0AF089EC...`) and Local DB (`1AEA3889...`) verified bit-for-bit unchanged.
- Release Status: OFFICIAL STABLE RELEASE COMPLETE.

# Daily Log - 2026-10-17

## 2026-10-17 — Prepare Staff Kit v2.2.1 Official Release (Start)
- Branch: `release/v2.2.1` from baseline `9e6525f1243791f7b249e7ab922bd1c6bb9ff883`.
- Objective: prepare official stable release v2.2.1 on dedicated release branch.
- Scope: update application version metadata consistently across `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.lock` to `2.2.1` (WiX `2.2.1.0`), prepare release notes in `docs/release-notes/2026-10-17-v2.2.1.md`.
- Preserve application functionality, database schema, and existing business logic unchanged.
- Validation: verify clean quality checks (`npm run check:quality`) and build Windows desktop packages (`npm run tauri:build`) to verify EXE, MSI, and NSIS setup artifacts with SHA256 checksums.

## 2026-10-17 — Prepare Staff Kit v2.2.1 Official Release (End)
- Updated version metadata consistently to `2.2.1` (WiX `2.2.1.0`):
  * `package.json`: `"version": "2.2.1"`
  * `src-tauri/Cargo.toml`: `version = "2.2.1"`
  * `src-tauri/tauri.conf.json`: `"version": "2.2.1"`, `"wix": { "version": "2.2.1.0" }`
  * `src-tauri/Cargo.lock`: `name = "staff-kit"` `version = "2.2.1"`
- Prepared comprehensive release notes in `docs/release-notes/2026-10-17-v2.2.1.md`.
- Application functionality, database schema, and existing business logic remain completely unchanged.
- Quality gates verified: `npm run check:quality` passed cleanly (ESLint, TypeScript typecheck, Vite production build, Cargo check). All unit, supplemental, and Tauri tests pass (247 frontend Vitest tests, 353 Rust tests).
- Packaged Windows desktop artifacts built successfully via `npm run tauri:build`:
  * Binary: `src-tauri/target/release/staff-kit.exe` (20,978,688 bytes | SHA256: `204a7ac7c7db049b35a02682a203679ff4177e8012cd608a9fb4aa5ba3b7682e`)
  * WiX MSI: `src-tauri/target/release/bundle/msi/Staff Kit_2.2.1_x64_en-US.msi` (7,749,632 bytes | SHA256: `30a77e8f05dd53cda929cd8d1fdfda895604d187475a25b53b8510293cc82a00`)
  * NSIS Setup: `src-tauri/target/release/bundle/nsis/Staff Kit_2.2.1_x64-setup.exe` (5,578,040 bytes | SHA256: `0ea2289c7cf903f4357181442d06e98f1d0c816c7cadc42eb4976fc76db4bdbf`)
- Dedicated release branch prepared: `release/v2.2.1`.

## 2026-10-17 — Prepare Staff Kit v2.2.0-rc.3 Release (Start)
- Branch: `release/v2.2.0-rc.3` from baseline `a2609a298c43e5f8a2b9f99a863fc6bd3193c8a2`.
- Objective: prepare version 2.2.0-rc.3 release candidate on dedicated release branch.
- Scope: update application version metadata consistently across `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.lock` to `2.2.0-rc.3` (WiX `2.2.0.3`).
- Preserve application functionality, database schema, and existing business logic unchanged.
- Validation: verify clean quality checks (`npm run check:quality`) and build Windows desktop packages (`npm run tauri:build`) to verify EXE, MSI, and NSIS setup artifacts.

## 2026-10-17 — Prepare Staff Kit v2.2.0-rc.3 Release (End)
- Updated version metadata consistently to `2.2.0-rc.3` (WiX `2.2.0.3`):
  * `package.json`: `"version": "2.2.0-rc.3"`
  * `src-tauri/Cargo.toml`: `version = "2.2.0-rc.3"`
  * `src-tauri/tauri.conf.json`: `"version": "2.2.0-rc.3"`, `"wix": { "version": "2.2.0.3" }`
  * `src-tauri/Cargo.lock`: `name = "staff-kit"` `version = "2.2.0-rc.3"`
- Application functionality, database schema, and existing business logic remain completely unchanged.
- Quality gates verified: `npm run check:quality` passed cleanly (ESLint, TypeScript typecheck, Vite production build, Cargo check).
- Packaged Windows desktop artifacts built successfully via `npm run tauri:build`:
  * Binary: `src-tauri/target/release/staff-kit.exe` (20,978,688 bytes)
  * WiX MSI: `src-tauri/target/release/bundle/msi/Staff Kit_2.2.0-rc.3_x64_en-US.msi` (7,749,632 bytes)
  * NSIS Setup: `src-tauri/target/release/bundle/nsis/Staff Kit_2.2.0-rc.3_x64-setup.exe` (5,576,309 bytes)
- Feature branch commit prepared: `chore(release): prepare v2.2.0-rc.3` on `release/v2.2.0-rc.3`.

## 2026-10-17 — Config Sync Header Keyboard & ARIA Accessibility (Start)
- Audit Config Sync collapse/expand panel for keyboard accessibility and ARIA semantics.
- Fix verified accessibility defects: native button semantics with focus-visible indicator, aria-controls referencing panel id, panel landmark region, and focus management when collapsed while strictly preserving visual appearance, transitions, and form persistence.
- Add focused UI regression tests for Enter/Space keyboard toggling, aria-controls, and aria-expanded state.

## 2026-10-17 — Config Sync Header Keyboard & ARIA Accessibility (End)
- Audited Config Sync collapsible panel for keyboard accessibility and ARIA semantics.
- Upgraded collapsible header to a semantic `<button type="button">` with `id="config-sync-header"`, `aria-expanded`, and `aria-controls="config-sync-panel"`.
- Assigned `id="config-sync-panel"` to the controlled container and applied `invisible` (`visibility: hidden`) / `pointer-events-none` when collapsed, transitioning via `transition-[max-height,opacity,visibility]` to prevent hidden form inputs from trapping keyboard focus when collapsed.
- Added emerald `focus-visible:ring-2` outline for clear keyboard focus indication while omitting rings on mouse clicks.
- Preserved visual appearance, CSS styling, form state persistence, idle auto-collapse, and security boundaries.
- Added comprehensive regression tests in `RemoteEeSyncSettings.test.tsx` verifying Enter/Space keyboard toggling, ARIA attributes, and focus-visible styling (247 tests passing).
- All checks pass cleanly: `npm run check:quality`, `npm run test:unit`, and `npm run test:supplemental`.

## 2026-10-17 — Config Sync UI Polish (Start)
- Rename Remote EE Sync to Config Sync. Make Config Sync a collapsible panel with styling and interaction identical to Admin Portal (default collapsed on Settings open, toggle via header/chevron, preserve fields, saved values, Save, Test Connection, and Advanced Settings).

## 2026-10-17 — Config Sync UI Polish (End)
- Renamed Remote EE Sync to Config Sync across UI title, labels, and feedback messages.
- Re-architected Config Sync as a collapsible card with styling and interaction identical to Admin Portal: rounded-[16px] surface, header with emerald Database icon, badge chevron indicator, and smooth height/opacity transitions via useIdleCollapse.
- Defaults to collapsed on Settings open; header click or chevron toggle smoothly expands and collapses the panel.
- Form state is strictly preserved across collapse and expand cycles; Save Settings and Test Connection operate reliably when expanded.
- Preserved all SQL connection, credential storage, Query B, TLS, and security gate boundaries without change.
- Automated tests pass: 353 Rust tests, 245 Vitest frontend tests; full quality gate clean (lint, typecheck, production build, cargo check).

# Daily Log - 2026-10-15

## 2026-10-15 — Simplify Remote EE Sync Settings (Start)
- Streamline Remote EE Sync settings UI to Server, SQL Username, Password with compact Save beside credentials, hide Port (default 1433), move Database into collapsed Advanced Settings (default AssetManagement), hide Authentication type (retain SQL Server), support admin account/password rotation in Save without plaintext leak or unsafe reuse, and preserve fail-closed security gating.

## 2026-10-15 — Simplify Remote EE Sync Settings (End)
- Simplified Remote EE Sync UI: removed technical description lines and unneeded separate account change panel.
- Arranged single compact row for Server, SQL Username, Password, compact ✓ Save button, and Test Connection button.
- Hidden Port (defaults to 1433 internally), hidden Authentication type (retaining SQL Server Authentication), and moved Database to collapsed Advanced Settings (defaults to AssetManagement).
- Single Save action supports admin account and password rotation, purges obsolete credentials on login change, records authorization metadata, and rejects prohibited logins (`aswhite-plus-webserver`) without credential leakage.
- Preserved TLS and fail-closed permission checks; backward-compatible defaults ensure existing saved profiles remain functional post-upgrade.
- Automated tests pass: 353 Rust tests, 243 Vitest tests, lint, typecheck, and Vite production build clean.

## 2026-10-15 — EE Sync Preview Buttons: Skip | Approve and no-op write gating (Start)
- Streamline EE List sync preview actions to two buttons (Skip | Approve), gate Approve on importable changes with no blocking conflicts, and guarantee no-op zero-write transactional safety.

## 2026-10-15 — EE Sync Preview Buttons: Skip | Approve and no-op write gating (End)
- Replaced Cancel with Skip and renamed Approve and update EE List to Approve; exactly two preview buttons.
- Skip clears Preview without calling IPC or DB writes; no lifecycle/group changes.
- Approve disabled when importable changes (New + Changed) == 0 (e.g. all-unchanged or missing-only) or when blocking conflicts exist.
- Backend transactional apply helper `apply_employee_list_diff_tx` rejects unapproved zero-diff/no-op apply attempts with error, guaranteeing zero DB writes.
- Verified Repeat sync is no-op and never writes. Verified Floating overlay, Offboarding, and asset loans remain intact.
- Full test suites pass: 348 Rust tests, 241 Vitest tests, lint and typecheck clean.


## 2026-10-15 — Fix login failure after lifecycle migration (Start)
- Diagnose and fix startup migration failure preventing local accounts from loading during app initialization.

## 2026-10-15 — Fix login failure after lifecycle migration (End)
- Root cause: `idx_employees_staff_group` and `idx_employees_floating` were defined in `BASE_SCHEMA_SQL` before `ensure_employee_columns()` ran `ALTER TABLE` to add the columns to existing databases.
- Fix: Deferred index creation in `ensure_employee_columns()` after column migration, removing premature index creation from `BASE_SCHEMA_SQL`.
- Preserved existing local accounts, credentials, employees, assets, and loans without DB recreation or reset.
- Added regression test `existing_db_without_lifecycle_columns_migrates_cleanly_and_preserves_local_accounts`.


## 2026-10-15 — Approved HR Lifecycle and Floating workflow implementation (Start)
- Implement approved HR lifecycle states (`lifecycle_status`), 4 canonical staff groups + Floating overlay (`is_floating`), transitions (Reject Offer, Offboarding, Rehire, Internal Movement), and Query B Sync diff/collapsing.

## 2026-10-15 — Approved HR Lifecycle and Floating workflow implementation (End)
- Kept 4 canonical staff groups; implemented Floating as overlay via `is_floating = 1`.
- Implemented `lifecycle_status` (`active`, `pending`, `offer_rejected`, `resigned`, `rehired`, `transferring`).
- Floating employees appear in both EE List and independent Floating view with synchronized counts.
- Implemented transitions with explicit approval confirmation (`window.confirm`).
- Preserved Staff ID, employee_id, audit history, teams, and asset loans.
- Query B comparison treats Missing (ASWVN1145) as report-only without auto-offboarding.
- Color-coded diff highlights and collapsible Unchanged rows in Sync Preview.
- Added DB migration and automated regression test suite; all 17 Rust employee tests and 236 frontend unit tests pass cleanly.

# Daily Log - 2026-04-20

## 2026-08-06 — SEC-002 Phase C blocker fixes (Start)

- Implement cancellation actor attribution, atomic Phase C migration rollback, and corrected pending-claim uniqueness documentation.

## Objective
Polish the `Asset Dashboard` serialized UI so the table header, ID presentation, holder layout, and category overview cards align more closely with the approved compact mock.

## Work Completed
- Updated the serialized table header in [AssetDashboard.tsx](/e:/Staff_Kit/src/features/assets/AssetDashboard.tsx) to match the mock more closely:
  - removed the leading drag icon before column labels
  - switched header typography to the stronger emerald treatment
  - removed `ASC / DESC` text in favor of the slimmer sort icon only
- Changed serialized `ID` rendering on the UI:
  - database value still keeps the full asset code like `VNWKS252`
  - UI now shows only the numeric suffix like `252`
  - search behavior still uses the full stored asset code
- Refined serialized `Holder` rendering and width:
  - name locked to the first line
  - employee code locked to the second line
  - widened the `Holder` column in [serializedAssetGridConfig.ts](/e:/Staff_Kit/src/features/assets/serializedAssetGridConfig.ts) for longer names
- Restyled the serialized category overview strip to follow the visual reference in [AssetDashboard.jsx](/e:/Staff_Kit/ExSource/AssetDashboard.jsx):
  - removed the colored title-strip background
  - kept per-category text/icon colors
  - kept the subtle body tint and bottom accent line
- Updated the UI rail in [asset-dashboard-phase1-ui.test.ts](/e:/Staff_Kit/scripts/asset-dashboard-phase1-ui.test.ts):
  - category filter default label now checks for `Categories`
  - clear-filter behavior now checks the search-bar clear button instead of a standalone `Clear Filters` button

## Validation
- Ran `node --experimental-strip-types scripts/asset-dashboard-phase1-ui.test.ts`
- Ran `npx eslint src/features/assets/AssetDashboard.tsx src/features/assets/serializedAssetGridConfig.ts scripts/asset-dashboard-phase1-ui.test.ts`
- Ran `npx tsc -b --pretty false`
- Result: passed

## Current State
The serialized Asset Dashboard now follows the current compact mock more closely:
- stronger emerald table headers
- cleaner header controls
- numeric-only displayed IDs
- two-line holder cells
- neutral title strip for the category overview cards

# Daily Log - 2026-04-20

## Objective
Audit the live `main` branch, confirm local/remote sync, verify the current codebase end-to-end, and capture the active task context before the next slice starts.

## Current Mainline State
- Local `main` is aligned with `origin/main` at `a73112f`.
- Root local drift was cleared by restoring `src-tauri/Cargo.lock` back to `HEAD`.
- Remaining local-only files are documentation drafts:
  - `docs/superpowers/plans/2026-04-17-employee-asset-seed-phase2-1-hardening.md`
  - `docs/superpowers/specs/2026-04-17-employee-asset-seed-phase2-1-hardening-design.md`

## Review Result
- Reviewed the current `main` codebase around the latest shipped slice:
  - `a73112f` `feat: derive computer names and add asset dashboard column settings`
  - preceding merged refactor/seed hardening commits already on `origin/main`
- No blocking findings were found on the current `main` after verification.
- One residual product check remains in `openspec/changes/asset-computer-name-derive-from-category/tasks.md`:
  - manual smoke test for seeded laptop display/computer name
  - manual smoke test for monitor rows showing blank computer name

## Verification
- Ran `npm run check:quality`
- Ran `npm run test:tauri`
- Result: passed
  - ESLint: passed
  - TypeScript build/typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed
  - Rust tests: `77 passed, 0 failed`

## Active Task Context
- The current mainline task/theme is `asset-computer-name-derive-from-category`.
- The design source for the latest shipped backend/frontend behavior is:
  - `openspec/changes/asset-computer-name-derive-from-category/design.md`
  - `openspec/changes/asset-computer-name-derive-from-category/tasks.md`
- The next practical step is manual smoke verification of the seeded Asset Dashboard behavior before starting a new slice.

# Daily Log - 2026-04-09

## Objective
Consolidate the `Settings` and auth surfaces into one compact visual system on `main`, then checkpoint that UI state into NotebookLM so future work starts from the current product shell instead of the older mixed styling.

## Work Completed
- Merged the compact `Admin Portal` line into `main` and kept the card behavior consistent with the new `Database & Backup` pattern:
  - compact header with icon + chevron
  - idle auto-collapse after `60s`
  - click header to reopen
  - import target moved into the card as a compact `Import + TO:` toolbar
- Restyled `Login` and `Forgot Password` to match the newer compact system while keeping the shared app shell:
  - dark mode: `#0d1117` background, `#161b22` card surface, slate borders, emerald primary action
  - light mode: matching compact structure with the same hierarchy and action emphasis
- Tightened `Settings` copy and reduced noise:
  - removed redundant helper text from `Admin Portal`, `Import`, and `Database & Backup`
  - moved Borrow LAN controls out of `Settings` and into `Borrow / Return Review`
  - removed the local-account quick-switch button so account testing now goes through real login/logout
- Compacted `Database & Backup` into the new tone:
  - unified background/text/input/button surfaces
  - neutralized the old amber DB-location block while keeping the warning copy
  - snapshot helper copy shortened to `Auto Save and Restore Back`
  - backup retention copy shortened to `7 versions and delete after 400 days`
- Final header polish on `main`:
  - `Database & Backup` renamed to `Data-Backup`
  - `Admin Portal (Local Accounts)` reduced to `Admin Portal`
  - both card headers now share the same height/padding rhythm

## Validation
- Ran `node --experimental-strip-types scripts/action-icon-ui.test.ts`
- Result: passed
- Ran `npm run check:frontend`
- Result: passed
  - ESLint: passed
  - TypeScript build/typecheck: passed
  - Vite production build: passed

## Current State
`main` now uses a compact, consistent Settings/auth shell: `Admin Portal`, `Data-Backup`, `AssetDashboard`, `Login`, and `Forgot Password` all follow the same dark/light visual language, and the card headers/controls are tighter and easier to scan.

## Next Suggested Focus
- Manual visual QA on desktop for `Settings`, `Login`, and `Forgot Password` in both dark and light modes.
- Continue replacing older copy/layout leftovers so the rest of the Settings area follows the same compact pattern.
- Introduce role-specific capability gating only after the visual shell is fully stable.

---

# Daily Log - 2026-04-07

## Objective
Stabilize the `Asset Dashboard` chunk after local Excel-import testing showed the Settings screen could fall into a black window state instead of returning cleanly to the Settings surface.

## Work Completed
- Reproduced the failure through the real local Tauri/WebView runtime instead of assuming a backend import issue.
- Narrowed the crash path to the post-import Settings reload flow:
  - import/stage action triggers `triggerReload()`
  - Settings re-renders `Asset Dashboard` and `Asset Import Wizard`
  - runtime crashes on string formatting that relied on `String.prototype.replaceAll`
- Replaced the risky `replaceAll` calls in the dashboard/import render path with runtime-safe formatting:
  - `src/features/assets/assetDashboardCopy.ts`
  - `src/features/assets/AssetImportWizard.tsx`
- Added a focused regression rail in `scripts/asset-dashboard-formatting.test.ts` that temporarily removes `String.prototype.replaceAll` to simulate the older WebView/runtime behavior seen on the local machine.
- Tightened the import UX so the asset import wizard can close itself after a fully successful import with no remaining review rows:
  - `src/features/assets/assetImportCopy.ts`
  - `src/features/assets/useAssetImportState.ts`

## Validation
- Ran `node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts`
- Result: passed
- Ran `npm run check:quality`
- Result: passed
  - ESLint: passed
  - TypeScript build/typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Current State
The Chunk 4 branch no longer depends on `String.prototype.replaceAll` inside the Settings/import render path, so local WebView runtimes that lack that API should stay on the Settings surface after import instead of dropping into a black screen.

## Next Suggested Focus
- Re-test the full Excel import flow end-to-end on the local desktop app with real files after this fix is pushed.
- If Settings stays stable, continue with review/merge of the stacked Asset Dashboard PRs.

---

# Daily Log - 2026-04-06

## Objective
Implement Chunk 1 of the `Asset Dashboard` plan by laying the schema and category foundation before touching workbook parsing or dashboard UI.

## Work Completed
- Created isolated branch/worktree `codex/asset-dashboard-chunk-1` from `origin/main` to keep the dashboard implementation separate from the root workspace docs checkpoint.
- Followed the Chunk 1 TDD path:
  - wrote a failing rail for `assets.display_name_short` and `assets.usage_location`
  - wrote a failing rail for the new `asset_category_prefixes` model and seeded laptop/monitor prefix families
  - verified both failures before implementation
- Extended the asset schema and migrations with:
  - `assets.display_name_short`
  - `assets.usage_location`
  - new `asset_category_prefixes` child table
  - active-prefix uniqueness index
- Migrated the category foundation so `Laptop` stays one logical category while supporting multiple code families:
  - `VNLAP`
  - `VNMACPRO`
  - `VNIMACPRO`
  - `VNMACAIR`
- Seeded monitor prefix coverage with `VNMON`.
- Added backend helpers for:
  - prefix normalization
  - category lookup by asset-code prefix
  - idempotent prefix upsert/update during seeding and future CRUD
- Committed the Chunk 1 slice as `cd977d8` with message:
  - `feat: add asset dashboard schema foundation`

## Validation
- Ran `cargo test --manifest-path src-tauri/Cargo.toml --lib assets_table_persists_dashboard_metadata_columns -- --nocapture` and confirmed the expected red failure before adding the new asset columns.
- Ran `cargo test --manifest-path src-tauri/Cargo.toml --lib seeded_asset_category_prefixes_cover_laptop_family_and_monitor_codes -- --nocapture` and confirmed the expected red failure before adding the prefix table.
- Re-ran `cargo test --manifest-path src-tauri/Cargo.toml asset:: -- --nocapture` after implementation.
- Result: `7 passed, 0 failed`.

## Current State
The repository now has the schema primitives required for the Asset Dashboard slice: monitor-specific asset metadata and multi-prefix category recognition. This is the backend base layer needed before generalizing workbook parsing for `AssetList.xlsx`, `Monitor.xlsx`, and `Mouse-Key.xlsx`.

## Next Suggested Focus
- Start Chunk 2 from `docs/superpowers/plans/2026-04-06-asset-import-dashboard.md`.
- Extend workbook parsing/staging for the three real file shapes.
- Keep raw source values intact while threading `usage_location` and `display_name_short` into staged rows.

---

# Daily Log - 2026-04-06

## Objective
Convert `Asset-import-dashboard-context.md` into the official dashboard design documents so the next coding slice can start from a spec-first, execution-ready baseline.

## Work Completed
- Re-read `E:\Staff_Kit\Asset-import-dashboard-context.md` against the current `v2.0.3` codebase instead of treating the context file as a greenfield plan.
- Promoted the dashboard idea into the official spec at `docs/superpowers/specs/2026-04-06-asset-import-dashboard-design.md`.
- Wrote the matching implementation plan at `docs/superpowers/plans/2026-04-06-asset-import-dashboard.md`.
- Corrected the most important architecture point before coding:
  - serialized inventory remains in `assets + asset_loans`
  - quantity inventory remains in `stock_items`
  - the dashboard is a read/operate surface on top of those tables, not a second asset system
- Locked the first execution order to:
  - schema and category-prefix foundation
  - workbook import expansion for `AssetList.xlsx`, `Monitor.xlsx`, and `Mouse-Key.xlsx`
  - dashboard read APIs and Settings UI
  - category management and hardening

## Current State
The dashboard slice now has an approved spec plus an execution-ready plan that matches the current repository reality. Coding can begin from the schema/category chunk without reopening earlier import decisions or reinterpreting the context file from scratch.

## Next Suggested Focus
- Start implementation from `docs/superpowers/plans/2026-04-06-asset-import-dashboard.md`.
- Keep serialized and quantity inventory as separate sources of truth while expanding dashboard coverage.
- Extract new dashboard state/helpers into focused files instead of growing `AssetImportWizard.tsx` and `useAssetImportState.ts` further.

---

# Daily Log - 2026-04-06

## Objective
Load the `Asset Import Dashboard` context into NotebookLM and prepare the project to start that slice from a clean design-first checkpoint.

## Work Completed
- Re-read `E:\Staff_Kit\Asset-import-dashboard-context.md` after the file was saved with real content.
- Confirmed the new context defines the next product direction as an `Asset Dashboard` inside Settings, with:
  - three Excel import structures
  - dashboard summary cards and detail tables
  - stock tracking integration with Borrow / Return
  - later category-management UI
- Captured the implementation shape from the context into the project progress log so NotebookLM reflects the new starting point.
- Flagged the main engineering constraints before coding:
  - `AssetImportWizard.tsx` and `useAssetImportState.ts` are already large and should be split while dashboard work grows
  - the context still contains one mojibake string in `Usage Location` (`Tại NHÃ€`) that should be normalized before final UI copy
  - stock mutation rules must stay aligned with the existing Borrow / Return approval flow instead of bypassing it

## Current State
The dashboard slice is now staged as the next implementation track after `v2.0.3`. NotebookLM has the release baseline plus this new dashboard context checkpoint, so design and execution can begin from the current mainline without reopening earlier asset-import history.

## Next Suggested Focus
- Start with backend schema/category groundwork and import-pipeline changes before adding dashboard UI.
- Split the work into:
  - category/schema foundations
  - import + stock integration
  - dashboard UI + category management UI
- Convert the context file into a formal spec/implementation plan before writing code.

---

# Daily Log - 2026-04-06

## Objective
Start the `Asset Import Dashboard` preparation slice from `Asset-import-dashboard-context.md` and sync the kickoff status into NotebookLM before implementation begins.

## Work Completed
- Checked the requested source file at `E:\Staff_Kit\Asset-import-dashboard-context.md`.
- Confirmed the file currently exists on disk but is empty (`0 bytes`), so it is not a usable implementation context yet.
- Deliberately did not upload the empty file to NotebookLM to avoid polluting the `Staff_Kit` notebook with a blank source.
- Paused implementation preparation until the dashboard context file is saved with real content.

## Current State
The project is ready to start the next slice, but the requested context source is still blank on disk. NotebookLM progress tracking will resume from this checkpoint once the file content is available.

## Next Suggested Focus
- Save the real dashboard context content into `Asset-import-dashboard-context.md`.
- Re-run the upload step to NotebookLM.
- Then start the design and execution prep for the dashboard slice from that saved source.

---

# Daily Log - 2026-04-06

## Objective
Ship `Staff Kit 2.0.3` as the first complete owner-aware asset import release on top of the stabilized Borrow / Return mainline.

## Work Completed
- Merged the owner-aware asset import slice into `main` after stabilizing the employee laptop-query path.
- Extended the asset import wizard so it now supports two serialized-asset lanes from workbook-style files:
  - `Available` rows import warehouse assets into `in_stock` without owner resolution.
  - `Laptop` rows stage owner-aware assigned assets with review before commit.
- Added employee-owner resolution against the full employee master across all staff groups (`employee_list`, `onboarding`, `internal_movement`, `offboarding`).
- Kept raw source columns intact in staged rows so future mapping expansion is still possible.
- Added review-time owner editing and warning handling:
  - unresolved employee rows stay out of the successful import set
  - mismatched name/team rows remain editable and reviewable
- Committed approved `Laptop` rows into official data by creating serialized asset records plus active `asset_loans`.
- Switched employee-table `Computer Name` rendering to derived active laptop ownership values:
  - display value = `ASW` + `asset_code`
  - multiple active laptops render as comma-separated lines in one cell
- Hardened employee computer-name behavior after review:
  - derived `Computer Name` is read-only in table edit mode
  - save payload preserves the stored raw `employees.computername` value instead of echoing the derived display
  - duplicate detection now tokenizes multi-line / comma-separated computer names per laptop
  - active-laptop aggregation order is deterministic by `asset_loans.id`
  - employee count queries now skip the laptop aggregation join unless a computer-name query actually needs it

## Validation
- Ran `npm run check:quality` -> passed.
- Ran `npm run test:tauri` -> passed.
- Ran `node --experimental-strip-types scripts/owner-aware-asset-import-mapping.test.ts` -> passed.
- Ran `node --experimental-strip-types scripts/employee-computer-name-rules.test.ts` -> passed.
- Ran `cargo test employee::tests:: -- --nocapture` -> passed.

## Current State
`main` now supports importing warehouse assets and already-assigned laptop assets from the same wizard flow, with IT review deciding what becomes official. Employee table `Computer Name` values are derived from active laptop loans, stay searchable, and support multiple active laptops per employee. This is the release baseline for `2.0.3`.

---

# Daily Log - 2026-03-31

## Objective
Start `ST 2.0.2` with the smallest backend-first groundwork for QR Return Flow by generalizing the pending review model just enough to carry a request type without changing current borrow behavior.

## Work Completed
- Created feature branch/worktree `codex/st-2-0-2-qr-return` from merged `security`.
- Added `request_type TEXT NOT NULL DEFAULT 'borrow'` to the `borrow_requests` base schema.
- Added a legacy migration step so existing databases gain `borrow_requests.request_type` with default `borrow`.
- Threaded `request_type` through the Rust `BorrowRequestRecord` load path and submit path audit payload.
- Exposed `requestType` in the shared TypeScript `BorrowRequestRecord` contract.
- Kept the scope intentionally narrow:
  - no return submit endpoint yet
  - no admin UI review changes yet
  - no approval/reject logic changes yet

## Validation
- Wrote the failing Rust rails first:
  - `submit_borrow_request_creates_pending_request_for_valid_employee_and_assets`
  - `apply_migrations_upgrades_existing_borrow_requests_with_request_type`
- Verified the red step before implementation:
  - compile/test failed because `BorrowRequestRecord` did not yet expose `request_type`
- Re-ran the two targeted Rust tests after implementation
- Ran `npm run check:quality`
- Result: passed
  - targeted borrow request test: passed
  - targeted legacy migration test: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vite production build: passed
  - Tauri `cargo check`: passed

## Git History Added
- `f19fd59` - `feat: add request type to pending review model`

## Current State
The codebase can now distinguish pending review records by type at the data-contract level while still behaving exactly like borrow-only flow in production. This gives `ST 2.0.2` a safe anchor point for the next slice: LAN return submit path.

## Next Suggested Focus
- Add a public LAN return-submit path and minimal request input shape without enabling return approval yet.
- Decide whether the first public return UX should be a dedicated `/return` page or a shared borrow/return page with mode switch.
# Daily Log - 2026-04-05

## Objective
Add Borrow / Return mode toggle to the LAN phone page so employees can choose which operation to perform when scanning a single shared QR code, and surface the LAN server liveness status with automatic Windows Firewall rule management.

## Work Completed

### LAN Server Firewall + Probe
- Added `ensure_firewall_rule(port)` in `src-tauri/src/lan_server.rs` that runs `netsh advfirewall` silently at startup to allow inbound TCP on the configured borrow port; no-op stub added for non-Windows targets.
- Added `probe_lan_server(port)` Tauri async command in `src-tauri/src/lib.rs` (TCP connect to `127.0.0.1:port`, 800 ms timeout) and registered in invoke handler.
- Added `probeLanServer` in `src/services/staff-api.ts`.
- Added `lanServerAlive: boolean | null` state in `src/features/borrow/useBorrowState.ts` with a `useEffect` that probes on mount and when port changes.
- Added server status badge (spinner / green / amber) in `src/features/borrow/BorrowAdminView.tsx`; shows firewall troubleshooting note when server appears down.

### Borrow / Return Mode on Phone Page
- Added `request_type TEXT NOT NULL DEFAULT 'borrow'` column to `borrow_requests` table in `src-tauri/src/db/schema.rs`.
- Added `ensure_borrow_request_columns(conn)` migration in `src-tauri/src/db/mod.rs` — adds column via `ALTER TABLE` if not present.
- Added `search_assigned_assets_conn(conn, query, limit)` in `src-tauri/src/db/asset.rs` — mirrors in-stock search but filters `status = 'assigned'`.
- Updated `src-tauri/src/db/borrow.rs`:
  - Added `REQUEST_TYPE_BORROW` / `REQUEST_TYPE_RETURN` constants.
  - Added `request_type: Option<String>` to `BorrowRequestSubmitInput` and `request_type: String` to `BorrowRequestRecord`.
  - `submit_borrow_request_conn` — validates request type, checks in-stock for Borrow or assigned for Return before inserting.
  - `load_request_state_tx` — returns 3-tuple `(employee_id, status, request_type)` using `COALESCE(request_type, 'borrow')`.
  - `approve_borrow_request_conn` — branches on request type: Return path sets `IN_STOCK` + updates `asset_loans.returned_at`; Borrow path sets `ASSIGNED` + inserts `asset_loans` row.
  - `load_borrow_request_record` — reads `request_type` from DB.
- Added `/api/assigned-assets` GET route in `src-tauri/src/lan_server.rs`.
- Rewrote `borrow_page_html()` in `src-tauri/src/lan_assets.rs`:
  - Mode toggle: **Borrow** (green) / **Return** (amber) buttons at top of form.
  - `MODES` config object switches title, description, helper text, submit label, and API endpoint on toggle.
  - Asset search hits `/api/assets` (Borrow) or `/api/assigned-assets` (Return).
  - POST body includes `requestType` field.
- Added `requestType: string` to `BorrowRequestRecord` type in `src/types/staff.ts`.
- Added Borrow/Return type badges in `src/features/borrow/BorrowAdminView.tsx` — pending queue list items and request detail header.
- Updated nav button label in `src/App.tsx` to **Borrow / Return** (dimmed slash) on both desktop and mobile nav.

## Validation
- `cargo check` → Finished dev target(s) — 0 errors
- `npm run build` → 1762 modules, 0 errors, built in 10.72s

## Current State
Employees can now scan one fixed QR code and choose to Borrow (green) or Return (amber) before submitting. IT sees Borrow/Return type badges in the admin approval queue, and the approval logic correctly branches: Return mode restores asset stock and closes the loan, while Borrow mode assigns the asset and opens a new loan. The LAN server auto-registers a Windows Firewall rule on startup so phone connections are not blocked by default.

---

# Daily Log - 2026-04-05

## Objective
Correct Borrow URL host detection so Staff Kit uses the machine's current LAN IP instead of a public internet IP, and add a manual refresh control for re-detecting the host from Settings.

## Work Completed
- Replaced the Borrow host detection path in `src-tauri/src/db/borrow.rs` so it now derives the active LAN IP from the machine's outbound network interface instead of calling external public-IP services.
- Renamed the Tauri command and frontend API wiring from public-WAN naming to Borrow LAN host detection:
  - `detect_public_wan_ip` -> `detect_borrow_lan_host`
  - `detectPublicWanIp` -> `detectBorrowLanHost`
- Kept the host normalization logic strict so loopback and unspecified addresses are rejected.
- Updated `useSettingsState.ts` so the Settings screen now:
  - auto-detects the current LAN IP after loading Borrow settings
  - preserves manual edits unless the user explicitly clicks refresh
  - supports `handleRefreshBorrowLanHost` to replace the current input with a fresh LAN IP detection result
  - shows clearer inline notes when detection succeeds or when LAN IP detection is unavailable
- Updated `SettingsView.tsx` so the Borrow LAN card:
  - explains LAN-IP detection instead of public-WAN detection
  - disables the host/port inputs while refresh is running
  - shows a new `Refresh LAN IP` action beside the save button
  - keeps the Borrow URL preview live from the current input values
- Removed the temporary `reqwest` dependency because LAN detection no longer needs outbound HTTP requests.

## Validation
- Re-ran `node --experimental-strip-types scripts/borrow-lan-autofill.test.ts` -> passed.
- Re-ran targeted Rust test:
  - `cargo test normalize_detected_borrow_lan_host_candidate_accepts_real_ips_and_rejects_loopback --lib` -> passed.
- Ran `npm run check:quality` -> passed.
- Ran `npm run test:tauri` -> passed (`40 passed, 0 failed`).

## Current State
The Borrow LAN settings now detect the machine's local LAN IP, let IT refresh that detection on demand, keep the host editable, and require an explicit save before changing persisted Borrow URL settings.

# Daily Log - 2026-04-05

## Objective
Auto-detect the current public WAN IP of the machine running Staff Kit and prefill the Borrow URL host automatically, while still allowing IT to edit the host manually before saving.

## Work Completed
- Added backend WAN IP detection in `src-tauri/src/db/borrow.rs` using a small fallback chain of public IP endpoints.
- Exposed new Tauri command `detect_public_wan_ip` in `src-tauri/src/lib.rs` and wired it through `src/services/staff-api.ts`.
- Added frontend helper module `src/features/settings/borrowLanAutoFill.ts` for:
  - choosing detected WAN IP over saved host when available
  - building live Borrow URL preview from current input values
  - formatting IPv6 hosts safely in preview URLs
- Updated `useSettingsState.ts` so Settings now:
  - loads saved Borrow settings immediately
  - detects current public WAN IP in the background
  - auto-fills the host input with detected WAN IP when available
  - does not overwrite the field after the user has started typing manually
  - keeps save behavior explicit; auto-detect only prefills, it does not auto-save
- Updated `SettingsView.tsx` so the Borrow URL preview now reflects the current input values instead of only the last saved backend value.
- Updated copy in the Settings card to explain that the Borrow URL host is auto-detected but still editable.

## Validation
- TDD red step (frontend): added `scripts/borrow-lan-autofill.test.ts` first and confirmed failure before the helper module existed.
- TDD red step (backend): added Rust tests first and confirmed failure before WAN parsing helpers existed.
- Re-ran `node --experimental-strip-types scripts/borrow-lan-autofill.test.ts` -> passed.
- Re-ran targeted Rust tests:
  - `parse_public_wan_ip_candidate_accepts_valid_ip_and_rejects_noise` -> passed
  - `build_borrow_lan_url_wraps_ipv6_hosts` -> passed
- Ran `npm run check:quality` -> passed.
- Ran `npm run test:tauri` -> passed (`40 passed, 0 failed`).

## Current State
The Borrow URL section now auto-fills the current public WAN IP for the host field when detection succeeds, keeps the field editable, and previews the exact URL that will be saved. Existing save behavior remains manual and explicit.

# Daily Log - 2026-04-05

## Objective
Unify admin action UI across employee table tooling, Teams, and Settings so row-level actions use the same icon-driven visual language.

## Work Completed
- Replaced column-drawer-specific icon button classes with generic reusable action icon classes in `src/index.css`.
- Kept the stronger gray employee table header and clearer header column dividers as part of the same visual consistency pass.
- Updated `ColumnsDrawer` to use the shared action-icon classes.
- Updated `TeamView` action column:
  - removed the three-dot dropdown for row actions
  - replaced it with direct edit and trash icon buttons
- Updated Settings account actions:
  - `Use` -> user-switch icon
  - `Edit` -> pencil icon
  - `Reset Password` -> key icon
  - `Delete` -> trash icon
- Preserved clarity with `title` and `aria-label` on all icon actions.
- Added `scripts/action-icon-ui.test.ts` to verify Teams, Settings, and ColumnsDrawer all use the shared icon-action pattern.

## Validation
- TDD red step: added `scripts/action-icon-ui.test.ts` first and confirmed failure before the generic action-icon pattern existed in all target screens.
- TDD green step: re-ran `node --experimental-strip-types scripts/action-icon-ui.test.ts` -> passed.
- Ran `npm run check:frontend` -> passed (ESLint + TypeScript + build).

## Current State
Teams, Settings, and Column Preferences now share the same icon-based action treatment, making the admin UI more compact, clearer, and visually consistent in both light mode and dark mode.

# Daily Log - 2026-04-05

## Objective
Refine the employee table header to look closer to the target UI and clean up Column Preferences actions by replacing text buttons with clearer icon actions.

## Work Completed
- Darkened the employee table header gray background one more step for stronger separation from body rows.
- Added dedicated vertical header dividers so column boundaries read more clearly.
- Kept the header styling driven by theme tokens for both light mode and dark mode.
- Updated Column Preferences action buttons:
  - replaced visible `Rename` text with edit icon button
  - replaced visible `Delete` text with trash icon button
  - preserved accessibility with `aria-label` and `title`
- Added dedicated icon-button styling for normal and destructive actions in the drawer.

## Validation
- Ran `npm run check:frontend` -> passed (ESLint + TypeScript + build).
- Confirmed `ColumnsDrawer.tsx` no longer renders visible `Rename/Delete` text for action buttons; labels remain only in accessibility attributes.

## Current State
The employee table header now has a stronger gray band with clearer column separation, and the Column Preferences drawer uses compact edit/trash icons for cleaner, sharper UI.

# Daily Log - 2026-04-05

## Objective
Give the employee table header a clearer gray background, matching the desired visual separation in both light and dark mode.

## Work Completed
- Added dedicated theme tokens for the employee table header background and divider in `src/index.css`.
- Updated `.table-head` and sticky header cells to use the new gray header background instead of the previous subtle surface/background mix.
- Kept the change scoped to the employee table header only.
- Applied the styling through theme variables so both light mode and dark mode render a consistent gray header treatment.

## Validation
- Ran `npm run check:frontend` -> passed (ESLint + TypeScript + build).

## Current State
The employee table header now reads as a distinct gray band in both themes, closer to the requested look from image 2, while body rows remain unchanged.

# Daily Log - 2026-04-05

## Objective
Show numeric-only staff ID on frontend tables (`ASWVN1253` -> `1253`) while preserving full `employeeId` in data/storage to keep import uniqueness and prevent collisions.

## Work Completed
- Added `formatEmployeeIdForDisplay` helper in `src/features/employees/employeeIdDisplay.ts`.
- Applied display formatting in `EmployeeView` only (UI layer):
  - EE ID move-selector button in table rows
  - table cell display for `employeeId` column
  - mobile card subtitle
- Kept raw `employeeId` unchanged in backend payloads, state, DB, and import flow.
- Because `EmployeeView` is shared by `employee_list`, `onboarding`, `offboarding`, and `internal_movement`, the display change is now active for all four views.

## Validation
- TDD red step: added `scripts/employee-id-display.test.ts` first, confirmed failure before helper existed.
- TDD green step: implemented helper and re-ran test.
- Ran `node --experimental-strip-types scripts/employee-id-display.test.ts` -> passed.
- Ran `npm run check:frontend` (ESLint + TypeScript + build) -> passed.

## Current State
Frontend now shows numeric-only ID in employee tables and cards, while the canonical full ID remains intact for uniqueness guarantees and Excel import integrity.

# Daily Log - 2026-04-05

## Objective
Full code-quality audit of the desktop frontend (TypeScript, ESLint, naming, file-size, browser API usage) before the next development cycle. No behavior changes — read-only pass with findings documented for future action.

## Scope
Audit covered all files under `src/` using the project's own `check:quality` pipeline plus manual inspection of: type safety, console usage, `eslint-disable` suppressions, TODO markers, naming conventions, and file-size guideline adherence.

## Findings — Green (no action needed)

- **TypeScript strict mode**: `strict: true`, `noUnusedLocals`, `noUnusedParameters` all active in `tsconfig.app.json`. Zero type errors. Zero `any` types. Zero `@ts-ignore` / `@ts-expect-error` suppressions.
- **ESLint**: passes cleanly. Only 2 `eslint-disable` comments in `App.tsx`, both for `react-hooks/exhaustive-deps` with documented intent (scoped theme restore and column prefs on auth). Acceptable.
- **Console discipline**: Only one `console.error` in `useAuthState.ts` (line 89) inside a catch block — permitted by convention.
- **No TODO / FIXME / HACK markers** anywhere in `src/`.
- **Naming conventions**: hooks use `use` prefix, constants use SCREAMING_SNAKE, types use PascalCase, files follow correct casing throughout.
- **Service layer is clean**: `staff-api.ts` is a thin typed wrapper over Tauri `invoke`. No business logic leaking into the API layer.
- **Shared copy helpers**: `assetImportCopy.ts`, `assetImportMessages.ts`, `assetImportStatusMeta.ts`, `assetImportModeConfig.ts` all under 340 lines and well-factored.

## Findings — Amber (technical debt, not blocking demo)

### File-size overages
The project follows a 300-line limit for React components and 150-line limit for hooks. Several files significantly exceed these:

| File | Lines | Guideline |
|------|-------|-----------|
| `AssetImportWizard.tsx` | 726 | 300 (component) |
| `SettingsView.tsx` | 707 | 300 (component) |
| `EmployeeView.tsx` | 632 | 300 (component) |
| `useAssetImportState.ts` | 628 | 150 (hook) |
| `useColumnState.ts` | 625 | 150 (hook) |
| `App.tsx` | 562 | 300 (component) |
| `useAuthState.ts` | 387 | 150 (hook) |
| `useSettingsState.ts` | 378 | 150 (hook) |
| `useTableEdit.ts` | 347 | 150 (hook) |

These are all cohesive and internally clean — no mixed concerns, no logic in wrong layers. The overages come from feature density, not sloppiness. They are tech debt to address when those features evolve next, not before demo.

### `window.confirm` / `window.prompt` usage
14 browser dialog calls spread across `useColumnState.ts`, `useAuthState.ts`, `useSettingsState.ts`, `useAssetImportState.ts`, and `useTeamState.ts`. These work in Tauri but are UX-rough (native OS dialogs instead of in-app confirms). Acceptable for current phase; worth replacing with in-app confirm components in a future UX polish pass.

## Validation
- Ran `npm run check:quality` → passed (ESLint + TypeScript + Vite build + `cargo check`)
- Ran `npm run test:tauri` → `38 passed, 0 failed`
- Ran all 3 script tests → `asset-import-copy`, `asset-import-messages`, `asset-import-category-options` all passed

## Current State
The codebase is in clean, demo-ready shape. Zero blocking quality issues. Technical debt is confined to file-size overages (no logic problems) and native browser dialogs (no functional problems). Safe to demo or start the next feature from here.

## Next Suggested Focus
- **Demo**: branch is clean (`security`, tagged `v2.0.1`), all checks green
- **Next feature**: consider splitting `useAssetImportState.ts` and `AssetImportWizard.tsx` naturally when QR return flow (v2.0.2) work begins — the new slice will make the split boundary obvious
- **UX polish future**: replace `window.confirm` / `window.prompt` calls with in-app modal confirms when the UI matures

# Daily Log - 2026-03-30

## Objective
Finish the nearby batch-title polish by aligning the wizard `Active Batch` label with the `Active Import Batch` wording already used in Settings.

## Work Completed
- Added a shared copy helper for the active staged-batch section title.
- Updated the wizard review rail to use `Active Import Batch` instead of `Active Batch`.
- Updated the Settings active-batch card to read from the same helper so both surfaces stay aligned.
- Kept scope copy-only:
  - no import behavior changes
  - no state changes
  - no summary-count changes

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
- `6af11b0` - `style: align active import batch titles`

## Current State
The asset-import batch vocabulary is now aligned across the main nearby headings: `Staged Import Batches`, `Active Import Batch`, and the staged-batch empty/count copy. The copy cluster is more stable for future UI work because the shared helper owns the repeated labels.

## Next Suggested Focus
- Decide whether the remaining asset-import headings should stay centralized in `assetImportCopy.ts` or be split once new behavior work resumes.
- Shift back from copy polish to behavior work on the import flow once product wants the next functional slice.

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

# Daily Log - 2026-04-02

## Objective
Start `ST 2.0.2` with the first meaningful QR return slice while keeping the current desktop borrow review UI unchanged.

## Scope Locked
- Backend-first and hidden from the current desktop UI
- Employee-side return submit creates a pending request only
- No stock mutation, no `returned_at` mutation, and no return approval/reject flow yet
- Current borrow-only queue and review actions stay protected from pending return requests

## Work Completed
- Created a fresh feature worktree from `security` and replayed the `request_type` groundwork.
- Added a narrow QR return submit backend flow in `src-tauri/src/db/borrow.rs`:
  - writes pending requests with `request_type = "return"`
  - validates submitted assets against active employee loans
  - writes `return_request.submit` audit logs
  - blocks current borrow approve/reject actions from handling non-borrow requests
  - keeps `list_pending_borrow_requests` borrow-only so the current admin UI is not polluted by hidden return requests
- Added LAN endpoint coverage in `src-tauri/src/lan_server.rs`:
  - `POST /api/return-requests`
  - valid and invalid return-submit tests
- Added/expanded Rust tests for:
  - valid return submit
  - unknown employee rejection
  - duplicate asset-code rejection
  - active-loan ownership validation
  - borrow approve/reject guards on non-borrow requests
  - audit logging
  - queue filtering to exclude pending return requests

## Verification
- Ran `node ./scripts/run-with-shared-cargo-target.mjs cargo test --manifest-path src-tauri/Cargo.toml borrow -- --nocapture`
- Ran `node ./scripts/run-with-shared-cargo-target.mjs cargo test --manifest-path src-tauri/Cargo.toml lan_server -- --nocapture`
- Ran `npm run check:quality`
- Ran `npm run test:tauri`
- Result: passed
  - targeted borrow tests: passed
  - targeted LAN server tests: passed
  - frontend lint/typecheck/build: passed
  - Tauri quality verification: passed
  - full Rust/Tauri tests: `49 passed, 0 failed`

## Git History Added
- `b93716e` - `feat: add request type to pending review model`
- `6632d8e` - `docs: add qr return submit slice spec and plan`
- `abfcd38` - `feat: add lan qr return submit endpoint`

## Current State
`codex/st-2-0-2-qr-return-v1` now has the first safe return-flow slice on top of `security`: return submit exists and is auditable, but desktop review remains borrow-only until the next visible review slice is designed.
# Daily Log - 2026-04-01

## Objective
Lock the `pending-reviews` web build regression with a dedicated CI rail that proves `npm run build` still works when `DATABASE_URL` is unset.

## Work Completed
- Added `.github/workflows/web-build-no-db.yml`.
- Scoped the workflow to changes under `web/**` and the workflow file itself.
- Configured the job to:
  - use Node.js 20
  - run in `web/`
  - install with `npm ci`
  - build with `DATABASE_URL` forced to an empty string

## Verification
- Re-ran `npm run build` in `web` with `DATABASE_URL` unset after adding the workflow.
- Result: passed

## Current State
The branch now has a dedicated GitHub Actions rail for the exact failure mode we just fixed, so future regressions should surface immediately on push/PR instead of during release prep.

# Daily Log - 2026-04-01

## Objective
Remove the last hard requirement for `DATABASE_URL` during `web` production builds on `pending-reviews`, so `npm run build` can succeed in CI or local release prep without a live database URL.

## Root Cause
- `web/prisma.config.ts` used `env("DATABASE_URL")`, so `prisma generate` failed while loading Prisma config before the build even started.
- `web/src/lib/prisma.ts` instantiated `PrismaClient` at module import time, so even after unblocking `prisma generate`, Next.js build-time route evaluation still crashed while collecting page data.

## Work Completed
- Changed `web/prisma.config.ts` to read `process.env.DATABASE_URL ?? ""` so Prisma config can load without throwing during `prisma generate`.
- Refactored `web/src/lib/prisma.ts` to lazily create the Prisma client only on first property access instead of at module import time.
- Preserved the existing test-mode guard so DB-backed suites still fail loudly if they accidentally touch Prisma without a configured test database.

## Verification
- Ran `npm run check` in `web` with `DATABASE_URL` unset.
- Result: passed
  - ESLint: passed
  - TypeScript typecheck: passed
  - Vitest: `28 passed`, `7 skipped`
  - `prisma generate`: passed without `DATABASE_URL`
  - `next build --webpack`: passed without `DATABASE_URL`

## Current State
`pending-reviews` no longer requires `DATABASE_URL` just to generate Prisma client code or complete a production build. A real database URL is still required at runtime when server code actually uses Prisma.

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
Capture the QR-based asset receive/return business rules from `AssetDesktop_Pro` so `Staff_Kit` can continue development without creating any code or runtime coupling between the two projects.

## Project Naming
- `Staff_Kit` is referred to as `project ST`.
- `AssetDesktop_Pro` (repository path `E:\AssetDesk-Pro`) is referred to as `project ASP`.

## Work Completed
- Reviewed `project ASP` in read-only mode to extract the business flow for QR-based asset handover.
- Confirmed `project ST` and `project ASP` are independent projects and must remain independent.
- Confirmed only `project ST` is allowed to change code; `project ASP` was used strictly as a flow and rules reference.
- Focused the review on asset receive, asset return, approval, and auditability rules that are relevant for future `project ST` development.

## ASP Reference Inputs
- `E:\AssetDesk-Pro\openspec\specs\receive-flow\spec.md`
- `E:\AssetDesk-Pro\openspec\specs\return-flow\spec.md`
- `E:\AssetDesk-Pro\openspec\specs\approval-workflow\spec.md`
- `E:\AssetDesk-Pro\openspec\specs\audit-log\spec.md`
- `E:\AssetDesk-Pro\src\lib\workflows\workflows.service.ts`

## Business Conditions Extracted From ASP
- QR receive and QR return must start from a valid, active session created by an authorized management user.
- Invalid, expired, or closed QR sessions must be rejected.
- Employee-facing QR submission must create a pending request only; it must not directly mutate the official asset stock or assignment state.
- Official stock and assignment changes must happen only after an approval step.
- Asset codes used in QR flows must already exist in the system before submission; unknown asset codes must be rejected.
- Duplicate asset codes inside the same request must be rejected.
- Receive flow may contain multiple assets in one request, but the reviewed assets must still be eligible for assignment when approved.
- Return flow may contain multiple assets in one request, but only assets that are currently assigned to the submitted employee and eligible for return may be accepted.
- Receive flow requires the employee to acknowledge the active policy version; stale policy versions must be rejected until the latest version is reloaded.
- Review actions must be role-restricted and auditable.
- Receive review may revise both the employee target and the asset list.
- Return review may remove assets from the submitted list, but must not add new assets that were not submitted.
- Rejected requests must leave official stock and assignment data unchanged.
- Submit, approve, reject, and sensitive session actions should all be audit logged with actor/context, timestamp, request type, result, and affected assets.

## Implications For Next ST Development
- `project ST` may continue building its own QR-based asset handover flow by following the business rules above, but all implementation must stay inside `project ST`.
- Do not import or mirror `project ASP` code directly into `project ST`.
- If `project ST` adds receive/return flows, model pending requests, approval decisions, policy version handling, and audit logs explicitly instead of mutating employee asset state directly from QR submissions.
- Keep the separation boundary clear: `project ASP` is a reference for business flow only, not a shared code dependency.

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

# Daily Log - 2026-04-22

## Objective
Deprecate the `employee_asset_seed` flow at the UI level while keeping the backend/API path intact as a fallback during transition, then sync repository state and NotebookLM context.

## Work Completed
- Removed `employee_asset_seed` wiring from [App.tsx](/e:/Staff_Kit/src/App.tsx):
  - no `useEmployeeAssetSeedState`
  - no `EmployeeAssetSeedDrawer` mount
- Removed the `Seed Assets` action from [EmployeeView.tsx](/e:/Staff_Kit/src/features/employees/EmployeeView.tsx).
- Kept backend/API contracts untouched:
  - Rust commands still exist
  - TypeScript API methods still exist
  - snapshot/review/import backend path remains available for future fallback or controlled reintroduction
- Updated [employee-asset-seed-ui.test.ts](/e:/Staff_Kit/scripts/employee-asset-seed-ui.test.ts) to assert the new state:
  - backend/API still present
  - UI entry points removed

## Product Decision Locked
- `Asset List import` is now the primary source-of-truth onboarding flow for assets.
- `employee_asset_seed` is deprecated from the visible UI first.
- Backend cleanup or full removal should happen in a later dedicated slice after confirming the fallback path is no longer needed.

## Verification
- `node --experimental-strip-types scripts/employee-asset-seed-ui.test.ts`
- `npx eslint src/App.tsx src/features/employees/EmployeeView.tsx scripts/employee-asset-seed-ui.test.ts`
- `npx tsc -b --pretty false`

## Git History Added
- `7a42311` - `refactor: deprecate employee asset seed ui`

## Start — Codex tooling runtime hardening — 2026-07-26 13:33 +07:00

- Session ID: `a38e47c0-6cb0-42d5-844e-8a63a69a4e1b`
- Machine: Windows workstation (`D:\Staff_Kit`)
- Branch: `main`
- Worktree: `D:\Staff_Kit`
- Base commit: `0c77d04dcd78b557fd23e00c7f976835b1ed0af0`
- Objective: repair and harden the complete Codex tooling environment until a fresh audit reaches deterministic `READY`.
- Known audit findings: global Superpowers is not project-local; legacy `.agent` workflows coexist with newer routers; `.tokensave` state and Headroom auto-registration exist; Headroom config contains stale `E:\Staff_Kit` and tracked-config mutation risk; wrapper auto-installs extra components; project-local runtime and fail-open behavior require verification.
- In scope: project-local Superpowers, ECC Lite/focused-output inventory, Headroom wrapper/runtime/config isolation, tokensave removal, portable paths, docs, verification script, and daily-log protocol.
- Out of scope: `src/`, `src-tauri/`, `web/`, database schema, application dependencies, global plugin cache deletion, secrets/auth, and unrelated user changes.
- Planned validation: static tooling audit, fresh runtime discovery, Headroom MCP/config immutability checks, fail-open check, wrapper syntax, documentation consistency, and Git scope review.
- Existing uncommitted changes: `.codex/config.toml` and `AGENTS.md` modified; `.agents/skills/staffkit-*`, `docs/CODEX_WORKFLOW.md`, `docs/ecc-lite-manifest.md`, and `.tokensave/` untracked. These are classified as expected tooling changes for this repair and were backed up outside the repository before edits.

## End — Codex tooling runtime hardening — 2026-07-26 13:41 +07:00

- Result: tooling repair completed; static audit is `READY`.
- Files changed: project-local Superpowers skills and provenance, legacy `.agent/README.md`, `.codex/config.toml`, `.gitignore`, `AGENTS.md`, `docs/CODEX_WORKFLOW.md`, `scripts/run-codex-headroom.ps1`, `scripts/verify-codex-tooling.ps1`, `docs/ecc-lite-manifest.md`, and this log.
- Exact commands executed: `git status/diff/remote`, backups to `%TEMP%`, cloned `obra/superpowers`, copied the pinned skill set, `npm run codex:headroom -- --version`, static verification, PowerShell fail-open smoke test, `git diff --check`, and tracked-scope checks.
- Checks passed: required project-local Superpowers inventory, ECC Lite count, focused-output presence, Headroom MCP JSON syntax, no project `.tokensave`, no tracked runtime state, daily log tracking, wrapper syntax, stale-path scan, Headroom launch, config hash immutability, and fail-open direct Codex launch.
- Checks failed: initial `git diff --check` reported three copied skills with extra blank EOF lines; normalized and reran successfully.
- Checks not run: product quality gate and application tests, because product code and application dependencies were not changed; full MCP request/response protocol test was not run because it would require launching a persistent server.
- Remaining risks: global Headroom history may retain prior logs/rtk artifacts, but the Staff Kit runtime does not discover or register them; the global Superpowers plugin cache remains installed but is not imported.
- Branch status: `chore/codex-tooling-runtime-hardening`.
- Commit status: committed as `eedc682` (`chore(tooling): harden portable Codex runtime`).
- Push status: push attempted twice and timed out; remote branch has not been confirmed.
- Second-machine availability: source-pinned Superpowers and portable wrapper/docs are ready; machine two must install prerequisites, create `.headroom-venv`, install `headroom-ai[all]`, then run `scripts/verify-codex-tooling.ps1`.

## Start — Focused Codex tooling acceptance repair — 2026-07-26 14:00 +07:00

- Session ID: `2fe9148d-04b0-41d5-9893-b728bf4cb8ef`
- Machine/worktree: Windows, `D:\Staff_Kit`
- Branch/base: `chore/codex-tooling-runtime-hardening` at `18ad6fcdd4394034f13c98c76a94e2fb9e61a3b2`
- Objective: resolve the remaining live-runtime acceptance blockers without changing product code.
- Existing change classification: `.codex/config.toml` contains the three expected, runtime-generated Headroom approval blocks; this is an `EXPECTED_TOOLING_CHANGE` preserved for deliberate normalization. No other working-tree change exists.
- In scope: deterministic Headroom approvals, `codex_apps` classification, stale tracked paths, legacy `.agent` authority, explicit fail-open, verifier coverage/wording, and documentation alignment.
- Out of scope: `src/`, `src-tauri/`, `web/`, Superpowers/ECC restructuring, main-branch merge, auth/provider corruption, new MCP installation.
- Planned validation: Codex strict config parsing, static verifier reporting `STATIC READY`, path/workflow scans, direct-launch smoke test, Headroom live acceptance with config hashes, and explicit Git scope review.

### Checkpoint — 2026-07-26 14:08 +07:00

- Root causes confirmed: missing Headroom `enabled_tools`; runtime auth/config stored inside the worktree; no explicit direct launch; `codex_apps` is host-provided rather than project MCP; legacy workflow mandates referenced unavailable tools/waits; verifier conflated static and live readiness.
- Static result: `STATIC READY`.
- Codex strict config parse: passed on `codex-cli 0.146.0-alpha.3.1`.
- Effective Headroom allow-list: exactly `headroom_stats`, `headroom_compress`, `headroom_retrieve`.
- Direct fail-open smoke test: `npm run codex:direct -- --version` returned exit code 0 without starting Headroom.
- Product scope: no changes under `src/`, `src-tauri/`, or `web/`.
- Next: commit this focused repair, then run a fresh live Headroom acceptance test before any push.

## End — Focused Codex tooling acceptance repair — 2026-07-26 14:29 +07:00

- Result: focused repair validated; live Headroom acceptance is `READY`.
- Static verifier: `STATIC READY`; strict Codex config parse passed on `codex-cli 0.146.0-alpha.3.1`.
- Live calls: fresh canonical Headroom sessions invoked `headroom_stats`, `headroom_compress`, and `headroom_retrieve`; all exited 0. Retrieval returned `STAFF_KIT_HEADROOM_ACCEPTANCE_2026_07_26_ALPHA` exactly.
- Config SHA-256: before `4FF2CF999260278C4711D19357B388E70DE86CAEDC9B005F4E793F1BCD033851`; after stats, compress, and retrieve unchanged at the same value. `.codex/config.toml` has no diff.
- `codex_apps`: classified as built-in/host-provided; absent from project/user MCP config and `codex mcp list`, present only in host-managed app capability inventory.
- Active project MCP inventory: `headroom` only, with exactly three approved tools (`headroom_stats`, `headroom_compress`, `headroom_retrieve`).
- Fail-open: `npm run codex:direct -- --version` returned exit code 0 and emitted the explicit Headroom-unavailable warning.
- Scope: no changes under `src/`, `src-tauri/`, or `web/`; stale paths and legacy workflow authority conflicts resolved. Runtime state remains ignored/untracked.
- Commit: `88d8e69` (`fix(tooling): stabilize Codex Headroom integration`), amended after this End entry.
- Push: pending final remote verification; no merge into `main`.
- Note: Headroom proxy emitted WebSocket 403 prewarm warnings before falling back to HTTPS; MCP operations still completed successfully.

## Start — Per-worktree Codex runtime isolation — 2026-07-26 17:46 +07:00

- Branch/base: `chore/codex-tooling-runtime-hardening` at `964389163f5444627d219aa773b8a626c8118f61`.
- Objective: isolate writable Codex runtime state per resolved repository/worktree root while keeping it outside the repository.
- Root cause: the fixed machine-local `Staff_Kit\codex-runtime` directory maps every clone and worktree on one machine to the same writable `CODEX_HOME`.
- Scope: Headroom launcher runtime-path derivation, read-only verifier coverage, concise workflow documentation, and this session log.
- Out of scope: `src/`, `src-tauri/`, `web/`, application dependencies, `scripts/db_query.py` unless inspection finds a concrete defect, and merging into `main`.
- Planned validation: red/green runtime-path checks, PowerShell syntax, `STATIC READY`, direct-launch smoke test, distinct synthetic-root demonstration, config hash immutability, diff checks, and explicit product-scope review.

## End — Per-worktree Codex runtime isolation — 2026-07-26 17:52 +07:00

- Result: writable Codex runtime state is derived outside the repository as `%LOCALAPPDATA%\Staff_Kit\codex-runtime\<16-character-sha256-id>`, with the resolved absolute worktree root normalized to lowercase before hashing.
- TDD evidence: the updated verifier first reported `STATIC NOT READY` because the shared production helper was absent; after adding the helper and wiring the launcher to it, the same verifier reported `STATIC READY`.
- Determinism evidence: neutral sample roots `C:\Staff_Kit` and `c:\staff_kit\` both produced ID `7e3b96ab03c315ad`; `C:\Staff_Kit\.worktrees\feature-a` produced distinct ID `dcd891a52cf75248`.
- Runtime boundary: the actual derived path was confirmed outside the resolved repository; no private absolute path was printed or recorded.
- Launcher checks: `npm run codex:direct -- --version` and `npm run codex:headroom -- --version` both exited 0; direct fail-open warning and exit-code propagation remain intact.
- Config immutability: `.codex/config.toml` SHA-256 remained `4FF2CF999260278C4711D19357B388E70DE86CAEDC9B005F4E793F1BCD033851` before and after launcher checks.
- Exclusions: `.tokensave` remained absent; Headroom configuration retained `--no-tokensave` and `--no-serena`.
- Review result: `scripts/db_query.py` retains its expected LOCALAPPDATA database path and required no change.
- Scope: no files under `src/`, `src-tauri/`, or `web/` changed; `main` was not merged.
- Commit/push: focused commit and remote SHA verification follow this End record.

## Start — Portable verifier search fallback — 2026-07-26 18:14 +07:00

- Branch/base: `chore/codex-tooling-runtime-hardening` at `115ecb784f5099f494c096d47140b0b9d6c3fae8`.
- Objective: remove the verifier's undeclared ripgrep dependency while preserving stale-path and legacy-MCP check semantics.
- Root cause: `scripts/verify-codex-tooling.ps1` invokes `rg` directly, but ripgrep is unavailable on this machine.
- Scope: verifier search backend selection, equivalent search semantics, this workflow documentation only if needed, and this session log.
- Out of scope: product code, runtime-path helper/launcher behavior, `scripts/db_query.py`, executable installation, PATH changes, and merging into `main`.
- Planned validation: no-`rg` static run, known-match and no-match fallback fixtures, runtime/config checks, diff checks, and explicit product-scope review.

## End — Portable verifier search fallback — 2026-07-26 18:31 +07:00

- Result: verifier no longer requires ripgrep; it selects `rg`, then `git grep`, then PowerShell `Select-String` without installing executables or changing PATH.
- Root cause fixed: direct `rg` invocation at the stale-path and legacy-workflow checks was replaced by structured `Search-RepositoryText` results with `Matches`, `NoMatch`, and `Failure` statuses.
- No-`rg` evidence: temporarily removed only the ripgrep directory from the current test process PATH; `Get-Command rg` returned no command, the verifier reported `Search backend: git grep`, and exited 0 with `STATIC READY`.
- Fallback evidence: outside the Git worktree, PowerShell fallback detected a known text fixture (`Matches`, 1 record), returned clean `NoMatch` for an absent pattern, and excluded a binary fixture.
- Normal backend evidence: with ripgrep available, verifier reported `Search backend: rg` and `STATIC READY`.
- Runtime/config checks: direct Codex launch exited 0; `.codex/config.toml` SHA-256 remained `4FF2CF999260278C4711D19357B388E70DE86CAEDC9B005F4E793F1BCD033851`; existing runtime-path isolation checks remained passing.
- Scope: no changes under `src/`, `src-tauri/`, or `web/`; `scripts/db_query.py` and docs were unchanged.
- Commit/push: focused commit and remote SHA verification follow this End record.

## Start — Fix Vite Cargo target watcher EBUSY — 2026-07-26 18:38 +07:00

- Branch: `main`.
- Base commit: `282c7d3d3281644d2d625bee5ea009c27e77fdc8`.
- Objective: prevent Vite from watching generated Cargo artifacts that are locked by Cargo/Tauri on Windows.
- Confirmed error: `EBUSY: resource busy or locked` while watching `src-tauri/target/debug/deps/staff_kit_lib.dll`.
- Root cause: Vite used the repository root and traversed `src-tauri/target`; `.gitignore` does not configure the Vite watcher.
- In scope:
  - `vite.config.ts` watcher exclusions;
  - validation;
  - this session log.
- Out of scope:
  - product business logic;
  - Cargo configuration;
  - pre-existing `src-tauri/Cargo.toml` changes;
  - unrelated lint problems under reference-only `web/`.
- Planned validation:
  - resolved Vite watcher config;
  - TypeScript build;
  - frontend build;
  - Tauri check;
  - `tauri:dev` smoke test;
  - `git diff --check`.

## End — Fix Vite Cargo target watcher EBUSY — 2026-07-26 19:35 +07:00

- Result: added repository-relative watcher exclusions for generated Cargo target directories.
- Behavior: `npm run tauri:dev` no longer crashes when Vite encounters `staff_kit_lib.dll` locked by Cargo/Tauri.
- Files changed:
  - `vite.config.ts`;
  - `daily_log.md`.
- Validation passed:
  - resolved Vite watcher regression assertion;
  - `npx tsc -b --pretty false`;
  - `npm run build`;
  - `npm run check:tauri`;
  - `npx eslint src vite.config.ts`;
  - `npm run tauri:dev` for approximately 65 seconds;
  - `git diff --check`.
- Smoke evidence: Vite became ready on port 5173, Rust compilation completed, the app launched, no `EBUSY` or `staff_kit_lib.dll` watcher error appeared, the targeted process tree stopped with zero processes remaining, and port 5173 was released.
- Validation limitations:
  - GUI observation and a separate HMR edit test were not available in the headless validation session;
  - repository-wide `npm run lint` remains affected by pre-existing issues under reference-only `web/`.
- Preserved unrelated change: `src-tauri/Cargo.toml` remained untouched and unstaged.
- Remaining risks: no known blocker for the watcher fix.
- Commit status: pending focused commit.
- Push status: pending governance check and remote verification.

## Start — Employee List click-to-copy email — 2026-07-27 14:47 +07:00

- Session: Codex Direct
- Worktree: `D:\Staff_Kit\.worktrees\employee-email-click-copy`
- Branch: `feature/employee-email-click-copy`
- Base: `d79eea91255ed8f5c0dce8660cb5b433a8a8e6f6`
- Objective: add metadata-driven Employee List email-cell clipboard interaction while preserving inline editing mode.
- Constraints: no commit, push, merge, or direct edits to `main`; stop after automated tests and bounded smoke validation.

## End - Employee List click-to-copy email foundation repair - 2026-07-27 15:20 +07:00

- Repaired semantic metadata for core email and canonical dynamic azure_account.
- Preserved optional dataType through the frontend column mapping.
- Added Rust serialization tests, Vitest/jsdom React DOM tests, and frontend mapping coverage.
- Checks passed: focused Rust tests (3/3), React unit suite (5/5), pure helper regression, and TypeScript typecheck.
- Deferred by checkpoint instruction: EmployeeView integration, final styling, full quality gate, manual smoke acceptance, commit, and push.
- Generated Tauri schema formatting changes from test execution were restored; no unrelated schema changes remain.

## End — Employee List click-to-copy email — 2026-07-27 15:35 +07:00

- Result: finalized the accepted Employee Email Click-to-Copy feature on `feature/employee-email-click-copy`.
- Manual GUI acceptance by anh Zon confirmed hover, orange copied state, exact/restarted 10-second visibility, `Copied` tooltip, correct clipboard contents, equivalent pasted-email search, Working Email and Azure AD Account support, Enter/Space activation, edit-mode preservation, stable layout, and no observed runtime errors.
- Fresh automated validation: Vitest, TypeScript typecheck, ESLint, production build, `npm run check:quality`, `npm run check:tauri`, and `git diff --check`.
- Known non-blocking items: accepted OpenSSL LNK4099 warning and two unrelated legacy static scripts expecting older Settings/borrow UI text.
- Commit/push: pending focused commit and push of this feature branch only; no merge into `main`.

## Start - Prepare Staff Kit v2.1.0 release - 2026-07-27

- Branch: `release/v2.1.0` from synchronized `main` at `22efe891a58fe068a57156da0e3dbe241669ce17`.
- Objective: prepare the v2.1.0 release branch and pull request without creating the tag or GitHub Release before merge.
- Scope: application semantic version metadata only; no dependency upgrades, database migration, product behavior changes, or machine-local tooling changes.
- Planned validation: package/Tauri version consistency, frontend checks, Tauri check/tests, diff checks, and explicit release-scope review.

## End - Prepare Staff Kit v2.1.0 release - 2026-07-27

- Result: prepared application version metadata for `2.1.0` on `release/v2.1.0`.
- Scope: updated `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`; no dependency versions or product behavior changed.
- Validation: Vitest 19/19 passed, `npm run check:quality` passed, `npm run test:tauri` passed with 92 tests, and `git diff --check` passed.
- Known warning: accepted OpenSSL `LNK4099` debug-symbol warning remains non-blocking.
- Tag/release: intentionally deferred until the release PR is merged into `main`.

## Start - Publish Staff Kit v2.1.0 Windows installers - 2026-07-27

- Branch: `main` at the immutable `v2.1.0` tag.
- Source verification: `HEAD`, `origin/main`, and `v2.1.0` all resolved to `3e44ac9795182259168a1597b3f9d079bbbff226`.
- Objective: build official Windows production bundles from the tagged release and attach them to the existing published GitHub Release.
- Scope: NSIS and MSI installer assets only; no tag movement, release recreation, product-code change, updater signing, or debug-binary upload.

## End - Publish Staff Kit v2.1.0 Windows installers - 2026-07-27

- Result: production Tauri build succeeded from `v2.1.0`; NSIS and MSI installers were produced and uploaded to the existing GitHub Release.
- Assets:
  - `Staff.Kit_2.1.0_x64-setup.exe` - 5,288,812 bytes - SHA-256 `9e8bbb904b6b146d60c43ca4a2889fb016ee5D898AEF2FA50A6278377B523897`.
  - `Staff.Kit_2.1.0_x64_en-US.msi` - 7,352,320 bytes - SHA-256 `b0793a1cb0e2fe97567b4e4699df1b6a3ec369a96e39820b96921fd598148a3b`.
- GitHub Release: upload succeeded; `v2.1.0` remained published and non-prerelease with both installer assets verified as uploaded. No updater artifacts or signatures were configured or uploaded.
- Validation: `npm run tauri:build` exited 0; release assets were verified with GitHub metadata and SHA-256 hashes; `main` remained clean and synchronized with `origin/main`.
- Known non-blocking warnings: accepted OpenSSL `ossl_static.pdb` LNK4099 linker warning; Tauri build-time LF/CRLF rewrite of `src-tauri/Cargo.toml` was restored without a tracked source change.
- Commit/push: no source commit or push was made; tag `v2.1.0` was not modified.

## Start - Implement EE List export in Edit mode - 2026-07-28

- Branch: `feature/ee-list-export` from clean, synchronized `main`.
- Objective: add admin-only CSV and real XLSX export for selected Employee List rows while preserving existing Edit-mode selection, pagination, visible-column ordering, and unsaved drafts.
- Scope: selection controls, filtered-result retrieval, export projection/downloads, focused tests, validation, and append-only log records; no commit, push, merge, or PR.
- XLSX dependency review: compared `write-excel-file`, `xlsx`, and `exceljs`; selected `write-excel-file` for browser Blob support, MIT license, approximately 1.8 MB unpacked size, and its minimal `fflate` dependency footprint.

## End - Implement EE List export in Edit mode - 2026-07-28

- Result: added admin-only `Export Selected (n)` CSV/XLSX controls, complete filtered-result selection, current-page unselection, clear-all selection, and selection persistence across pagination.
- Selection behavior: search/team/date filter changes clear selected IDs; pagination, sorting, grouping, and rows-per-page changes preserve them. `selectedMoveEmployeeIds` remains the only selection source of truth.
- Export behavior: retrieves the complete filtered result set in API-sized pages, exports only selected IDs and visible columns in persisted order, excludes hidden columns, overlays all in-memory drafts by employee ID including off-page drafts, emits empty cells instead of `-`, and does not save/reset/clear selection or show a success toast.
- Files: added `src/features/employees/employeeListExport.ts` and focused tests; updated `EmployeeView.tsx`, `useEmployeeState.ts`, `useTableEdit.ts`, `App.tsx`, `package.json`, and `package-lock.json`.
- XLSX choice: added `write-excel-file@4.1.1` exact, MIT licensed, browser-compatible, approximately 1.8 MB unpacked with one `fflate` dependency; `xlsx` was approximately 7.5 MB and `exceljs` approximately 21.8 MB with larger dependency trees. No new production audit finding was introduced; the existing audit report remains 11 vulnerabilities (2 low, 8 high, 1 critical).
- Validation: focused export tests 3/3 passed; full Vitest suite 22/22 passed across 5 files; `npm run typecheck`, focused ESLint, `npm run build`, `npm run check:quality`, and `git diff --check` passed.
- Known non-blocking warnings: Vite reported a bundle chunk over 500 kB; Git reported repository LF/CRLF normalization notices. No product behavior outside EE List export was intentionally changed.
- Commit/push: no commit or push made; branch remains `feature/ee-list-export`.

## End - Repair EE List Tauri XLSX export - 2026-07-28

- Root cause: the original export stopped at browser-only `Blob`/`URL.createObjectURL`/anchor download behavior, which is not a reliable file-save path in the Tauri WebView; XLSX errors were also not surfaced by the async handler.
- Fix: use `write-excel-file/universal` to generate the real workbook, open the existing Tauri save dialog, pass the selected path and workbook bytes through `write_export_file`, and write them with a narrowly scoped Rust command. CSV uses the same native save path.
- Lifecycle: the menu closes immediately before export begins; the awaited export promise continues independently. Failures now flow to the existing global error feedback through `getUserErrorMessage`; selection and drafts are preserved.
- Validation: focused export tests 9/9 passed, full Vitest 28/28 passed, typecheck, ESLint, build, `check:quality`, and `git diff --check` passed. The XLSX payload test verified ZIP signature bytes `50 4B 03 04`; no CSV content is renamed as XLSX.
- Tauri smoke: `npm run tauri:dev` reached and ran the desktop app after clearing a stale project Vite process on port 5173; only accepted OpenSSL LNK4099 warnings appeared during startup.
- Manual limitation: this headless Codex session could not perform the authenticated 9-row GUI interaction, select the save-dialog path, or open Excel, so final filename/size and Excel-open confirmation remain pending user GUI validation.

## End - Finalize EE List export controls - 2026-07-28

- Final UI: labels are `Select All (n)`, `Unselect Page`, `Clear All (n)`, and `Export (n)`.
- Dropdown behavior: Unselect and Export open upward, remain right-aligned, close on outside click or action, close each other when opened, reset their 3-second auto-close timers on pointer/focus/keyboard interaction, and clear timers on close/unmount.
- Export and selection behavior remains unchanged: CSV/XLSX output, filtered cross-page selection, visible-column ordering, hidden-column exclusion, unsaved draft overlays, and admin-only export access are preserved.
- Validation: focused export/timer tests, full Vitest, typecheck, ESLint, build, quality checks, npm audit, and diff checks recorded for this final implementation pass.

## End - Manual EE List export acceptance - 2026-07-28

- Manual GUI validation passed in the Tauri app for both CSV and XLSX export with the native save path after selecting exactly 9 employees.
- Confirmed the real XLSX workbook is created and usable, while CSV export remains functional; selection, visible-column ordering, hidden-column exclusion, Vietnamese text, empty cells, and unsaved draft overlays were preserved.
- Final implementation includes the Tauri-native export fix, immediate menu close without cancelling the async export, and existing error feedback on failure.
- Repository changes remain uncommitted at this checkpoint; no tag or release was modified.

## End/Release Checkpoint - SEC-001 Authentication - 2026-08-04

- SEC-001 backend Tauri command authorization completed and merged to `main` as squash commit `fff32be3200fcf6c7f368a744cd3cd93ea45fee8`.
- 75 registered Tauri commands reconciled: 9 public, 22 authenticated, 36 admin, 8 super_admin.
- Backend-owned memory-only session tokens (128-bit OsRng, idle 8h / absolute 12h, poison-recovery).
- Actor identity derived exclusively from verified `SessionContext`; no frontend-supplied role/account trusted.
- Session invalidation lifecycle: hooks on password change/reset, account deletion, role change, `reset_all_data`, database restore/move, and app shutdown — all fire after mutation success.
- Forced password reset: `forcePasswordReset` blocks normal app access until password is changed.
- Self-delete protection: `AUTH_CANNOT_DELETE_SELF` + last-super-admin rule + disabled Delete button.
- Role-aware startup: standard users never invoke admin/super_admin commands during initialization.
- Inline safe login errors: "Username or password is incorrect." in the Login card (not a global banner).
- `set_active_local_account` and `run_auto_backup_if_due` removed from IPC.
- Manual GUI validation passed: login, logout, restart, account switching, forced reset, standard/admin/super_admin behavior, self-delete blocked, Employee list loads, no global error for standard users.
- Automated totals: frontend 130/130, Rust 154/154, total 284 tests.
- Known non-blocking warnings: 1 ESLint (`useSettingsState` deps), 9 Cargo (test infrastructure + defense-in-depth).
- Merge was performed as one local squash commit and pushed directly because GitHub CLI (`gh`) was not authenticated.
- No SEC-002 (LAN authentication) work started.

## Start — SEC-002 Phase B repair — 2026-08-05

- Branch: `security/lan-borrow-authentication`; Phase A is already committed and pushed.
- Objective: repair Phase B LAN token lifecycle, fragment-based QR/browser authentication, pre-body bearer authentication, request identity fields, minimal submit responses, and regression coverage.
- Scope: `src-tauri/src/lan_auth.rs`, `src-tauri/src/lan_server.rs`, `src-tauri/src/lan_assets.rs`, `src-tauri/src/lib.rs`, focused tests, and this session log only.
- Out of scope: schema migration, Phase C business rules, `main`, `origin/main`, commit, push, merge, rebase, and amend.
- Planned validation: focused Rust/browser-contract tests followed by the complete requested frontend, Tauri, formatting, and diff gates.

## End — SEC-002 Phase B repair — 2026-08-05

- Result: repaired the end-to-end LAN flow without schema changes or Phase C business rules.
- Root causes fixed: no issued token at startup/admin boundary, query-token page auth, unauthenticated browser fetches, missing request identity fields, body extraction before auth, full DB record returned to LAN clients, and raw backend submit errors.
- Token lifecycle: admin-only issue/regenerate, revoke, and readiness IPC commands; token remains memory-only, is revoked on app close, and is never logged or returned in HTTP responses.
- Browser flow: QR uses `#t=`, page removes the fragment from visible history, API calls use Bearer auth, client session identity is stable in page memory, and each submission gets a fresh request ID.
- Validation: frontend unit tests 130/130; Rust tests 200/200; typecheck, lint, build, `check:quality`, `check:tauri`, edition-aware `rustfmt --check`, and `git diff --check` passed.
- Known warnings: one pre-existing React hook dependency warning, existing Vite chunk/dynamic-import warnings, and existing Rust dead-code warnings.
- Scope: no Phase C schema or business-rule files changed; no commit, push, merge, rebase, amend, or main/origin-main modification.

## Start — SEC-002 Phase C — 2026-08-05

- Branch: `security/lan-borrow-authentication`; starting from Phase B checkpoint `74aef2fb199b4d285705f73b4caabaafd7596141`.
- Objective: implement only Phase C schema migration, manual borrowers, authoritative employee/asset validation, atomic pending claims, and focused regression tests.
- Out of scope: Phase D, commit, push, merge, rebase, amend, and any modification to `main` or `origin/main`.

## End — SEC-002 Phase C — 2026-08-05

- Result: implemented non-destructive Phase C schema extensions, authoritative employee rules, lifecycle/loan validation, pending asset claims, immediate transactions, return attribution, and replay-after-business-validation ordering.
- Validation: frontend unit tests 130/130; Rust/Tauri tests 211/211; typecheck, lint, build, `check:quality`, `check:tauri`, rustfmt checks, and `git diff --check` passed.
- Warnings: existing React hook dependency warning, existing Vite dynamic-import/chunk-size warnings, and existing Rust dead-code warnings.
- Scope: no Phase D work, no commit/push, no merge/rebase/amend, no `main`/`origin/main` changes; incidental `db/column.rs` formatting was reverted.

## Start — SEC-002 Phase C correction — 2026-08-06

- Objective: remove the sentinel design, migrate populated borrow requests to a nullable employee foreign key, add cancellation and regression coverage, and preserve Phase B controls.
- Out of scope: Phase D, commit, push, merge, rebase, amend, and any modification to `main` or `origin/main`.

## End — SEC-002 Phase C correction — 2026-08-06

- Result: removed the sentinel design, added transactional nullable-FK migration coverage against populated legacy data, implemented atomic pending-request cancellation, and added rollback, concurrency, idempotency, and manual-return tests.
- Validation: frontend unit tests 130/130; Rust/Tauri tests 215/215; typecheck, lint, build, `check:quality`, `check:tauri`, changed-file rustfmt, and `git diff --check` passed.
- Warnings: existing React hook dependency warning, existing Vite dynamic-import/chunk-size warnings, and existing Rust dead-code warnings.
- Scope: no Phase D work, no commit/push, no merge/rebase/amend, and no `main`/`origin/main` changes.

## End — SEC-002 Phase C blocker fixes — 2026-08-06

- Result: cancellation now records the authenticated admin actor; Phase C migration is atomic with pragma restoration and rollback coverage; pending-claim uniqueness documentation matches multi-asset behavior.
- Validation: focused Phase C tests 14/14; full Rust/Tauri tests 216/216; frontend unit tests 130/130; typecheck, lint, build, quality, Tauri check, targeted rustfmt, and `git diff --check` passed.
- Warnings: pre-existing React hook dependency, Vite dynamic-import/chunk-size, and Rust dead-code warnings.
- Scope: no Phase D, commit, push, dependency/generated/database/secret changes, or `main`/`origin/main` changes.
## 2026-08-06 SEC-002 Phase D1 Start

- Implement only the audited D1 blockers: safe LAN browser DOM rendering, managed LAN server lifecycle, and admin pending-request cancellation UI.
- Preserve Phase A/B/C security and business rules; do not start D2 or commit/push.

## 2026-08-06 SEC-002 Phase D1 End

- Completed safe LAN browser rendering, managed LAN Start/Stop/status lifecycle, and pending-request cancellation UI.
- Validation passed: 15 frontend test files/133 tests and 227 Rust tests; no commit or push performed.

## 2026-08-06 SEC-002 Phase D2 Start

- Implement only the shared mobile Borrow/Return workflow hardening and pending queue coverage on top of committed D1.
- Preserve D1 lifecycle, Phase B authentication, and Phase C business rules; do not modify schema or start later Phase D work.

## 2026-08-06 SEC-002 Phase D2 End

- Hardened the shared LAN page with explicit search/submit loading guards, safe API error handling, and executable Borrow/Return flow coverage.
- Added admin queue coverage for pending Borrow/Return display, pending-only cancellation, loading state, and duplicate action prevention.
- Validation passed: 17 frontend test files/143 tests and 230 Rust tests; typecheck, lint, build, quality, Tauri check, rustfmt, and `git diff --check` passed.
- No commit or push performed; Phase D3 and later work were not started.
## 2026-08-07 SEC-002 PR blocker fixes — Start

- Fixing the managed LAN Stop → Start lifecycle race and aligning Phase C manual employee schema with the approved nullable email/team fields.
- Scope is limited to lifecycle synchronization, migration/persistence/DTO coverage, and focused regression tests. No commit or push in this session.

## 2026-08-07 SEC-002 PR blocker fixes — End

- Fixed the LAN lifecycle race by serializing Start/Stop with an async lifecycle mutex held through listener task termination; added deterministic concurrent restart coverage.
- Added nullable manual employee email/team fields to fresh schema, atomic legacy migration/rebuild, request persistence/load mappings, DTOs, validation, and manual-entry coverage.
- Validation: focused lifecycle test 1/1; focused Phase C tests 14/14; frontend unit tests 17 files/143 tests; full Rust/Tauri tests 231/231; typecheck, lint, build, quality, Tauri check, changed-file rustfmt, and `git diff --check` passed.
- Warnings: pre-existing React hook dependency, Vite dynamic-import/chunk-size, and Rust dead-code warnings. No commit/push, merge, Phase D3, dependency/generated/database/secret, or main/origin-main changes.
## 2026-08-07 SEC-002 deterministic lifecycle test — Start

- Strengthening only the Stop → Start concurrency regression test with test-scoped shutdown gating. No production lifecycle redesign, commit, push, or PR metadata changes.

## 2026-08-07 SEC-002 deterministic lifecycle test — End

- Result: made the Stop → Start regression deterministic with a test-only shutdown gate that proves Start remains blocked until Stop completes listener termination and socket release.
- Validation: focused lifecycle test 1/1; LAN lifecycle tests 32/32; focused Phase C tests 14/14; frontend unit tests 17 files/143 tests; full Rust/Tauri tests 231/231; typecheck, lint, build, quality, Tauri check, rustfmt, and `git diff --check` passed.
- Warnings: pre-existing React hook dependency, Vite dynamic-import/chunk-size, and Rust dead-code warnings. No production lifecycle redesign, commit/push, schema or manual-field changes, dependency/generated/database/secret changes, or main/origin-main changes.

## 2026-08-07 SEC-002 deterministic lifecycle test hardening — Start

- Replace the remaining timing-based Stop → Start regression-test proof with test-only listener readiness and ordered lifecycle signals. No production lifecycle redesign, commit, push, or PR metadata changes.

## 2026-08-07 SEC-002 deterministic lifecycle test hardening — End

- Result: replaced timing-based Stop → Start proof with test-only listener-ready, shutdown, lifecycle-acquisition, termination, and completion signals plus ordered event assertions. No release lifecycle behavior changed.
- Validation: focused lifecycle test 1/1; LAN lifecycle tests 32/32; focused Phase C tests 14/14; full Rust/Tauri tests 231/231; frontend unit tests 17 files/143 tests; typecheck, lint, build, quality, Tauri check, changed-file rustfmt, and `git diff --check` passed.
- Warnings: pre-existing React hook dependency, Vite dynamic-import/chunk-size, and Rust dead-code warnings. No schema/manual-identity changes, dependency/generated/database/secret changes, commit/push, or main/origin-main changes.

## 2026-08-10 release blocker fixes — Start

- Objective: migrate restored databases before reporting restore success, strengthen Borrow backup round-trip coverage, and repair maintained LAN UI supplemental checks.
- Scope: restore/migration helpers and tests, Borrow backup/restore tests, and supplemental scripts; no version bump, tag, release, UI redesign, or Borrow business-rule changes.

## 2026-08-10 release blocker fixes — End

- Result: restored database-file and history restores now apply the canonical migrations before success; Borrow backup coverage includes policy, confirmation metadata, exact signature bytes, identity snapshots, and request items; maintained supplemental scripts now match the current LAN UI contracts.
- Validation: focused backup tests 4/4; focused Borrow backup test 1/1; supplemental scripts passed; frontend unit tests 22 files/197 tests; full Rust/Tauri tests 263/263; Tauri check, typecheck, lint, build, quality, `git diff --check`, and `tauri:build` passed.
- Warnings: pre-existing React hook dependency, Vite dynamic-import/chunk-size, Tauri identifier, and Rust dead-code/unused Phase-2 API warnings. No version bump, tag, release, commit, push, generated/local/sensitive files, or unrelated business/security changes.

## 2026-08-10 Staff Kit 2.2.0-rc.1 build — Start

- Objective: prepare a local Windows release-candidate build from clean main at b6a77dc2f29bfe37cdf2877c145664d4ea43a0c1.
- Version strategy: app/package/Tauri/Cargo use 2.2.0-rc.1; WiX MSI uses the valid numeric product version 2.2.0.1; NSIS retains the SemVer prerelease identity.
- Scope: version metadata, generated package lock metadata, WiX bundle version override, and release validation only; no Borrow/Restore, schema, mapping, or business-logic changes.

## 2026-08-10 Staff Kit 2.2.0-rc.1 build — End

- Result: local Windows RC artifacts built successfully with app/package/Tauri/Cargo version 2.2.0-rc.1 and WiX MSI ProductVersion 2.2.0.1.
- Validation: unit tests 197/197; Rust/Tauri tests 263/263; Tauri check, typecheck, lint, build, quality, `git diff --check`, and `tauri:build` passed.
- Artifacts: `Staff Kit_2.2.0-rc.1_x64_en-US.msi` and `Staff Kit_2.2.0-rc.1_x64-setup.exe`; stale generated 2.1.0 bundle files removed. No commit, push, tag, release, or version beyond the RC preparation.
- Warnings: existing React hook dependency, Vite dynamic-import/chunk-size, Tauri identifier, and Rust dead-code/unused Phase-2 API warnings.

## 2026-08-18 new-machine environment bootstrap � Start

- Objective: prepare and verify the Zonnguyen510 Windows development environment for continued Staff Kit work from E:\Staff_Kit, preserving local repository state and making no business-logic changes.
- Scope: repository synchronization, required tool/dependency checks, baseline validation, Tauri runtime and database-path verification, and initial adman credential-rule inspection. No commit, push, reset, production-data changes, or credential changes.

## 2026-08-18 new-machine environment bootstrap � End

- Result: repository remains on main at 7881cf7296e6f3fe231a1db3833a7f0b30c5c92b, synchronized with origin/main at 0/0. Existing tracked and untracked local work was preserved; no reset, commit, push, import, restore, or credential change was performed.
- Environment: Node/npm, Rust/Cargo MSVC, Visual Studio Build Tools C++, WebView2, Perl, Codex, RTK, Python 3.13, and Headroom were prepared. Codex static tooling audit reported STATIC READY.
- Validation: frontend unit tests 24 files/200 tests, typecheck, lint, build, supplemental checks, Rust/Tauri tests 283/283, check:tauri, check:quality, and git diff --check passed. Tauri dev Vite reached port 5173 and the direct dev executable opened a responding Staff Kit window against the existing database path.
- Blockers: GitHub CLI remains unauthenticated after the browser-login attempt; complete gh auth login interactively for quockhanh2376. Source inspection found adman is currently seeded as admin, not super_admin; its initial password is defined in source and first-login forced reset is enabled. No product logic was changed.

## 2026-08-22 Employee unified-import sheet routing fix - Start

- Objective: propagate the detector-selected Employee worksheet through inspection, preview, validation, and approval preparation without changing Asset routing or schema.
- Scope: Employee import state/API/backend sheet selection, focused regression coverage, and validation only. No commit, push, runtime DB, archive, audit-folder, or worktree changes.

## 2026-08-22 Employee unified-import sheet routing fix - End

- Result: detector-selected Employee sheet names now flow through inspection, preview, and explicit approval payloads; missing or invalid selected sheets fail clearly without fallback. Asset routing, schema, dependencies, and existing launchers were unchanged.
- Validation: focused frontend 2/2; frontend unit 28 files/215 tests; detector 14/14; selected-sheet import guards 2/2; full Rust/Tauri 286/286; typecheck, lint, build, quality, and git diff --check passed.
- Safety: no runtime DB, production data, archive/audit folder, linked worktree, commit, or push was touched. Existing warnings remain unchanged.

## 2026-08-22 Employee multi-sheet fix finalization - Start

- Objective: final scope review, validation, focused commit, and push of the completed Employee selected-sheet fix on main.
- Scope: reviewed seven tracked files only; preserved audit docs, plan, help, archive/audit folders, and linked worktrees. No runtime or production DB writes.

## 2026-08-22 Employee multi-sheet fix finalization - End

- Result: final diff review and all required validation gates passed; commit and push remain the only integration actions in this finalization step.
- Validation: focused frontend 2/2; frontend unit 28 files/215 tests; import guards 2/2; detector 14/14; full Rust/Tauri 286/286; typecheck, lint, build, quality, and git diff --check passed.
- Safety: no production/runtime DB write, no force-push, no archive/audit/worktree modification, and no unrelated file staged.

## 2026-08-23 Asset unified-import detector contract fix - Start

- Objective: align the frontend unified-import detector contract with the backend 	ype field so valid Asset detection can continue to preview.
- Scope: shared TypeScript detector type, unified-import routing consumers/tests, and validation only. No runtime DB, production data, archive/audit folder, or preserved untracked file changes.


## 2026-08-23 Asset unified-import detector contract fix - End

- Result: frontend now consumes the backend 	ype detector field consistently; valid Asset detection enables preview routing and preserves subtype/sheet selection.
- Validation: focused frontend 14/14; frontend unit 217/217; Rust/Tauri 290/290; quality gate, typecheck, lint, build, and git diff --check passed. Existing warnings remain unchanged.
- Delivery: commit 38f2fc16c6cb7bc61984692fb1bdba75f48089b9 pushed to origin/main. Runtime/production DB was not accessed or modified.

## 2026-08-23 real-data Borrow/Return acceptance + LAN host audit - Start

- Objective: record the completed Unified Employee Import -> Onboarding -> Unified Asset Import -> Borrow -> duplicate protection -> Return milestone and audit stale LAN host behavior.
- Scope: canonical session log update and read-only LAN/IP implementation review only. No LAN configuration, business data, auth, import, or code changes.

## 2026-08-23 real-data Borrow/Return acceptance + LAN host audit - End

- Result: acceptance PASS for ASWVN1325 with VNLAP504; borrow BR-ZXVYZGSYDN and return BR-RVODA3JT4B completed. Final counts: employees 431, onboarding 7, assets 37, active loans 30, returned loans 2; integrity OK and FK violations 0. Canonical assignment required no legacy Employee computer field.
- LAN audit: host is persisted in runtime settings and discovery updates the UI/input only; explicit admin save is required. Server binds 0.0.0.0 while QR generation uses the saved host, so stale values can break remote QR/browser access but do not change the auth/token boundary. Classification: MEDIUM - BORROW/LAN RELIABILITY. No code or runtime-data changes.

## 2026-08-23 LAN host refresh reliability fix - Start

- Objective: persist a valid freshly detected LAN host before QR/Borrow URL use while preserving token security, port settings, and failure behavior.
- Scope: focused settings-state regression coverage and minimal LAN host persistence fix. No business-data, auth, import, schema, dependency, archive, audit-folder, or worktree changes.

## 2026-08-23 LAN host refresh reliability fix - End

- Result: valid detected LAN hosts now persist through the existing settings API before QR construction; failed detection preserves the saved host.
- Validation: focused LAN 44/44; frontend 218/218; Rust/Tauri 290/290; supplemental LAN checks PASS; quality gate PASS; no business DB writes.
- Commit/push: 71010318abcab84ac2787a32c3724484706b910e pushed to origin/main.

## 2026-08-23 Borrow/Return signer-name autofill - Start

- Scope: initialize the shared LAN Borrow/Return confirmation typed-name field from the canonical employee full name after Staff ID lookup; preserve confirmation-only edits and leave Employee, loan, auth, LAN security, and schema behavior unchanged.

## 2026-08-23 Borrow/Return signer-name autofill - End

- Result: shared LAN Borrow/Return confirmation now looks up the canonical Employee full name by Staff ID and initializes the confirmation typed-name field; edits remain confirmation-local and employee changes refresh the default.
- Validation: focused LAN lookup/autofill tests PASS; LAN server 41/41; LAN page 14/14; Borrow frontend 44/44; full frontend 218/218; Rust/Tauri 292/292; quality gate PASS; no business DB writes.
- Commit/push: dbd9fb71820fd0e177da38d8c9915080f6d1fa6f pushed to origin/main.

## 2026-08-24 LAN signer autofill runtime-ordering investigation - Start

- Finding: the prior browser test set Staff ID and blurred only after asset auto-selection, bypassing the real mobile order. The auto-selected asset branch did not invoke employee lookup; scope is limited to invoking the existing read-only lookup there and preserving confirmation-local edits across re-renders.

## 2026-08-24 LAN signer autofill runtime-ordering investigation - End

- Root cause: the auto-selected single-asset path reset confirmation state but did not invoke the existing employee lookup; the prior test populated Staff ID only after that path and therefore missed the defect.
- Fix: auto-selected and manually selected assets now trigger lookup; confirmation-local typed-name edits survive asset re-renders and delayed lookup completion. No business DB writes.
- Validation: focused mobile-order Borrow/Return test PASS; Rust/Tauri 292/292; frontend 218/218; quality gate PASS; commit 102b371cdf35037d87fe41329a8b42b34594fab9 pushed and synchronized.
- Manual phone verification: unavailable in this environment; no LAN listener was running.

## 2026-08-24 compact signature clear action - Start

- Scope: replace the shared LAN Borrow/Return signature-area button with an inline touch-friendly icon that clears only signature evidence and preserves typed-name/agreement state.

## 2026-08-24 compact signature clear action - End

- Result: shared Borrow/Return LAN confirmation now uses an inline top-right trash icon inside the signature box, hidden until signature evidence exists. It clears only signature evidence; typed-name and Borrow agreement state remain unchanged.
- Validation: focused Borrow/Return signature test PASS; LAN page 15/15; Rust/Tauri 293/293; frontend 218/218; quality gate PASS; no business DB writes.
- Commit/push: 93f8a43fc3c610bdab03228bc2a4ace9bab6c8e5 pushed and synchronized with origin/main.

## 2026-08-24 Staff_Kit session close summary

- Unified Data Import completed and hardened: Employee selected-sheet propagation fixed; detector restricted to admins.
- Canonical Asset assignment boundary enforced: active `asset_loans` is authoritative. Real acceptance passed for `Onboarding.xlsx` (5 employees processed; employees 431; onboarding 7) and `AssetList.xlsx` (6 serialized assets; assets 37).
- Real Borrow/Return acceptance passed for `ASWVN1325` with `VNLAP504`; canonical lifecycle verified. LAN stale-host reliability fixed; signer typed-name autofill was added and real-phone verified; single-asset mobile ordering was fixed; the Clear Signature button became an inline trash action.
- Final automated baseline: frontend 218/218; Rust/Tauri 293/293; quality gates PASS.
## 2026-08-25 Staff Kit 2.2.0-rc.2 release candidate - Start

- Objective: prepare the Windows release candidate from synchronized `main` at `d3df2a45db9e98e150e54d4d9bccc691033eed2c` for controlled real-world testing.
- Scope: authoritative version metadata, release validation/build, copy-based data-safety checks, concise release notes, and release commit/tag delivery. Preserved untracked audit docs, plan, and `help` remain untouched; no production-data transaction is planned.
## 2026-08-25 Staff Kit 2.2.0-rc.2 release candidate - End (Blocked)

- RC metadata and Windows bundles were prepared locally as `2.2.0-rc.2`; artifacts: `Staff Kit_2.2.0-rc.2_x64_en-US.msi`, `Staff Kit_2.2.0-rc.2_x64-setup.exe`, and `staff-kit.exe`.
- Validation: frontend 218/218; Rust/Tauri 293/293; typecheck, lint, build, `check:tauri`, `check:quality`, and `git diff --check` passed. Existing warnings remain unchanged.
- Data safety: a cold prelaunch copy remained readable and intact with employees 431, assets 37, active loans 30, returned loans 2, integrity OK, and zero FK violations. Packaged startup changed only live SQLite `schema_version` 364 to 372; `user_version` remained 0 and business counts stayed unchanged.
- Blocker: Windows Tauri path resolution ignored the temporary `LOCALAPPDATA` redirect, so the packaged smoke launch touched the live DB. No import, Borrow/Return transaction, or other business workflow was executed. Commit/tag/push deferred; next step is an approved isolated packaged-build smoke procedure.

## 2026-08-25 Staff Kit 2.2.0-rc.2 release candidate - Final

- RC.2 prepared; Zon manual packaged UI smoke PASS using the isolated `db_settings.json` custom-folder procedure. Live DB remained untouched and the isolation sidecar was removed afterward.
- Release commit: `chore(release): prepare 2.2.0-rc.2`; tag: `v2.2.0-rc.2`. Next step: Zon controlled real-world RC.2 testing. No credentials recorded.

## 2026-08-26 idempotent Asset re-import - Start

- Objective: classify exact existing Assets as Existing/Skip, preserve true code/serial conflicts as Error/Conflict, and approve only genuinely new Assets.
- Acceptance source: local `00_ExSource/AssetList.xlsx` with six existing Lenovo rows and four new Samsung rows; no production DB writes during automated validation.
- Scope: Asset import preview/commit and regression coverage only; preserve Employee import, canonical Asset identity, uniqueness constraints, active loan/status state, and existing uncommitted changes in the main checkout.

## 2026-08-26 idempotent Asset re-import - End

- Implemented canonical asset-code/serial classification: exact compatible existing rows are Existing/Skip, genuine rows are New, and cross-identity or duplicate conflicts remain Error/Conflict.
- Approve revalidates inside the transaction and inserts only New rows; existing Asset metadata and active loan/status state are not updated.
- Validation: Asset import focused 39/39; frontend unit 29 files / 220 tests; Tauri full 298/298; quality gate passed (lint, typecheck, build, cargo check).
- Acceptance workbook was read-only parsed and exercised against SQLite in-memory: 10 total, 6 Existing/Skip, 4 New, 0 Error, 4 inserts, repeat 0 inserts.
- No production DB writes were performed.
## 2026-08-26 Asset re-import approval gating follow-up - Start

- Objective: block serialized Asset approval whenever any blocking Error/Conflict row remains, while preserving Existing/Skip and New behavior.
- Scope: focused frontend approval-gating regression coverage, smallest state fix, and isolated real-workbook acceptance using `H Phone`; do not merge or push.
- Safety: preserve the two pre-existing modified generated schema files in this worktree; read `E:\Staff_Kit\00_ExSource\AssetList.xlsx` only and never write the live DB.
## 2026-08-26 Asset re-import approval gating follow-up - End

- Result: serialized approval now requires at least one New row and zero blocking Error/Conflict rows; Existing/Skip rows remain non-blocking.
- Validation: focused approval 4/4; focused Asset import 39/39; full frontend 224/224; full Rust/Tauri 298/298; quality gate PASS; `git diff --check` PASS.
- Real workbook acceptance: `AssetList.xlsx` read-only with `H Phone` setup in isolated SQLite; preview 10 total / 6 Existing-Skip / 4 New / 0 Error-Conflict; first import inserted 4 with zero existing writes and preserved active state; second import 10 Existing-Skip / 0 New / 0 writes.
- No production DB writes; no merge or push.
## 2026-08-26 Asset re-import approval gating review fix - Start

- Objective: preserve quantity-import partial approval while keeping serialized approval blocked by any Error/Conflict rows, per independent review.
- Scope: add a mixed quantity frontend regression, scope the approval gate by import mode, rerun all required checks, and request re-review; do not merge or push.
- Safety: preserve the two pre-existing modified generated schema files in this worktree; no production database writes.

## 2026-08-26 Asset re-import approval gating review fix - End

- Result: serialized previews block approval when any Error/Conflict row remains; quantity previews still allow approval when valid rows coexist with skippable invalid rows.
- Review: initial Important finding resolved; independent re-review returned no findings and marked the slice ready for QEEN revalidation.
- Validation: approval regression 5/5; frontend unit 225/225; Rust/Tauri 298/298; quality gate PASS; git diff --check PASS.
- No production DB writes; no merge or push.
## 2026-08-26 — Session Summary

### Completed
- Asset re-import idempotency implemented, Qeen-approved, merged, and pushed.
- Real workbook acceptance: 10 total, 6 Existing/Skip, 4 New H Phone, 0 Error.
- Repeat Asset import verified zero-write.
- Post-merge validation PASS.
- LAN input placeholder polish committed/pushed.

### Current Baseline
- HEAD: 8066f899f8186dbac813b47fc6a9d2a457eb7142
- Frontend tests: 225/225
- Rust/Tauri tests: 298/298
- main == origin/main
- ahead/behind: 0/0

### Product / Business Rules Confirmed
- Existing Asset rows are non-blocking Existing/Skip.
- True Asset identity conflicts remain blocking errors.
- Serialized Approve requires New rows > 0 and zero Error/Conflict rows.
- Repeat Asset import must be idempotent and zero-write.
- daily_log.md is the canonical handoff file for the next session.

### Open Items / Next Step
- Preserve existing untracked audit/docs/plan/help files.
- Next product task: TBD by Zon/Cheki.

## 2026-09-12 Tauri startup black-screen fix — Start

- Objective: eliminate the persistent black main window observed in `npm run tauri:dev` after React has rendered successfully in the WebView DOM.
- Scope: delay main-window visibility until the frontend signals its first rendered frame; add focused regression coverage and verify Tauri startup. No database, import, auth, LAN, or release changes.

## 2026-09-12 Tauri startup black-screen fix — End

- Root cause: the WebView contained a fully rendered React Login DOM without JavaScript errors, but the native main window had been shown from Rust `setup` before the first WebView frame; affected Windows WebView2 rendering remained black.
- Result: the main window remains hidden at native setup. React schedules one public `show_main_window` IPC call on its next animation frame; the backend resolves and shows only the `main` window.
- Validation: focused frontend tests 20/20 passed; `npm run check:tauri` passed; `npm run check:quality` passed; manual `npm run tauri:dev` opened and visibly rendered the Login page without DevTools or a black client area.
- Existing warnings unchanged: React hook dependency, Vite dynamic-import/chunk size, Rust dead-code, and bundled OpenSSL PDB linker warnings. No production DB, auth data, imports, or LAN state was modified.

## 2026-09-12 Tauri startup visibility follow-up — Start

- Root cause follow-up: the first implementation used `requestAnimationFrame` while the native WebView remained hidden. On Windows WebView2, a hidden document can pause animation frames, leaving the window hidden indefinitely.
- Objective: use an execution mechanism that runs while hidden, retain a short first-paint delay, and verify the actual `E:\Staff_Kit` launch path.

## 2026-09-12 Tauri startup visibility follow-up — End

- Result: replaced the hidden-WebView `requestAnimationFrame` callback with a 50 ms timer before invoking `show_main_window`; timers continue while the WebView is hidden, so the native window can leave its hidden state.
- Validation before root-launch smoke test: focused frontend tests 20/20 and `npm run typecheck` passed.

## 2026-09-12 Tauri startup visibility follow-up — End

- Final root-launch smoke test: `npm run tauri:dev` from `E:\Staff_Kit` built `E:\Staff_Kit\src-tauri` and started `staff-kit.exe`; the main window reported title `Staff Kit`, `Visible: True`, responded to Windows, and had the configured full-size rectangle.
- The visibility handoff now runs natively 500 ms after setup, avoiding both the too-early first paint and hidden-WebView scheduler deadlock. The frontend IPC startup bridge was removed.

## 2026-09-20 Borrow/LAN canonical identity display mismatch — Start

- Objective: make known Staff ID lookups populate the same canonical employee name in the top form and Borrow/Return confirmation signer, while preserving manual signer edits locally and preventing stale delayed lookup results.
- Confirmed root cause: the LAN page resolved `/api/employee` only into signer state; `#full-name` was never updated after Staff ID changes and could retain stale/manual text. No database writes are part of this change.

## 2026-09-20 Borrow/LAN canonical identity display mismatch — End

- Fix: known employee lookup now clears stale top-name state on Staff ID changes, populates `#full-name` from the accepted canonical lookup response, and makes that field read-only for known employees. Unknown/manual IDs remain editable; signer edits remain confirmation-local.
- Focused regression: LAN Borrow/Return identity flow passed, including `aswvn1325` normalization, delayed A→B lookup, asset auto-selection, signer-only edit, and canonical top/signer refresh.
- Validation: LAN-focused Rust tests 15/15; frontend unit tests 225/225; full Rust/Tauri tests 298/298; build, typecheck, lint, and cargo check passed with existing warnings. No production DB writes.

## 2026-09-20 LAN top-name editability regression — Start

- Objective: retain canonical employee lookup as the initial Full Name default while preserving user edits through lookup completion, asset selection, signer interaction, and unrelated renders; reset only when Staff ID changes.
- Root cause: the prior canonical-display fix set `#full-name.readOnly = true` and had no separate dirty flag for top-name edits.

## 2026-09-20 LAN top-name editability regression — End

- Fix: added independent `fullNameDirty` tracking. Canonical lookup provides the editable default; lookup completion and asset-triggered re-resolution preserve dirty top-name edits, while a different Staff ID clears and refreshes the default. Signer state remains independent.
- Validation: focused Borrow/Return regression 1/1; frontend unit 225/225; full Rust/Tauri 298/298; quality gate passed. No live DB writes.
# 2026-09-20 LAN signature-area UI refinement — Start

- Objective: compact the Borrow/Return signature area while preserving the approved canonical employee-name and editable signer behavior.
- Scope: inline editable signer name, compact clear action, updated heading, larger signature canvas; no database writes.
- Approval: Qeen-approved logic baseline confirmed by the user.

## 2026-09-20 LAN signature-area UI refinement — End

- Implemented inline editable signer name, `Sign here !` heading, taller canvas, compact red X clear action, and removed the lower typed-name section.
- Validation: focused LAN asset tests, all 299 Rust/Tauri tests, 225 frontend unit tests, and `npm run check:quality` passed; no live business DB writes.

## 2026-09-20 LAN signature Full Name mirroring — Start

- Objective: make the signature-box name mirror the current editable Full Name field exactly for Borrow and Return.
- Scope: remove independent signer display state, preserve canonical Staff ID defaults and submit behavior, retain compact signature UI and signature-only red X clearing.
- Safety: work isolated on `fix/borrow-fullname-mirror`; no live DB writes, no Employee master/loan/auth changes, and no merge or push.

## 2026-09-20 LAN signature Full Name mirroring — End

- Root cause fixed: independent `typedName` signer state was removed; the read-only inline signature name now mirrors `#full-name` immediately, including lookup defaults, edits, clears, Staff ID changes, and Borrow/Return mode changes.
- Red X remains signature-only, with 15px glyph and 4px top/right inset; Employee master, loan identity, auth, and backend validation were not changed.
- Validation: focused LAN assets 17/17; Full Name mirror and compact UI tests passed; frontend unit 225/225; full Rust/Tauri 300/300; quality gate passed; `git diff --check` passed; no live DB writes.

## 2026-09-21 LAN runtime identity and VNLAP369 audit — Start

- Objective: remove the reported employee-name fixture from runtime/test paths, preserve direct Full Name mirroring, and audit the missing VNLAP369 search result without touching the live database.
- Read-only audit: SQLCipher query against the local app database found no `assets.asset_code = 'VNLAP369'`; search code preserves case-insensitive partial matching and Borrow/Return status eligibility.
- Scope: update only stale fixture assertions and add focused VNLAP369 eligible-search coverage; no Employee master, loan, auth, or signature-X behavior changes.

## 2026-09-21 LAN runtime identity and VNLAP369 audit — End

- Root cause: the old employee string existed only in LAN/import test fixtures; no production LAN page hard-coded it. It was replaced, and the signature mirror remains sourced only from the current Full Name field.
- Read-only DB result: encrypted local database query found no `VNLAP369` asset row. The search path itself remains correct for exact/partial/case-insensitive matching and Borrow/Return status eligibility; no production search workaround was added.
- Validation: focused LAN assets 17/17, VNLAP369 asset search 1/1, import fixture 1/1, frontend 225/225, full Rust/Tauri 300/300, `npm run check:quality` passed, `git diff --check` passed; no live DB writes.

## 2026-09-21 LAN empty Full Name autofill and signature sizing — Start

- Objective: prevent focus-time browser autofill from injecting a stale employee name, preserve direct Full Name mirroring, and refine the signature box/X without changing Borrow/Return identity rules.
- Root cause: Full Name and read-only signature inputs still advertised `autocomplete="name"`; the old employee string was absent from the e9a5 runtime but could be supplied by browser autofill state.
- Scope: set both inputs to `autocomplete="off"`, add empty-focus regression, make the signature canvas ~20% taller and wider toward its container edges, and use a tighter red X in a green circle.

## 2026-09-21 LAN empty Full Name autofill and signature sizing — End

- Result: Full Name and read-only signature mirror no longer advertise browser name autofill; empty focus remains empty and the placeholder stays guidance-only. Borrow/Return mirroring remains direct and independent of any signer source.
- Result: signature canvas uses a 1.45:1 aspect ratio with 166px minimum height and a -14px inline margin; the clear action is a 36px uppercase red X inside a green circular border at 1px top/right and still clears signature evidence only.
- Validation: focused LAN UI/focus/mirror/clear tests 4/4; frontend 225/225; full Rust/Tauri 301/301; `npm run check:quality` passed; `git diff --check` passed; no live DB writes.

## 2026-09-20 LAN Full Name placeholder and signature X refinement — Start

- Objective: use the exact Full Name guidance placeholder and refine the compact signature clear control without changing Full Name mirroring.
- Scope: placeholder-only guidance, read-only in-box mirror, uppercase green circular X about 10% larger and closer to the top-right; Borrow and Return remain consistent.
- Safety: feature branch only; no live DB writes, Employee master/loan/auth changes, merge, or push.

## 2026-09-20 LAN Full Name placeholder and signature X refinement — End

- Result: Full Name now uses `Your name here ...` as guidance only; the read-only signature-box mirror remains driven by the current Full Name value for Borrow and Return.
- Result: clear control is uppercase `X`, 44px circular green bordered, positioned 2px from the top/right; its existing handler still clears signature evidence only.
- Validation: focused LAN assets 17/17; frontend unit 225/225; full Rust/Tauri 300/300; `npm run check:quality` passed; `git diff --check` passed; no live DB writes.

## 2026-09-21 LAN legacy-name source/cache audit — Start

- Objective: trace why legacy employee names can still appear after the approved empty-name/autofill fix, scanning tracked/untracked source, generated frontend/dev artifacts, fixtures, and caches from `0dda589`.
- Scope: remove only confirmed runtime/default/test/cache injectors; preserve Employee master semantics, user docs/help, and all untracked files outside safe project caches; add regression scans for both legacy variants plus Borrow/Return mirror and signature-X behavior.
- Safety: dedicated feature branch/worktree; no live DB writes, merge, or push.

## 2026-09-21 LAN legacy-name source/cache audit — End

- Root cause: `E:\Staff_Kit` was launching `main`/older worktrees instead of approved `0dda589`; those stale fixtures still contained the `Cao Tran [Quang] Duc` variant in `src-tauri/src/lan_assets.rs` and `src-tauri/src/db/import.rs`. The approved runtime branch contained neither legacy variant and no Employee/DB lookup injected them.
- Regression: added `runtime_page_has_no_legacy_employee_name_literals`; the existing empty-Full-Name focus, Borrow/Return mirror, and signature-only X tests remain green.
- Exact scan: both legacy variants (`Cao Tran [Van|Quang] Duc`) = 0 in the feature worktree after the change. No matching strings were found in `dist` or `node_modules/.vite` before cleanup.
- Cache cleanup: removed only `E:\Staff_Kit\dist` and `E:\Staff_Kit\node_modules\.vite`; preserved `help` and untracked docs; restored generated Tauri schemas after checks.
- Validation: focused LAN tests 19/19; frontend unit tests 225/225; `npm run check:quality` passed with existing warnings; no live DB writes, merge, or push.

## 2026-10-13 EE List MSSQL preview sync — Start

- Objective: replace immediate Employee List Update writes with a read-only remote preview, Staff Code diff, explicit approval, and local EE List-only apply.
- User-approved Query B: fixed SELECT; `INNER JOIN AspNetUsers`; `Resigned = 0`; no TOP row cap; dedupe by Code; fields Code, Name, NickName, COALESCE(WorkEmail, WorkEmail2), AzureAD. Staff without AspNetUsers are intentionally excluded. Missing-from-source is report-only; non-EE-List employees remain untouched.
- Audit findings: current Update calls test connection then `import_mssql_staff`, which queries then immediately upserts SQLite; SQL A uses INNER JOIN AspNetUsers and TOP (1000); update upsert matches employee IDs across every group.
- Evidence limit: no MSSQL host/user/password configured in Process/User/Machine environment; no source DB connection or count comparison performed. No DB writes.
- Safety: dedicated feature worktree `feat/employee-mssql-preview-sync`; preserve root untracked files; no merge or push before Qeen approval.
- Design decision update (2026-10-13): user explicitly selected Query B with INNER JOIN; staff without AspNetUsers are intentionally excluded. The earlier proposal to LEFT JOIN was replaced before implementation. User requested spec correction followed by implementation.
- Implementation update: added fixed Query B preview/apply commands, fail-closed source-login permission preflight, read-only encrypted SQLite preview, Staff Code classification, stale-snapshot checks, and explicit approval UI. Preview rows are process-memory only; no source or local business DB was touched.
- Verification so far: focused frontend 9/9, full frontend 228/228, focused Rust MSSQL 9/9, read-only SQLite guard 1/1, full Rust/Tauri 308/308, `npm run check:quality` passed. Direct source counts remain unverified because no MSSQL environment keys are configured.

## 2026-10-13 MSSQL source permission gate review — Start

- Objective: close Qeen's finding that the fail-closed MSSQL permission preflight omitted escalation permissions, then request revalidation of the exact new commit.
- Safety: feature branch only; do not connect to or write the live source DB; direct Query B counts remain pending configured read-only credentials.

- Review finding: the first follow-up gate still omitted login/user IMPERSONATE securables and allowed REFERENCES despite the strict read-only claim. Added effective permission enumeration for server logins and database users/roles; removed REFERENCES from every read-only allowlist.
- Verification update: read-only permission gate regression 1/1; full Rust/Tauri 308/308; `npm run check:quality` passed with existing lint/build/Rust warnings; no live DB touched. Awaiting Qeen revalidation and configured source credentials for direct Query B comparison.

## 2026-10-13 MSSQL source permission gate review — End

- Result: Qeen's static review of the latest implementation found no remaining escalation issue; the gate rejects permissions beyond explicit read-only allowlists, including impersonation of database and server principals. Query B and the preview/approval lifecycle remain intact.
- Validation: focused permission-gate test 1/1; full Rust/Tauri 308/308; `npm run check:quality` passed; `git diff --check` passed. Environment checks found no source credentials in Process/User/Machine; no connection or DB writes occurred. Direct Query B counts/mapping and runtime SQL-version behavior remain pending the user's configured read-only login and Qeen's direct audit.

## 2026-10-13 Remote EE Sync secure settings — Start

- Objective: update the approved EE List MSSQL sync spec and implement secure Remote EE Sync settings with verified read-only access, fixed Query B, and preview-before-apply.
- Security: source permissions must be verified fail-closed including inherited roles; secrets stay out of logs/source/plaintext settings; no remote or live local DB writes during validation.
- Scope: isolated feature worktree only; preserve all existing untracked files; no merge before Qeen approval.

## 2026-10-13 Remote EE Sync secure settings — End

- Updated the EE List sync spec and implementation plan for the approved Query B, Remote EE Sync settings, secure password storage, and fail-closed permission verification.
- Implemented the Settings profile, Windows Credential Manager password storage, full-profile-only environment override, PASS/FAIL Test Connection, configured-database Query B, and removed arbitrary MSSQL import/query IPC and UI.
- Verification: `npm run check:quality` passed; frontend unit tests 231/231; Rust/Tauri tests 311/311; `git diff --check` passed. Existing lint/build/Rust warnings remain.
- DB safety: source read-only credentials are not configured in Process/User/Machine environment. No source or live Staff Kit DB connection/write was made. Direct Query B row/mapping comparison is blocked pending user configuration and Qeen verification.
- Commit remains on the feature branch; no push or merge. Qeen independent review pending exact commit.

## 2026-10-13 Remote EE Sync Qeen review follow-up — Start

- Objective: close Qeen's finding that the read-only gate omitted role-specific server securable permissions.
- Approach: inspect effective ANY permission on each visible server-role securable; treat NULL/unverifiable results as write-capable. SQL Server documentation was checked because `fn_my_permissions` does not document SERVER ROLE as a supported class.
- No source/live DB connection or writes; environment variables remain unconfigured.

## 2026-10-13 Remote EE Sync Qeen review follow-up — End

- Added a regression assertion and server-role permission gate using `HAS_PERMS_BY_NAME(role, 'SERVER ROLE', 'ANY')`, with NULL failing closed.
- Red test reproduced the missing gate before implementation; focused permission test passed after the fix.
- Verification: `npm run check:quality` passed; frontend unit tests 231/231; full Rust/Tauri tests 311/311. Existing lint/build/Rust warnings remain.
- Qeen revalidation of the updated commit pending. Direct Query B count/mapping remains blocked because `STAFFKIT_MSSQL_*` is not configured. No DB connection or writes occurred.

## 2026-10-13 Remote EE Sync TLS diagnostics — Start

- Objective: investigate SQL Server TLS validation failure and improve safe Test Connection diagnostics while preserving certificate verification, Query B, and permission gate.
- Evidence: TCP 10.184.0.19:1433 is reachable; reverse DNS resolves to ASWPLUSV2.aswigsyd.int. Actual certificate SAN, expiry, and issuer chain are not yet available from the client.
- Safety: feature worktree only; no DB connection or writes, credentials, merge, or push.

## 2026-10-13 Remote EE Sync TLS diagnostics — End

- Root cause confirmed in application diagnostics: native TLS error categories were discarded by generic formatter/sanitizer. Actual server certificate defect cannot be assigned without reading the certificate; the IP resolves to `ASWPLUSV2.aswigsyd.int`.
- Added safe Test Connection guidance for hostname/SAN mismatch, untrusted chain, expiry, and generic TLS failures. Encryption remains required and certificate validation remains enabled; Query B and read-only permission gate are unchanged.
- Verification: focused TLS tests 4/4; all MSSQL Rust tests 15/15; full Rust/Tauri tests 315/315; `npm run check:quality` passed with pre-existing warnings; `git diff --check` passed after log formatting.
- DB safety: no remote or live local DB connection/writes; no credentials accessed. Direct Test Connection and Query B comparison pending a securely configured profile; no merge or push.


## 2026-10-13 Remote EE Sync server input — Start

- Objective: accept valid IPv4, IPv6, or DNS hostnames exactly as entered; reject malformed endpoints/URL schemes; preserve the host for persistence and SQL TLS verification.
- TLS status: previous network evidence shows ASWPLUSV2.aswigsyd.int resolves to 10.184.0.19 and the requested ASWPLUSV2.aswigsvd.int does not resolve here. Actual certificate SAN, chain, and expiry remain unavailable; no source DB connection will be attempted without a profile/test result.
- Safety: feature branch only; no live DB writes, merge, or push.

## 2026-10-13 Remote EE Sync server input — End

- Root cause: server validation accepted any non-empty string and profile persistence trimmed it, allowing unsupported URL strings and altering the entered host. Connection code already uses the persisted host for TCP and Tiberius TLS verification.
- Fix: validate IPv4, IPv6, and DNS hostname syntax without canonicalizing; reject URL/malformed values; persist accepted Server verbatim; surface safe validation detail. TLS validation remains enabled and unchanged.
- Evidence: ASWPLUSV2.aswigsyd.int resolves to 10.184.0.19 and its TCP/1433 responds; ASWPLUSV2.aswigsvd.int does not resolve on this client. Actual SAN/issuer/expiry and exact saved-profile TLS error remain unverified; no live connection.
- Verification: focused MSSQL tests 19/19; full Rust/Tauri tests 319/319; `npm run check:quality` passed; target file rustfmt check passed. Repo-wide cargo fmt check reports unrelated pre-existing formatting diffs.
- Safety: tests use in-memory DB fixtures only. No live DB writes, merge, or push.

## 2026-10-13 Remote EE Sync SQL TLS certificate — Start

- Objective: record the DBA-confirmed SQL Server TLS cause and define acceptance checks for the replacement trusted certificate.
- Evidence supplied by user: SQL Server 2017 ERRORLOG says, “A self-generated certificate was successfully loaded for encryption.” Treat the server's self-generated certificate as the confirmed TLS root cause; do not change client TLS validation or connection code.
- Safety: documentation only; no credentials, DB connection, DB writes, Apply, merge, or push.

## 2026-10-13 Remote EE Sync SQL TLS certificate — End

- Recorded the confirmed server-side TLS root cause: SQL Server 2017 loaded its self-generated certificate. No speculative connection-code change or validation bypass was made.
- Added DBA certificate acceptance checks covering exact host preservation, DNS/IP SAN matching, certificate validity/trusted chain, Staff Kit Test Connection, read-only permission gate, Query B mapping/counts, Preview-only verification, and no writes.
- Verification: `git diff --check` passed. No app tests were needed because only documentation changed. No connection, Apply, DB write, merge, or push occurred.

## 2026-10-13 Remote EE Sync SQL account allowlist — Start

- Objective: enforce the exact approved SQL login `user_reader-` in Rust before any SQL network/authentication activity, covering saved profiles, environment overrides, Test Connection, Preview, and Apply.
- Safety: do not inspect/change passwords or HRMS credentials; preserve TLS, permission gate, and Query B; no live DB connection or writes; feature branch only, no merge/push.

## 2026-10-13 Remote EE Sync SQL account allowlist — End

- Added exact `user_reader-` enforcement for saved profiles, environment overrides, settings validation, and the final Rust `connect()` boundary. Rejects the prohibited/unknown usernames, case/whitespace variants, and Windows Integrated auth before TCP/TLS/authentication.
- Updated the EE Sync design spec. TLS, the fail-closed read-only permission gate, and Query B were not changed.
- TDD: new tests failed against the previous behavior, then passed. Focused MSSQL Rust tests 22/22; full Rust/Tauri tests 322/322; `npm run check:quality` passed with pre-existing lint/build/Rust warnings; target-file rustfmt check passed.
- Safety: only synthetic test credentials used; no real passwords/profile secrets accessed or modified. No live DB connection or writes; no merge or push.


## 2026-10-13 Remote EE Sync SQL login correction — Start

- Objective: replace the exact approved SQL login with `user_reader` and add regressions proving all other usernames, including prohibited HRMS login `aswhite-plus-webserver`, are blocked before network/authentication.
- Preserve the current Rust boundary, fail-closed read-only gate, TLS validation, and Query B; do not read or alter credentials or contact SQL Server.
- Safety: feature branch only; no DB writes, merge, or push.
## 2026-10-13 Remote EE Sync SQL login correction — End

- Updated the exact Rust allowlist and current sync spec to `user_reader`; the gate still applies to saved profiles, environment overrides, and the final connect boundary. Added coverage that rejects the old `user_reader-`, case/whitespace variants, unknown names, and `aswhite-plus-webserver` before network/authentication.
- TDD: focused allowlist test failed before the implementation change, then focused MSSQL tests passed 22/22. Full Tauri tests passed 322/322; `npm run check:quality` passed with existing warnings; target Rust formatting and diff checks passed.
- TLS, Query B, and fail-closed permission checks unchanged. No passwords/HRMS credentials inspected or modified; no live DB connection/writes; no merge or push.

## 2026-10-13 Remote EE Sync credential and approval cleanup — Start

- Objective: remove prohibited legacy account use from Staff Kit runtime settings, make the approved SQL login a persisted admin-controlled value defaulting to `user_reader`, and require explicit super_admin authorization for changes.
- Gate saved profiles and `STAFFKIT_MSSQL_*` overrides against the persisted approved account before SQL network/authentication; retain TLS, fixed Query B, and fail-closed read-only permission checks.
- Legacy cleanup may clear only an exact prohibited username in Staff Kit EE Sync settings and delete only the Staff Kit Credential Manager target `staffkit.remote-ee-sync.sql-password.v1`; never inspect or touch HRMS-owned credentials.
- Safety: feature worktree only; tests use in-memory state; no live SQL/Staff Kit DB access or writes, merge, or push.

## 2026-10-13 Remote EE Sync credential and approval cleanup — End

- Persisted the approved login in SQLCipher app settings (initial `user_reader`); super_admin-only confirmation and backend account checks cover saved and environment profiles before password retrieval/network. Legacy exact prohibited profile is migrated, Staff Kit password target cleared, and pending previews invalidated on account change.
- Updated the sync spec and added Rust/frontend regressions. Focused Rust 27/27, full Tauri 327/327, frontend 232/232, `npm run check:quality`, and `git diff --check` passed; only existing warnings remain.
- Deleted only the exact Staff Kit Credential Manager target without reading its secret. No HRMS-owned credential touched; no SQL/live Staff Kit DB accessed or written; no merge/push.

## 2026-10-13 Remote EE Sync read-only gate diagnosis — Start

- Trace the exact effective-permission check rejecting `user_reader` and distinguish actual write grants from false-positive scope checks.
- Use only SELECT diagnostics against the approved source when an authorized read-only connection is available; never Apply or write to production or local business data.
- Keep the gate fail-closed; add a focused regression and commit on the existing feature branch only if the exact false rejection is demonstrated.

## 2026-10-13 Remote EE Sync read-only gate diagnosis — End

- Found a concrete false-positive predicate: `HAS_PERMS_BY_NAME(P.name, 'SERVER ROLE', 'ANY')` classified every permission on a server role—including read-only `VIEW DEFINITION`—as write access. Replaced it with explicit `ALTER`, `CONTROL`, and `TAKE OWNERSHIP` checks; all unknown/NULL cases remain fail-closed.
- TDD regression failed against the old `ANY` predicate, then passed after the change. Full Tauri suite 327/327 and `npm run check:quality` passed (existing warnings only); `git diff --check` passed.
- Live per-login evidence could not be collected: all `STAFFKIT_MSSQL_*` environment variables are unset and the app-owned password target is absent. No SQL/DB connection or writes; exact `user_reader` predicate match remains to confirm with Test Connection/SELECT output.

## 2026-10-13 Remote EE Sync read-only gate diagnostic detail — Start

- Follow up on the user's exact Test Connection result: “Could not verify read-only access.” The previous session recorded no connection data; this session sees the Staff Kit Credential Manager target present but no `STAFFKIT_MSSQL_*` environment profile.
- Trace the generic result to the discarded Tiberius query/stream error and add bounded, sanitized SQL Server diagnostics without weakening the permission gate.
- Safety: do not retrieve credentials or connect to production in this session; feature branch only, SELECT-only design, no Apply, DB writes, merge, or push.

## 2026-10-13 Remote EE Sync read-only gate diagnostic detail — End

- Confirmed the current generic Test Connection response originates from `ensure_mssql_login_is_read_only` discarding Tiberius errors for both query execution and result streaming; the UI sanitizer then collapses the remaining permission diagnostic to a generic message. This confirms where the failure occurs, but not its exact SQL permission/scope.
- Added bounded, control-character-stripped SQL Server error number/message display for server errors, and explicit fail-closed diagnostics for driver/no-result cases. Permission result logic, SQL query, Query B, and connection checks are unchanged.
- Focused diagnostics tests 2/2 and full Tauri tests 329/329 passed; `npm run check:quality` passed (existing warnings). Target file rustfmt and `git diff --check` passed. No credential was read; no SQL query/live DB writes, merge, or push. Exact runtime scope awaits rerunning Test Connection with this build.

## 2026-10-14 Remote EE Sync distinguish gate outcomes — Start

- Investigate the user's screenshot showing the same generic failure after diagnostic work. Determine whether the UI is still on an older build or the permission query positively reported write access; make those outcomes distinguishable without weakening the gate.
- TDD a regression for preserving the explicit positive write-permission result. No connection, credential access, Apply, or DB writes; feature branch only.

## 2026-10-14 Remote EE Sync distinguish gate outcomes — End

- Screenshot's generic wording could be produced by an older binary or by the current positive `HasWriteAccess = true` path, because sanitizer merged that verified finding with permission-check failures. This change preserves a distinct blocked message when effective write-level permissions are positively detected; diagnostics remain fail-closed.
- Regression failed before the sanitizer update and passed after it. Full Tauri tests passed 330/330; `npm run check:quality` completed successfully with existing warnings; targeted Rust format and diff checks passed.
- No SQL/live DB connection, credential access, Query B, Preview, Apply, merge, or push. Exact user_reader grant/scope remains unverified until the updated Test Connection reports its distinct result.

## 2026-10-14 EE List Update end-to-end root-cause audit — Start

- Trace the approved Query B sync from UI and saved profile through connection, permission verification, source mapping, preview, local Apply transaction, and UI refresh. Gather runtime evidence from the current feature build before changing behavior.
- Use the saved Windows credential only through Staff Kit; do not inspect or print it. Remote operations are SELECT-only, and live local Apply is excluded.
- Preserve unrelated worktree changes, implement a focused root-cause fix only when supported by evidence, run required checks, commit only this feature branch, and request independent Qeen validation. No merge or push.

## 2026-10-14 EE List Update end-to-end root-cause audit — End

- Static trace: Update invokes a fresh profile resolution and connection through the saved Staff Kit Credential Manager target, then the fail-closed permission query and fixed Query B. Query B uses the approved INNER JOIN, active filter, Code dedupe, five mapped columns, and no TOP cap. Preview reads local EE List and holds a diff; only explicit Apply re-fetches source and writes New/Changed EE List rows in a transaction; UI refreshes after success.
- Confirmed a separate observability defect in EE List Update: its frontend error mapper collapsed the backend's bounded permission-query diagnostic and verified-write result into identical generic DBA guidance. Focused tests failed before the UI change and passed after. The UI now preserves only recognized bounded safe diagnostics, with credential-like content rejected.
- Added isolated regressions for local AzureAD mapping, Missing retention, unrelated employee-group preservation, and UI refresh after Apply. Frontend tests 236/236, Rust tests 330/330, `npm run check:quality`, target Rust formatting, and diff checks passed; existing warnings remain.
- Started `npm run tauri:dev` from the feature worktree at port 5173; saved Credential Manager target exists and no MSSQL environment override is set. Windows remained at the lock screen, so no live Test Connection, Query B count, direct field comparison, or Preview could be observed. No Apply, remote SQL command, credential inspection, merge, or push; underlying live failure remains unconfirmed.

## 2026-10-14 Live EE Sync permission-gate diagnosis — Start

- Zon unlocked and signed in to the feature build. Test Connection with the saved profile passed transport/authentication and returned the verified `HasWriteAccess` rejection; Query B has not run.
- Identify the exact permission scope using SELECT-only diagnostics through the existing saved-profile connection. Preserve the fail-closed gate and never read/display credentials or Apply to live data.

## 2026-10-14 Live EE Sync permission-gate diagnosis — End

- Runtime Test Connection on saved profile reported `effective SERVER permission ALTER ANY DATABASE`; TLS/login succeeded on the successful attempt. A later attempt coincided with TCP unavailable, then port 1433 recovered. The effective server permission is the confirmed cause; Query B and Preview remain blocked before source fetch.
- Added bounded SELECT-only permission finding after the existing positive write gate; it only explains the rejection, cannot authorize, and falls back to the original block if diagnostic fails. Qeen independently reviewed the diff and reported PASS; no permission bypass, Apply, live DB write, merge, or push.
- Full Rust tests passed 331/331, focused finding test passed, `npm run check:quality` passed (one existing ESLint warning), rustfmt and diff checks passed. After admin re-login, EE List → Update displayed the same exact permission finding and stopped before Preview.
- No Query B, source row count, preview, Apply, local/remote write, merge, or push; the positive server permission remains the blocker.

## 2026-10-14 Verify SQL 2017 permission NULL handling — Start

- User supplied SSMS evidence that `IS_SRVROLEMEMBER('dbcreator')` and `IS_SRVROLEMEMBER('sysadmin')` are 0 while `HAS_PERMS_BY_NAME(NULL,'SERVER',...)` returned NULL. Verify SQL Server 2017 semantics and compare the explicit SERVER class, default class, and `fn_my_permissions` through the saved Staff Kit profile.
- Correct any false interpretation of NULL without allowing unknown permissions through. Keep the connection/permission gate fail-closed; after a proven correction, test Update/Query B/Preview only, never Apply.

## 2026-10-14 Verify SQL 2017 permission NULL handling — End

- Microsoft SQL Server 2017 documentation demonstrates server permission checks with `HAS_PERMS_BY_NAME(NULL, NULL, ...)`; SSMS evidence supplied by Zon showed the prior `NULL, 'SERVER'` call returned NULL. The gate had wrapped this result with `COALESCE(..., 1)`, which could report a false elevated permission. Updated server checks to the documented form and retained `COALESCE(..., 1)` so any remaining NULL still blocks.
- Added a SELECT-only comparison diagnostic for server-class/default-class results, effective `fn_my_permissions`, built-in permission existence, dbcreator, and sysadmin; regression tests prove NULL remains blocked and comparison displays it distinctly. Qeen static review: PASS, safe pending live values.
- Verification: focused permission tests passed; full Rust suite 332/332 passed; `npm run check:quality` passed (existing lint/build warnings only); scoped rustfmt and `git diff --check` passed. Full-workspace `cargo fmt --check` reports pre-existing formatting differences in unrelated files; none were formatted.
- Live SSMS comparison and app Test Connection/Query B Preview remain unverified this turn; Computer Use stopped on physical Escape. No remote/local live DB writes, no Apply, merge, or push.

## 2026-10-14 Fix permission diagnostic precedence — Start

- Zon supplied SQL Server output: `HAS_PERMS_BY_NAME(NULL,'SERVER','ALTER ANY DATABASE')=NULL`, documented `NULL,NULL` form=0, `fn_my_permissions` count=0, built-in permission exists=1, dbcreator=0, sysadmin=0. The app screenshot still reports a failure but displays only this comparison.
- Trace the gate diagnostic path: determine why the comparison short-circuits before the actual permission finding; adjust diagnostics only, preserve read-only gate and fail-closed behavior, then request Qeen review. No Apply or live DB writes.

## 2026-10-14 Fix permission diagnostic precedence — End

- Zon provided runtime SSMS evidence for `user_reader`: `HAS_PERMS_BY_NAME(NULL,'SERVER','ALTER ANY DATABASE')=NULL`; `HAS_PERMS_BY_NAME(NULL,NULL,...)=0`; `fn_my_permissions` count=0; built-in permission exists=1; dbcreator/sysadmin=0. Staff Kit still displayed only the comparison and blocked Test Connection.
- Traced the message path: `read_only_permission_finding` returned the first comparison row before executing `MSSQL_READ_ONLY_FINDING`, so the actual gate trigger was hidden. Changed it to query the permission finding first and append the comparison as secondary evidence; gate semantics remain unchanged and fail-closed.
- TDD: new diagnostic-priority test failed before implementation and passed after. Full Rust tests passed 333/333; `npm run check:quality` passed with existing lint/build warnings; scoped rustfmt and `git diff --check` passed.
- No Apply, source/local DB writes, credential reads, merge, or push. New diagnostic must still be tested in the live app to identify the actual permission or unverifiable check causing rejection.

## 2026-10-14 Investigate procedure EXECUTE gate — Start

- User reports the permission gate now fails with effective PROCEDURE permission EXECUTE. Trace the gate and diagnostic to exact SQL; do not relax the fail-closed rule without identifying the procedure, module execution context, and effective grant source. Live DB remains SELECT-only; no Apply, merge, or push.


## 2026-10-14 Investigate procedure EXECUTE gate — End

- Traced the block to the blanket any-procedure EXECUTE test in src-tauri/src/db/mssql_import.rs (MSSQL_READ_ONLY_CHECK) and its generic finding in MSSQL_READ_ONLY_FINDING. Current app diagnostic omits procedure name, is_ms_shipped, execution context, and grant source. Without those live values, cannot distinguish a harmless read-only procedure from executable code with write capability; no permission gate was relaxed and no DB connection/query was run. Requested sanitized SELECT-only SSMS evidence for the exact procedure and grants. Awaiting result before any fix or commit.

## 2026-10-14 Audit final EE Sync permission gate — Start

- Staff Kit runtime reports effective DATABASE EFFECTIVE permission VIEW ANY COLUMN ENCRYPTION KEY DEFINITION. Audit every SQL Server 2017 permission scope; narrowly allow proven metadata reads, explicitly reject NULL/unknown and all write/elevation/unsafe EXECUTE capabilities. Use saved profile for SELECT-only Test Connection and Query B Preview after safe code verification; never Apply or write live DB.


## 2026-10-14 Audit final EE Sync permission gate — End

- Traced the permission gate blockers in src-tauri/src/db/mssql_import.rs:
  1. C13_SchemaEffective: Built-in fixed database role schemas (schema_id >= 16384 or S.name IN ('db_owner', 'db_accessadmin', ...)) inherit permissions from db_datareader. Scoped schema checks to S.schema_id < 16384 AND S.name NOT IN ('db_owner', ...) in both MSSQL_READ_ONLY_CHECK and MSSQL_READ_ONLY_FINDING.
  2. C14_ObjectEffective: Scalar functions ([dbo].[fn_diagramobjects], type FN) require EXECUTE (side-effect free in SQL Server per error 443). Allowed EXECUTE strictly on scalar functions (O.type IN ('FN', 'FS', 'AF')), while maintaining strict blocks on procedures (P, PC, RF, X) and tables (U).
  3. C15_DbPrincipals & C16_ServerLogins: SQL Server grants self-management permissions (CONTROL, ALTER, IMPERSONATE) to principals on themselves (P.name = USER_NAME() / P.name = SUSER_SNAME()). Added P.name <> USER_NAME() and P.name <> SUSER_SNAME() to prevent false positives while strictly preventing elevation over other users or logins.
- Live SQL Server verification against 10.184.0.19:1433 (DB AssetManagement, user user_reader):
  - Read-only permission gate PASSED (Ok(())).
  - Canonical Query B fetched exactly 397 rows with all 5 required fields verified (ASWCVN01: Brendan Cunningham, Brendan, Brendan@consulting.aswhiteglobal.com, Brendan@consulting.aswhiteglobal.com).
- Full regression verification: 338 Rust unit tests passed (38/38 in mssql_import), npm run check:quality passed (lint, strict typecheck, vite build, cargo check). Zero writes performed on remote DB; local employee list remains untouched prior to user approval. Ready for Zon acceptance testing.

## 2026-10-18 UI Polish: Themes, Team Colors & EE Sync Popover — Start

- Branch: `feat/ui-polish-themes-sync-popover`
- Objectives:
  1. Fix Asset Dashboard light-mode styling bug across all sections, tables, category tabs, filters, buttons, badges, and popovers using global CSS theme variables; preserve dark appearance.
  2. Add distinct, deterministic Team pill colors in Employee List (table and mobile cards) across dark/light themes without modifying DB schema or team records; unrecognized teams fall back to neutral style.
  3. Replace the wide EE List Sync Preview panel with a compact popover anchored to the Update button; float over employee table without table layout shift; show New, Changed, Missing, and Conflicts counts; keep Skip and Approve actions.
  4. Show "View details" only when New/Changed or Conflicts exist, displaying relevant rows and never Unchanged rows; eliminate Unchanged rows UI while preserving internal diff classification.
  5. Implement No-change behavior: when New/Changed/Missing/Conflicts are 0, suppress popover, display "✓ No changes" badge next to Update with 10s auto-hide, 0 IPC apply calls, 0 DB writes.
  6. Missing-only sync shows warning popover (report-only, Approve disabled); Conflicts show blocking popover (Approve disabled); Approve enabled only when New/Changed > 0 with 0 conflicts.

## 2026-10-18 UI Polish: Themes, Team Colors & EE Sync Popover — End

- Implementation:
  1. `AssetDashboard.tsx` & `AssetDashboardColumnsDrawer.tsx`: Replaced hardcoded dark hex codes (`#151921`, `#0d1117`, `#0b0f15`, `#11161f`, etc.) and slate classes with CSS variables (`var(--border)`, `var(--surface)`, `var(--surface-hover)`, `var(--bg)`, `var(--text-primary)`, `var(--text-secondary)`, `var(--primary)`). Added `AssetDashboard.theme.test.ts`.
  2. `src/features/teams/teamColors.ts`: Created canonical team color styling mapping (`engineering`, `design`, `marketing`, `hr`, `it`, `sales`, `management`, `delivery`, `support`, `examworks`, `floating`/`zz_floating`) and neutral fallback. Added `teamColors.test.ts`.
  3. `EmployeeCellContent.tsx` & `EmployeeView.tsx`: Applied `getTeamPillClasses` to table team column and mobile employee cards.
  4. `EmployeeView.tsx`: Replaced wide sync preview panel with compact anchored popover (`absolute left-0 top-full z-40 mt-2 w-[440px]`). Preserved exact test contract strings and buttons (`Skip`, `Approve`). Added New/Changed/Missing/Conflict counts and "View details" toggle showing only actionable rows. Removed unchanged rows table and details element.
  5. `useEmployeeState.ts`: Added `mssqlAutoDismissTimerRef` (10s auto-dismiss). If `hasAnyChanges === false`, preview popover is suppressed and `mssqlUpdateMessage = "✓ No changes"` is shown with status `success`.
  6. `scripts/action-icon-ui.test.ts`: Updated line 74 regex to match theme-compliant `border-emerald-500`.
- Verification:
  - `npm run check:quality`: PASSED (0 lint errors, tsc typecheck clean, vite build clean, cargo check clean)
  - `npm run test:unit`: PASSED (33 files, 259/259 tests passed)
  - `npm run test:tauri`: PASSED (353/353 tests passed)
  - `npm run test:supplemental`: PASSED (borrow-lan-autofill & action-icon-ui tests passed)
  - Invariants: 0 DB schema changes, 0 live DB writes, 0 MSSQL writes, all worktrees and untracked files preserved.
