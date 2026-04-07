Plan: Asset Import Dashboard
Add an Asset Dashboard section inside Settings for centralized asset management — import 3 Excel file types, display summary cards + detail tables, and integrate stock tracking with Borrow/Return flow.

Steps

Phase 1: Category Master & Schema Updates (backend only)

Expand asset_categories seed with new prefixes: MACPRO, IMACPRO, MACAIR
Add usage_location TEXT column to assets table (Monitor: office vs home)
Add display_name_short TEXT column to assets table (Mon709 for Monitor)
Create Rust helpers: derive_computer_name() and derive_display_name()
Phase 2: Import Pipeline for 3 File Structures (backend)
5. Add header aliases for usage_location (parallel with step 4)
6. Expand asset_import_rows schema with usage_location TEXT
7. Update create_asset_import_batch() — StaffID present → assigned, empty → in_stock
8. Update import_asset_import_batch_valid_rows() — use original codes from file, create loans for assigned rows
9. Remove serialized asset code auto-generation blocker (depends on 8)

Phase 3: Borrow/Return Stock Integration (can run parallel with Phase 4)
10. Extend approve_borrow_request_conn() — borrow: decrement quantity_on_hand, increment assigned_quantity
11. Extend return branch — increment quantity_on_hand, decrement assigned_quantity

Phase 4: Asset Dashboard UI (depends on Phase 1 & 2)
12. Add new backend commands: list_assets_dashboard, get_asset_dashboard_summary, list_stock_items_dashboard, update_stock_item_quantity
13. Create AssetDashboard.tsx — summary cards row + two-tab detail view (Serialized / Quantity)
14. Create useAssetDashboardState.ts — state management hook
15. Integrate into SettingsView.tsx — full-width, above existing import section

Phase 5: Category Management UI (depends on Phase 4)
16. Add "Manage Categories" action within Asset Dashboard — CRUD form with prefix uniqueness validation

Relevant files

schema.rs — add columns, seed categories
asset_import.rs — header aliases, row processing, code generation bypass
asset.rs — derive helpers, new dashboard commands
borrow.rs — stock integration on approve/return
lib.rs — register new Tauri commands
SettingsView.tsx — mount dashboard component
src/features/assets/AssetDashboard.tsx — new file
src/features/assets/useAssetDashboardState.ts — new file
staff-api.ts — new API functions
staff.ts — new TypeScript types
Verification

cargo test --lib passes after Phase 1-3 changes
Successfully import all 3 real Excel files (AssetList.xlsx, Monitor.xlsx, Mouse-Key.xlsx)
Borrow approval decrements stock; return approval restores stock
npm run check:quality passes after Phase 4-5 changes
Manual smoke test: dashboard shows correct summary counts matching imported data
Decisions

Asset codes kept as-is from Excel (no auto-generation)
Computer Name = ASW + asset_code for laptops; Mon + sequence for monitors
Dashboard stays inside Settings (no new nav tab)
Usage Location tracked for monitors only (Tại CTY / Tại NHÀ)
Adapter number not tracked (implicit 1:1 with laptop)
Category prefixes expandable by IT via UI
Stock auto-adjusts on borrow approve / return approve
Further Considerations

AssetImportWizard.tsx (1100 lines) and useAssetImportState.ts (600 lines) exceed recommended size — consider splitting during Phase 4 work
14x window.confirm/prompt calls throughout codebase — replace with in-app modals in a future UX pass