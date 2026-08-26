# Asset Re-import Approval Gating Follow-up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure serialized Asset approval requires at least one New row and zero blocking Error/Conflict rows, then prove the real workbook imports four Samsung `H Phone` assets without touching existing assets.

**Architecture:** Keep the existing backend classifier and commit path unchanged. Derive the frontend approval-disabled state from the direct preview’s New count and blocking error summary, and exercise the hook with real preview-shaped rows in four regression cases. Run a temporary isolated acceptance harness through the existing Rust parser/import functions with `H Phone` inserted through the normal category setup path.

**Tech Stack:** React 19, TypeScript, Vitest, Rust, rusqlite, calamine, Tauri shared-target scripts.

## Global Constraints

- Existing/Skip rows are non-blocking and never receive metadata/status updates.
- New rows are the only rows eligible for insertion.
- Error/Conflict rows are blocking and prevent approval.
- `E:\Staff_Kit\00_ExSource\AssetList.xlsx` is read-only acceptance input.
- Acceptance uses an isolated database; the live Staff Kit database is never opened for writes.
- Do not merge to `main`, push, modify the production category model, or stage the pre-existing generated schema changes.

---

### Task 1: Add frontend approval-gating regressions

**Files:**
- Create: `src/features/assets/useAssetDirectImportState.approval.test.tsx`
- Reference: `src/features/assets/useAssetDirectImportState.ts`
- Reference: `src/types/staff.ts`

**Interfaces:**
- Consumes: `useAssetDirectImportState`, `staffApi.inspectAssetImportFile`, and `staffApi.previewAssetImportFile`.
- Produces: four hook-level assertions for the exact product examples.

- [ ] **Step 1: Write the failing tests**

  Prepare the hook with a selected serialized file, return preview-shaped rows with `validRows`, `skippedRows`, `errorRows`, and statuses, then assert `previewApproveDisabled` is false only for six Existing/Skip plus four New and zero errors. Assert it is true for Existing-only, mixed New plus Conflict, and New plus malformed/Conflict rows.

- [ ] **Step 2: Run the focused tests red**

  Run `npm run test:unit -- src/features/assets/useAssetDirectImportState.approval.test.tsx`. The mixed New plus Error/Conflict case must fail against the current `validRows > 0` gate.

### Task 2: Apply the smallest state fix

**Files:**
- Modify: `src/features/assets/useAssetDirectImportState.ts:283`
- Test: `src/features/assets/useAssetDirectImportState.approval.test.tsx`

**Interfaces:**
- Consumes: `AssetDirectImportPreview.validRows` and `AssetDirectImportPreview.errorRows`.
- Produces: `previewApproveDisabled` true when there are no New rows or any blocking error rows.

- [ ] **Step 1: Implement the gate**

  Replace the `validRows`-only expression with a boolean requiring `validRows > 0` and `errorRows === 0`; leave the backend API, Existing/Skip semantics, conflict detection, and approval handler unchanged.

- [ ] **Step 2: Run focused frontend tests green**

  Run the focused approval file, then the existing Asset hook/status tests. Confirm all four cases pass and no unrelated test is changed.

### Task 3: Run real workbook acceptance in isolation

**Files:**
- Validation only: temporary isolated DB/setup artifact outside tracked product files; remove it after evidence is captured.
- Read-only input: `E:\Staff_Kit\00_ExSource\AssetList.xlsx`

**Interfaces:**
- Consumes: the actual workbook, existing parser/classifier/commit functions, and the supported asset category setup path.
- Produces: preview 10/4/6/0, first import 4 with zero existing writes, second import 0 writes.

- [ ] **Step 1: Establish isolated DB state**

  Open an in-memory or temporary-copy SQLite DB, run normal migrations/setup, seed the six existing Lenovo assets and their active-loan/status fixture, and add category `H Phone` through the supported category setup function/API path only.

- [ ] **Step 2: Parse and preview the actual workbook**

  Read `AssetList.xlsx` without modifying it and assert total 10, Existing/Skip 6, New 4, Error/Conflict 0, with all four new rows using category `H Phone`.

- [ ] **Step 3: Commit and re-import**

  Assert first import inserts exactly 4, writes 0 existing assets, preserves existing status/loan state, then run the same workbook again and assert Existing/Skip 10, New 0, and writes 0.

### Task 4: Verify and commit

**Files:**
- Modify: `daily_log.md` with the End record.
- Review: all tracked diff files; do not stage pre-existing generated schemas.

- [ ] **Step 1: Run required checks**

  Run focused approval-gating frontend tests, focused Asset import Rust tests, `npm run test:unit`, `npm run test:tauri`, `npm run check:quality`, and `git diff --check`.

- [ ] **Step 2: Review scope**

  Confirm only the hook, focused test, plan/log evidence, and any explicitly required acceptance validation artifact changed; confirm the live DB was not touched.

- [ ] **Step 3: Commit without merge or push**

  Run `git add src/features/assets/useAssetDirectImportState.ts src/features/assets/useAssetDirectImportState.approval.test.tsx daily_log.md docs/superpowers/plans/2026-08-26-asset-reimport-approval-gating-follow-up.md` followed by `git commit -m "fix(import): block approval on asset conflicts"`.
