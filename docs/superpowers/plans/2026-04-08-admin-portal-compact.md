# Admin Portal Compact Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the Settings `Admin Portal` card into the same compact collapsible style as `Database & Backup`, including the import target dropdown toolbar and 60-second idle collapse.

**Architecture:** Keep all existing account/import behaviors in place and only reorganize the Settings UI. Reuse the existing `useIdleCollapse` hook, keep state in `SettingsView.tsx`, and lock the visible contract with a lightweight UI rail in `scripts/action-icon-ui.test.ts`.

**Tech Stack:** React, TypeScript, Tailwind utility classes, existing Settings state/hooks, Node strip-types script rail.

---

## Chunk 1: Compact Admin Portal Shell

### Task 1: Lock the new UI contract with a failing rail

**Files:**
- Modify: `scripts/action-icon-ui.test.ts`

- [ ] **Step 1: Write the failing test**

Add assertions that require:
- `Admin Portal` to use `Users`, `ChevronUp`, and `ChevronDown`
- `Admin Portal` to reference `useIdleCollapse`
- `Import` toolbar copy to exist in `SettingsView.tsx`
- the old always-open `mt-4 grid gap-6 md:grid-cols-2` admin body signature to be gone or restructured

- [ ] **Step 2: Run test to verify it fails**

Run: `node --experimental-strip-types scripts/action-icon-ui.test.ts`
Expected: FAIL because the current `Admin Portal` is still always expanded and does not contain the new compact toolbar markers.

- [ ] **Step 3: Write minimal implementation**

Refactor `SettingsView.tsx` so `Admin Portal`:
- has a compact header
- uses `useIdleCollapse(60000)`
- contains a collapsible body wrapper

- [ ] **Step 4: Run test to verify it passes**

Run: `node --experimental-strip-types scripts/action-icon-ui.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scripts/action-icon-ui.test.ts src/features/settings/SettingsView.tsx
git commit -m "style: compact admin portal shell"
```

## Chunk 2: Import Toolbar + Compact Body Layout

### Task 2: Reintroduce compact import target controls inside Admin Portal

**Files:**
- Modify: `src/features/settings/SettingsView.tsx`
- Test: `scripts/action-icon-ui.test.ts`

- [ ] **Step 1: Write the failing test**

Add assertions for:
- `Import`
- `TO:`
- import target dropdown labels in `SettingsView.tsx`

- [ ] **Step 2: Run test to verify it fails**

Run: `node --experimental-strip-types scripts/action-icon-ui.test.ts`
Expected: FAIL because the compact toolbar does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Render a compact toolbar above the admin body using existing import state:
- keep `imp.selectedTarget`
- keep `imp.setSelectedTarget`
- keep existing `Import Excel` action behavior
- style the grouped button/dropdown to align with the approved mock

- [ ] **Step 4: Run test to verify it passes**

Run: `node --experimental-strip-types scripts/action-icon-ui.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scripts/action-icon-ui.test.ts src/features/settings/SettingsView.tsx
git commit -m "style: add compact admin portal import toolbar"
```

## Chunk 3: Compact Account Panels and Verification

### Task 3: Align create-account and account-list surfaces with Database & Backup

**Files:**
- Modify: `src/features/settings/SettingsView.tsx`
- Test: `scripts/action-icon-ui.test.ts`

- [ ] **Step 1: Write the failing test**

Add assertions for the compact styling markers that should be present in the final admin card:
- compact nested panels
- scrollable account list region
- no `Use` button

- [ ] **Step 2: Run test to verify it fails**

Run: `node --experimental-strip-types scripts/action-icon-ui.test.ts`
Expected: FAIL until the account layout is restyled.

- [ ] **Step 3: Write minimal implementation**

Compact the left and right admin sections:
- create-account panel matches compact surfaces
- account list sits in a bounded scrollable panel
- retain edit/reset/delete behavior
- keep inline edit behavior intact
- ensure card inactivity resets when interacting with the account list and form fields

- [ ] **Step 4: Run test to verify it passes**

Run: `node --experimental-strip-types scripts/action-icon-ui.test.ts`
Expected: PASS

- [ ] **Step 5: Run broader verification**

Run:
- `npm run check:frontend`

Expected:
- lint passes
- typecheck passes
- build passes

- [ ] **Step 6: Commit**

```bash
git add scripts/action-icon-ui.test.ts src/features/settings/SettingsView.tsx
git commit -m "style: align admin portal with compact settings cards"
```

Plan complete and saved to `docs/superpowers/plans/2026-04-08-admin-portal-compact.md`. Ready to execute.
