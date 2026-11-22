# Edit Asset — short design (2026-11-22)

## Why
Super admins have no way to fix wrong details on an existing asset. Trigger: H Phone 001 and 004 were entered as "Samsung Galaxy A18/A19"; all four (001–004) are "Samsung Galaxy A17". Update Metadata (Excel) is Monitor-only on purpose, and the Settings seed box is gone.

## Goals / non-goals
- Goal: a super admin opens one asset from the Serialized table, fixes its descriptive fields, saves; the change is audit-logged with before/after.
- Non-goal: bulk edit, inline cell editing, widening Update Metadata to other categories.
- Anti-goal: editing anything that loans, history or reports key on — asset code, category, status, holder stay read-only.

## Approaches
1. **Per-row Edit drawer (picked).** Small, safe, matches the existing Category edit drawer.
2. Widen Update Metadata to all categories — bulk, but removes a deliberate guard and is heavy for 1–2 fixes.
3. Inline cell editing — easy to edit the wrong row in a paged/sorted/filtered table; more work.

## Design
- **Backend:** new Tauri command `update_asset_details(session_token, asset_id, input)`, `require_super_admin`.
  - Editable: `display_name` (required, trimmed, non-empty), `display_name_short`, `brand`, `model`, `serial_number`, `adapter_number`, `usage_location`, `notes`. Blank optional field → NULL.
  - One transaction: read current row (not found → "Not found: asset"), `UPDATE assets SET … , updated_at = datetime('now') WHERE id = ?`, write one audit row (`asset_updated`, entity = asset code) with only the changed fields as before/after. No changes → no write, no audit.
  - Error messages use the business prefixes (`Not found`, `Invalid`) so the UI shows them.
- **Frontend:** `staffApi.updateAssetDetails`; in `AssetDashboard` Serialized tab, super-admin-only "Edit" button (`isSuperAdminAccount`) per row opens a `Drawer` (same pattern as the Category drawer) with the fields above, asset code/category/status shown read-only. Save → refresh the table, show success; errors via `classifyError`.
- **Tests:** Rust — updates fields + writes audit with before/after; rejects blank name; admin (non-super) rejected; no-op writes nothing. Vitest — Edit button hidden for non-super-admin; save calls the API with edited values and refreshes.

## Left to the builder
Exact drawer layout and button placement in the row; audit detail JSON shape (follow existing audit rows).
