# Employee List Click-to-Copy Email Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reusable, metadata-driven click-to-copy behavior for valid Employee List email cells while preserving inline editing in table Edit mode.

**Architecture:** Extend the Rust/TypeScript employee column contract with an optional semantic `dataType`, marking core `email` and canonical dynamic `azure_account` as `email`. Render a focused React copy-cell component in normal mode, with isolated mouse and keyboard handlers, transient success/failure state, and native `title` tooltips; leave existing `<td>` edit handlers authoritative in Edit mode. Add a small DOM test harness dependency only if the existing frontend toolchain cannot exercise the component, otherwise test the reusable copy logic and static wiring through repository-native Node scripts plus a bounded smoke check.

**Tech Stack:** React 19, TypeScript 5.9, Tauri v2/Rust serde, browser Clipboard API, existing CSS variables, Node `--experimental-strip-types` regression scripts.

## Global Constraints

- Email behavior is selected by semantic column metadata, never by mutable labels.
- Mark `email` and canonical dynamic key `azure_account` as email fields.
- Valid values are trimmed only for clipboard output; preserve displayed casing and do not normalize the address.
- Null, undefined, empty, whitespace-only, and exact `-` placeholder values are inert.
- Normal mode copy must not navigate, select/activate rows, invoke parent row behavior, or open employee details.
- Edit mode retains inline editing and must not call the clipboard.
- Enter copies; Space copies and prevents scrolling.
- Success tooltip/state is `Copied` for approximately 1500 ms; rejected clipboard calls show `Copy failed`.
- Use `navigator.clipboard.writeText`; do not add a Tauri clipboard plugin or command.
- Do not modify `web/`, commit, push, merge, or alter `main`.

---

### Task 1: Extend semantic employee-column metadata

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/src/db/column.rs`
- Modify: `src/types/app.ts`
- Modify: `src/types/staff.ts`
- Test: `src-tauri/src/db/column.rs` unit tests or the narrowest existing Rust column-contract test location

- [ ] Write a failing Rust/TypeScript contract test proving `email` and `azure_account` return `dataType: "email"` while an ordinary dynamic field does not.
- [ ] Run the targeted test and confirm it fails because the response contract has no semantic type.
- [ ] Add `data_type`/`dataType` to the serialized column definition and centralize the two email keys beside existing schema constants.
- [ ] Populate core and dynamic definitions from that centralized semantic mapping.
- [ ] Update frontend types and API-facing structures to accept the optional semantic type.
- [ ] Run the targeted Rust test and `npm run typecheck`.

### Task 2: Add the reusable copy-cell behavior

**Files:**
- Create: `src/features/employees/EmailCopyCell.tsx`
- Create: `src/features/employees/emailCopyCell.ts`
- Create: `scripts/employee-email-copy.test.ts`
- Modify: `src/index.css`

- [ ] Write failing tests for trimming/casing, inert values, success/failure results, and keyboard activation semantics.
- [ ] Run the tests and confirm they fail before implementation.
- [ ] Implement the smallest reusable copy operation around `navigator.clipboard.writeText`, returning explicit success/failure and never calling the clipboard for inert values.
- [ ] Implement the button component with `type="button"`, accessible name `Copy email <email>`, native tooltip state, click/Enter/Space activation, `preventDefault` for Space, event propagation isolation, focus preservation, and 1500 ms success reset.
- [ ] Add green hover/focus/success/failure styles using existing theme variables without changing ordinary cells.
- [ ] Run the targeted regression script and confirm it passes.

### Task 3: Integrate normal-mode copy with Edit-mode precedence

**Files:**
- Modify: `src/features/employees/EmployeeView.tsx`
- Modify: `scripts/employee-edit-ui.test.ts`
- Modify: `scripts/employee-email-copy.test.ts`

- [ ] Add failing integration regression coverage for normal click copy, Edit-mode inline editor activation, no clipboard call in Edit mode, and restoration after leaving Edit mode.
- [ ] Run the regression scripts and confirm the new assertions fail against current table markup.
- [ ] Render `EmailCopyCell` only when the column metadata is `dataType: "email"`, the value is valid, and the cell is not currently editable; keep inert values as ordinary spans.
- [ ] Ensure the existing `<td>` edit click/double-click handlers remain the only path in Edit mode and that the copy button stops propagation in normal mode.
- [ ] Run targeted UI regression scripts and `npm run typecheck`.

### Task 4: Validate and bounded-smoke the feature

**Files:**
- Modify: `daily_log.md`

- [ ] Run `npm run lint`.
- [ ] Run `npm run typecheck`.
- [ ] Run `npm run build`.
- [ ] Run `npm run check:tauri`.
- [ ] Run the targeted employee regression scripts, including the new email-copy coverage.
- [ ] Perform a bounded smoke check using the available headless/dev tooling: verify the app frontend starts, inspect the rendered email button behavior where possible, and ensure no runtime console/launch error is introduced.
- [ ] Review `git diff`, confirm only scoped files changed, and append an End record with PASS/FAIL/NOT RUN evidence; leave all changes uncommitted for user review.
