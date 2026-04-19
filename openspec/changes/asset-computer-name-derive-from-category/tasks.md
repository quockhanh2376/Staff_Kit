## 1. Schema & Migration (Rust)

- [ ] 1.1 Add `has_computer_name INTEGER NOT NULL DEFAULT 0` to `asset_categories` CREATE TABLE in `schema.rs`
- [ ] 1.2 Add migration in `mod.rs`: `ALTER TABLE asset_categories ADD COLUMN has_computer_name INTEGER NOT NULL DEFAULT 0` (idempotent)
- [ ] 1.3 Update category seed `UPDATE` in `mod.rs`: set `has_computer_name = 1` for laptop, macpro, macair, imacpro, wks categories
- [ ] 1.4 Add migration in `mod.rs`: `ALTER TABLE assets DROP COLUMN computer_name` (guard with column-exists check)
- [ ] 1.5 Write Rust migration test: verify `has_computer_name` column exists after migration on a legacy DB

## 2. Asset Query Layer (Rust — asset.rs)

- [ ] 2.1 Remove `computer_name` from `AssetUpsertInput` struct
- [ ] 2.2 Remove `computer_name` param from `insert_asset_stmt` and `upsert_assets_conn`
- [ ] 2.3 Update `list_asset_dashboard_serialized_conn` query: JOIN `asset_categories` and derive `computer_name` as `CASE WHEN c.has_computer_name = 1 THEN 'ASW' || UPPER(a.asset_code) ELSE NULL END`
- [ ] 2.4 Update `AssetDashboardSerializedRecord` struct: `computer_name` field now populated from derived SQL value (no structural change needed — field stays)
- [ ] 2.5 Write Rust test: laptop asset returns derived computer name; monitor asset returns null

## 3. Employee Seed Fix (Rust — employee_asset_seed.rs)

- [ ] 3.1 Update `import_employee_asset_seed_conn`: set `display_name = strip_vn_prefix(asset_code)` helper (strip case-insensitive `"VN"` prefix, fallback to `asset_code`)
- [ ] 3.2 Remove `computer_name` from `AssetUpsertInput` call in seed function
- [ ] 3.3 Simplify `asset_exists_for_seed_conn` / `asset_exists_for_seed_tx`: remove `OR computer_name = ?` check, only check `asset_code`
- [ ] 3.4 Write Rust test: seeded asset has `display_name = "LAP293"` for `asset_code = "VNLAP293"`
- [ ] 3.5 Write Rust test: seeded asset has `display_name = "MACPRO010"` for `asset_code = "VNMACPRO010"`

## 4. Employee Query Fix (Rust — employee.rs)

- [ ] 4.1 Update employee query: add JOIN to `asset_categories` in the laptop-loans subquery
- [ ] 4.2 Filter derived computer-name loans to `categories.has_computer_name = 1` only
- [ ] 4.3 Write Rust test: employee with laptop + monitor loans → Computer Name shows only laptop name

## 5. Import Wizard Cleanup (Rust — asset_import.rs)

- [ ] 5.1 Remove `computer_name` from `AssetUpsertInput` construction when committing import rows to `assets`
- [ ] 5.2 Keep `COMPUTER_NAME_ALIASES` parsing and `asset_import_rows.computer_name` staging (for review UI display only)
- [ ] 5.3 Verify existing import tests still pass with no `computer_name` in committed asset record

## 6. Frontend Types & Rendering

- [ ] 6.1 `src/types/staff.ts`: `AssetDashboardSerializedRecord.computerName` remains — no type change needed (derived value already returned from backend)
- [ ] 6.2 `src/features/assets/serializedAssetGridConfig.ts`: update `resolveSerializedAssetComputerName` — when `computerName` is null (non-network category), return `""` without fallback to `ASW + assetCode` (backend now controls this)
- [ ] 6.3 `src/features/assets/assetImportModeConfig.ts`: `buildDerivedComputerName` can remain for display purposes in import review UI — no change needed

## 7. Verification

- [ ] 7.1 Run `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture` — all Rust tests green
- [ ] 7.2 Run `npm run check:frontend` — TypeScript + ESLint + build green
- [ ] 7.3 Manual smoke test: seed from EE list, verify Asset Dashboard shows `LAP293` in Asset Name and `ASWVNLAP293` in Computer Name
- [ ] 7.4 Manual smoke test: monitor asset shows null in Computer Name column
- [ ] 7.5 Commit: `feat: derive computer name from category flag, fix asset display_name on seed`
