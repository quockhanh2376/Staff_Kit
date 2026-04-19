## Why

`assets.computer_name` is always derivable as `"ASW" + asset_code` — yet it is persisted redundantly, causing seed-from-EE-list to set `display_name = computer_name`, making the Asset Name and Computer Name columns show identical values. At the same time, non-network assets (monitors, keyboards, etc.) should never show a computer name, but the current code does not enforce this.

## What Changes

- **BREAKING**: Remove `assets.computer_name` column. Computer name is derived on-the-fly as `"ASW" + asset_code` only for categories with `has_computer_name = 1`.
- Add `has_computer_name INTEGER NOT NULL DEFAULT 0` column to `asset_categories`.
- Update category seed data: laptop, macpro, macair, imacpro, desktop/wks → `has_computer_name = 1`; monitor, keyboard, mouse, headset → `has_computer_name = 0`.
- Fix seed-from-EE-list (`employee_asset_seed.rs`): `display_name` = strip `"VN"` prefix from `asset_code` (e.g. `VNLAP293` → `LAP293`), not `computer_name`.
- Update `asset_exists_for_seed` dedup check: only check `asset_code` (no longer check `computer_name`).
- Update employee query: derive laptop-holder display from loans where category `has_computer_name = 1` only.
- Update Asset Dashboard serialized record: computer name is derived at query time via JOIN with `asset_categories`, not read from column.
- Update frontend type `AssetDashboardSerializedRecord`: replace `computerName: string | null` with derived value already present in the row; update `resolveSerializedAssetComputerName` to use the category flag.
- Import Wizard: `COMPUTER_NAME_ALIASES` column from Excel is ignored when committing to `assets` (no target column to write).

## Capabilities

### New Capabilities

- `category-has-computer-name`: `asset_categories.has_computer_name` flag that controls whether a category's assets derive a computer name and whether those assets appear in the EE list Computer Name column.

### Modified Capabilities

- `asset-display-name-seed`: display_name for seeded assets changes from `computer_name` → `strip("VN", asset_code)`.

## Impact

- **Rust**: `schema.rs`, `mod.rs` (migration), `asset.rs`, `employee_asset_seed.rs`, `employee.rs`, `asset_import.rs`
- **Frontend**: `src/types/staff.ts`, `src/features/assets/serializedAssetGridConfig.ts`, `src/features/assets/AssetDashboard.tsx`
- **Breaking DB migration**: existing databases will have `computer_name` column dropped and `has_computer_name` seeded into categories
- **No API shape change** at the Tauri command boundary — `AssetDashboardSerializedRecord` still carries the derived computer name value, just computed differently
