# Asset Import Rebaseline Execution Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-align `codex/asset-import-wizard` with `business_rules_master_spec_st_adp` and `Staff_Kit-All-Docs.md` before resuming feature code.

**Architecture:** Keep the current staged-import skeleton, but turn it into a mode-aware SQLite pipeline: `quantity` imports create stock records only, while `serialized` imports create borrowable asset records with spec-aligned statuses. Keep the LAN borrow runtime untouched until import outputs and status semantics are stable.

**Tech Stack:** Tauri v2, Rust, rusqlite, React 19, TypeScript, NotebookLM source docs (`Business Rules Master Spec`, `Staff_Kit-All-Docs`, `staff_kit_asset_import_summary`, `DAILY_LOG`).

---

## Chunk 1: Re-lock the business model

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/src/db/asset.rs`
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src-tauri/src/db/borrow.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/services/staff-api.ts`
- Modify: `src/types/staff.ts`

- [ ] Add `asset_categories` with `tracking_mode`, `prefix_code`, `qr_required`, and `is_active`.
- [ ] Split import outputs explicitly: `quantity` goes to stock data only; `serialized` creates individual borrowable asset records.
- [ ] Keep borrow compatibility by making the current serialized asset store the only source for borrowable search results.
- [ ] Align post-import status semantics so serialized assets land in the borrow-searchable `in_stock` / `In Stock` state, while quantity data keeps its own `available` / `assigned` semantics.
- [ ] Keep import scope clean: import never assigns assets to employees and never bypasses approval rules.

## Chunk 2: Replace the generic wizard path with a mode-aware preview flow

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/features/settings/SettingsView.tsx`
- Modify: `src/features/assets/useAssetImportState.ts`
- Modify: `src/features/assets/AssetImportWizard.tsx`

- [ ] Make `import type` a first-class branch choice: `quantity` or `serialized`.
- [ ] Require explicit header mapping from detected columns before staging; do not blanket-import file columns.
- [ ] Align required fields to the approved templates:
  - `quantity`: `item_name, category, brand, model, quantity, warehouse, note`
  - `serialized`: `category, asset_name, brand, model, serial_number, warehouse, note`
- [ ] Keep the current review UI as the desktop equivalent of `preview`, and keep it reading only from SQLite staged rows, never directly from raw Excel/CSV.
- [ ] Preserve the current manual-add entry point, but treat it as a serialized-only fallback until quantity manual handling is designed separately.

## Chunk 3: Add the validation and confirm rules that unblock later borrow work

**Files:**
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src-tauri/src/db/asset.rs`
- Modify: `src-tauri/src/db/audit.rs`
- Modify: `src/services/staff-api.ts`
- Modify: `src/types/staff.ts`
- Modify: `src/features/assets/AssetImportWizard.tsx`

- [ ] Generate serialized asset codes from `prefix_code`, with a global per-prefix sequence and no duplicate codes under concurrent import.
- [ ] Normalize staged values before confirm: uppercase serials, reject duplicate active serials, reject invalid categories, reject invalid warehouses if warehouse validation is enabled, reject `quantity <= 0`.
- [ ] Finalize row-level validation feedback so `Import Valid Rows` commits only valid rows and leaves invalid rows staged for later correction.
- [ ] Write/import audit entries for staging, confirm, skip, and manual add paths.
- [ ] Carry forward only the rules that matter now; defer `computer name` regex and receive-form-specific fields to the later receive-review slice.

## Explicit Defers

- [ ] Do not add return flow logic in this slice.
- [ ] Do not add per-asset QR generation or two-QR comparison logic in this slice.
- [ ] Do not add maintenance, damaged, retired, or replacement workflows in this slice.
- [ ] Do not add auto-approve, bulk QR printing, or advanced duplicate heuristics in this slice.

## Verification

- [ ] `npm run lint`
- [ ] `npm run typecheck`
- [ ] `npm run check:quality`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] Manual smoke: preview and confirm one `quantity` file and one `serialized` file.
- [ ] Manual smoke: confirm that borrow search still sees only eligible serialized `in_stock` assets after import.

