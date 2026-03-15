# Search Highlight Yellow Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make employee-table search matches stand out with a stronger yellow text highlight and a soft yellow matched-cell background.

**Architecture:** Keep search behavior in `EmployeeView.tsx`, but factor the cell-match decision into a small helper so both the text renderer and the cell container can use the same match result. Put the visual styling in shared CSS tokens and utility classes in `src/index.css`.

**Tech Stack:** React 19, TypeScript, Vite, Tailwind-compatible CSS, existing ESLint/TypeScript/Tauri quality gate.

---

## Chunk 1: Search Match Styling

### Task 1: Add CSS tokens and matched-cell styles

**Files:**
- Modify: `src/index.css`
- Test: `npm run check:quality`

- [ ] **Step 1: Add the highlight tokens and table-cell styles**

Add warm yellow tokens for search highlighting and a class for matched table cells that uses a soft background plus subtle inset emphasis.

- [ ] **Step 2: Run the quality gate to ensure CSS changes are valid**

Run: `npm run check:quality`
Expected: PASS

### Task 2: Make employee cells aware of search matches

**Files:**
- Modify: `src/features/employees/EmployeeView.tsx`
- Test: `npm run check:quality`

- [ ] **Step 1: Write the failing test surrogate by identifying the exact behavior gap**

Use the current implementation as the red step:
- matched text is highlighted
- matched cell container is not highlighted

Document the gap in code by introducing a helper that can report whether a rendered cell matches the active search term.

- [ ] **Step 2: Verify the current behavior is insufficient**

Run: `npm run check:quality`
Expected: PASS, while manual inspection of current UI still shows only text-level emphasis.

- [ ] **Step 3: Write the minimal implementation**

Update `EmployeeView.tsx` so each rendered table cell:
- computes whether its text matches the current search term
- applies the matched-cell class when true
- keeps the stronger yellow `mark` rendering for the exact matched substring

- [ ] **Step 4: Run the quality gate to verify the implementation passes**

Run: `npm run check:quality`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-03-15-search-highlight-yellow-design.md docs/superpowers/plans/2026-03-15-search-highlight-yellow.md src/index.css src/features/employees/EmployeeView.tsx
git commit -m "feat: strengthen employee search highlight"
```
