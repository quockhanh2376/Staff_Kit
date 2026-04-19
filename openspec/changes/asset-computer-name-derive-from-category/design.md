## Context

Currently `assets.computer_name` stores `"ASW" + asset_code` for every asset — the same value that can be derived deterministically. The EE-list seed (`employee_asset_seed.rs`) copies this into `display_name`, causing Asset Dashboard to show identical values in both the Asset Name and Computer Name columns.

Additionally, the employee query derives the "Computer Name" column by joining all active laptop loans, regardless of whether the asset category is a network device. This means monitors, keyboards, and other non-network assets can accidentally appear in the EE list Computer Name column.

## Goals / Non-Goals

**Goals:**
- Remove `assets.computer_name` (zero information gain — fully derivable)
- Add `asset_categories.has_computer_name` flag to control which categories produce a computer name
- Fix seed: `display_name = strip("VN", asset_code)` e.g. `VNLAP293 → LAP293`
- Employee query: filter laptop-holder display to categories where `has_computer_name = 1`
- Asset Dashboard query: derive computer name at SQL/Rust layer using the category flag
- Frontend: no visible behavior change — Computer Name column still shows for laptop-type assets

**Non-Goals:**
- No changes to Borrow/Return flow
- No changes to Import Wizard field mapping UI
- No changes to QR flow
- Not enforcing `display_name` format for existing records (only new seeds)

## Decisions

### D1: Derive computer name at query time, not stored

**Choice:** `computer_name` is derived as `"ASW" + UPPER(asset_code)` in the SQL SELECT, gated by `categories.has_computer_name = 1`.

**Alternative considered:** Store a `computer_name` override column for edge cases.

**Rationale:** At ASW all devices follow the `ASW + assetCode` convention strictly. Storing derived data violates single source of truth. If an edge case arises in future, the `has_computer_name` flag on the category is the right escape hatch — not a per-asset override field.

### D2: `has_computer_name` lives on `asset_categories`, not per-asset

**Choice:** The flag is category-level, not per-asset.

**Alternative considered:** Per-asset boolean flag.

**Rationale:** The decision is always category-wide (all laptops have a computer name, no monitors do). Category-level flag means IT admin only needs to configure it once when creating a new category, not per-import row.

### D3: SQLite DROP COLUMN via migration

**Choice:** Use `ALTER TABLE assets DROP COLUMN computer_name` (SQLite ≥ 3.35, available since 2021).

**Alternative considered:** Rename + recreate table (full table migration).

**Rationale:** SQLite version bundled with Tauri 2 is well above 3.35. Simpler migration, less risk of data corruption during table recreation.

### D4: `display_name` for seed = strip "VN" prefix

**Choice:** `display_name = asset_code.strip_prefix("VN").unwrap_or(asset_code)` (case-insensitive).

**Rationale:** Consistent with observed data pattern: `VNLAP293 → LAP293`, `VNMACPRO010 → MACPRO010`, `VNMON211 → MON211`. This is human-readable without the company/country prefix noise.

### D5: Import Wizard ignores `computer_name` from Excel

**Choice:** `COMPUTER_NAME_ALIASES` still parses the column from the source file (for validation and display in review UI) but does not write to `assets.computer_name` on commit.

**Rationale:** Avoids breaking existing import files that include a Computer Name column. The field is staged in `asset_import_rows.computer_name` but not promoted to `assets` on commit.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Existing records with non-standard `computer_name` lose stored value | At ASW all follow `ASW + assetCode`. Value can be re-derived. |
| SQLite DROP COLUMN not available on very old installs | Tauri 2 bundles SQLite ≥ 3.38. Migration guard can check and skip gracefully. |
| `asset_import_rows.computer_name` still exists but never commits to assets | Documents clearly in code. Future cleanup can remove the staged column. |
| EE list Computer Name changes for any seeded asset whose category lacks `has_computer_name` | Only affects monitor/keyboard/mouse — which should never have appeared there anyway. |

## Migration Plan

1. **`ALTER TABLE asset_categories ADD COLUMN has_computer_name INTEGER NOT NULL DEFAULT 0`**
2. **UPDATE seed rows**: set `has_computer_name = 1` for laptop, macpro, macair, imacpro, wks categories
3. **`ALTER TABLE assets DROP COLUMN computer_name`** (idempotent — skip if already absent)
4. Rollback: no rollback needed — derived value is always reconstructable from `asset_code`

## Open Questions

- Desktop/WKS (`VNWKS*`) categories — are these already in `asset_categories` seed data or will they be added later? If not yet seeded, the migration UPDATE for `has_computer_name` must handle them when added.
