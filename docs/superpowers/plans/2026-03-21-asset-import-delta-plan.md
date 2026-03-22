# Asset Import Wizard Delta Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-baseline the `codex/asset-import-wizard` branch so the next implementation steps match the NotebookLM asset-import requirements before more code is added.

**Architecture:** Keep the current desktop staged-import skeleton, but pivot the domain model from one generic `assets` import path to a two-mode import system: `quantity` stock import and `serialized` asset import. Treat the current branch as a good UI/backend prototype for staging, not yet the final business model.

**Tech Stack:** Tauri v2, Rust + rusqlite, React 19, TypeScript, existing `staffApi` IPC bridge, NotebookLM requirements from `staff_kit_asset_import_*`, `discuss.pdf`, and `st-2-0-1-lan-borrow-design`.

---

## Branch vs Requirement

- Current branch already has:
  - staged batch persistence
  - review/edit/import wizard skeleton
  - manual add path
  - batch list/detail loading
- Current branch is still missing the business split required by NotebookLM:
  - `quantity` vs `serialized` import modes
  - category master with `tracking_mode`, `prefix_code`, `qr_required`, `is_active`
  - generated asset code flow for serialized assets
  - stock-item flow for quantity imports
  - explicit alignment between import output and borrow-ready `in_stock` assets

## Plan Change

### Chunk A: Re-baseline the data model before Chunk 4

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/src/db/asset.rs`
- Modify: `src-tauri/src/db/asset_import.rs`
- Modify: `src/types/staff.ts`
- Modify: `src/services/staff-api.ts`

- [ ] Re-scope the current generic `assets` import path as `serialized-first prototype`, not the final import architecture.
- [ ] Add the missing business layer to the plan: `asset_categories`, `quantity` stock records, and `serialized` asset records must be represented explicitly.
- [ ] Decide whether the existing `assets` table becomes the serialized asset table or remains a temporary compatibility layer for borrow `2.0.1`.
- [ ] Lock the minimum import output for this phase:
  - quantity import updates stock only
  - serialized import creates individual borrowable asset records
  - only serialized output feeds the current LAN borrow selector

### Chunk B: Replace the wizard's single import flow with a mode-aware flow

**Files:**
- Modify: `src/features/assets/useAssetImportState.ts`
- Modify: `src/features/assets/AssetImportWizard.tsx`
- Modify: `src/features/settings/SettingsView.tsx`

- [ ] Add `import type` as a first-class decision in the wizard: `quantity` or `serialized`.
- [ ] Make file parsing, required columns, and review grid depend on the selected mode.
- [ ] Align required columns to the templates already stored in NotebookLM:
  - quantity: `item_name, category, brand, model, quantity, warehouse, note`
  - serialized: `category, asset_name, brand, model, serial_number, warehouse, note`
- [ ] Keep the current staged review/edit UX, but treat it as the `preview` step from the spec and `Import Valid Rows` as the desktop equivalent of `confirm`.

### Chunk C: Defer the right things explicitly

**Files:**
- Reference only: `docs/superpowers/specs/2026-03-16-st-2-0-1-lan-borrow-design.md`
- Reference only: `docs/superpowers/plans/2026-03-21-asset-import-wizard.md`

- [ ] Do **not** pull phase-2 requirements into the next coding slice:
  - per-asset two-QR comparison
  - physical QR verification logs
  - full return/repair/disposal lifecycle
  - bulk QR printing
  - advanced duplicate heuristics
- [ ] Keep the next coding slice focused on P0 only:
  - category tracking mode
  - quantity vs serialized split
  - serialized asset-code generation
  - preview/review/confirm flow
  - batch history that supports IT traceability

## Exact Decision Before Coding

- The old plan should **not** continue straight into the current Chunk 4.
- The next implementation plan must be rewritten so Chunk 4 becomes a **re-baseline chunk**, not a UI-polish chunk.
- After that re-baseline, implementation should continue in this order:
  1. category master + mode split
  2. serialized asset-code generator
  3. mode-aware preview/review flow
  4. batch history + borrow alignment

**Result:** we keep the current branch work, but stop treating it as "almost done". It is now the staging foundation for the real P0 import design.
