# QR Return Admin Review Copy Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the desktop admin queue and detail screen communicate Borrow and Return review semantics clearly.

**Architecture:** Keep the current review state/actions intact and isolate the change to copy helpers in the React admin view/state layer. Verify the intended semantics with a lightweight script test instead of introducing a full component test harness.

**Tech Stack:** React, TypeScript, Node strip-types script tests, ESLint, TypeScript compiler

---

## Chunk 1: Lock The Desired Semantics With A Failing Script Test

### Task 1: Add a copy regression script

**Files:**
- Create: `scripts/borrow-admin-review-copy.test.ts`
- Test: `node --experimental-strip-types scripts/borrow-admin-review-copy.test.ts`

- [ ] **Step 1: Write the failing test**

Implementation:
- Assert the admin view no longer centers the feature around borrow-only review wording
- Assert the view exposes shared Borrow/Return review language
- Assert request-type-specific action labels/messages are present

- [ ] **Step 2: Run the script and confirm it fails**

Run: `node --experimental-strip-types scripts/borrow-admin-review-copy.test.ts`
Expected: FAIL until the copy helpers and labels exist

## Chunk 2: Implement Minimal Dynamic Copy

### Task 2: Make the admin screen request-type-aware

**Files:**
- Modify: `src/features/borrow/BorrowAdminView.tsx`
- Modify: `src/features/borrow/useBorrowState.ts`
- Test: `node --experimental-strip-types scripts/borrow-admin-review-copy.test.ts`

- [ ] **Step 1: Add shared review copy**

Implementation:
- Rename borrow-only headings/descriptions to shared Borrow/Return review wording
- Keep queue/detail layout unchanged

- [ ] **Step 2: Add selected-request-specific labels**

Implementation:
- Approve/reject button labels adapt to borrow vs return
- Rejection placeholder and success messages adapt to borrow vs return

- [ ] **Step 3: Re-run the script**

Run: `node --experimental-strip-types scripts/borrow-admin-review-copy.test.ts`
Expected: PASS

## Chunk 3: Verify The Slice

### Task 3: Run full quality verification

**Files:**
- No additional files required

- [ ] **Step 1: Run the quality gate**

Run: `npm run check:quality`
Expected: PASS
