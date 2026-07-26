# Daily Log - 2026-04-20

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
