# Super Admin Computer Name Edit Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let only `super_admin` users edit the employee-table `Computer Name` field while all other roles keep it read-only.

**Architecture:** Add a dedicated auth capability for `Computer Name`, thread it through the employee-table edit hook, and persist the raw employee fallback value through the existing save payload. Keep derived laptop-display precedence unchanged.

**Tech Stack:** React, TypeScript, local script rails, existing employee table/frontend auth state.

---

## Chunk 1: Capability and helper rules

### Task 1: Add failing capability test

**Files:**
- Modify: `scripts/auth-capabilities.test.ts`
- Modify: `src/features/auth/authCapabilities.ts`

- [ ] **Step 1: Write the failing test**

Add assertions for a new `canEditEmployeeComputerName` capability:
- false when signed out
- false for `user`
- false for `admin`
- true for `super_admin`

- [ ] **Step 2: Run test to verify it fails**

Run: `node --experimental-strip-types scripts/auth-capabilities.test.ts`
Expected: FAIL because `canEditEmployeeComputerName` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add `canEditEmployeeComputerName` to `AuthCapabilities` and derive it from the `super_admin` role.

- [ ] **Step 4: Run test to verify it passes**

Run: `node --experimental-strip-types scripts/auth-capabilities.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scripts/auth-capabilities.test.ts src/features/auth/authCapabilities.ts
git commit -m "test: cover super admin computer name capability"
```

### Task 2: Add failing employee-rule test

**Files:**
- Modify: `scripts/employee-computer-name-rules.test.ts`
- Modify: `src/features/employees/employeeTableRules.ts`

- [ ] **Step 1: Write the failing test**

Add assertions that:
- `computerName` is editable only when an explicit `canEditComputerName` flag is true
- payload save preserves stored fallback for non-super-admin
- payload save writes draft `computerName` for super-admin

- [ ] **Step 2: Run test to verify it fails**

Run: `node --experimental-strip-types scripts/employee-computer-name-rules.test.ts`
Expected: FAIL because helper signatures and payload behavior do not support the new rule.

- [ ] **Step 3: Write minimal implementation**

Update helper signatures and payload assembly to support role-gated `computerName` editing.

- [ ] **Step 4: Run test to verify it passes**

Run: `node --experimental-strip-types scripts/employee-computer-name-rules.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scripts/employee-computer-name-rules.test.ts src/features/employees/employeeTableRules.ts
git commit -m "test: cover super admin computer name edit rules"
```

## Chunk 2: Wire capability into employee table

### Task 3: Thread capability through app state

**Files:**
- Modify: `src/features/auth/useAuthState.ts`
- Modify: `src/App.tsx`
- Modify: `src/features/employees/useTableEdit.ts`

- [ ] **Step 1: Write the failing test**

Reuse the script rails from Chunk 1; no new test file is needed. The next failure should come from TypeScript usage sites that do not pass the new capability through.

- [ ] **Step 2: Run type/quality check to verify it fails**

Run: `npm run check:frontend`
Expected: FAIL or lint/type errors until the new capability is wired through.

- [ ] **Step 3: Write minimal implementation**

Pass `canEditEmployeeComputerName` from auth state into the employee table edit hook and use it when deciding whether `computerName` can enter edit mode.

- [ ] **Step 4: Run check to verify it passes**

Run: `npm run check:frontend`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features/auth/useAuthState.ts src/App.tsx src/features/employees/useTableEdit.ts
git commit -m "feat: wire super admin computer name edit capability"
```

### Task 4: Update employee table copy

**Files:**
- Modify: `src/features/employees/EmployeeView.tsx`

- [ ] **Step 1: Write the failing test**

Add or extend a lightweight UI rail if needed to assert the edit hint now states that only `Super Admin` can edit `Computer Name`.

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run check:frontend`
Expected: FAIL if the rail expects the new copy before implementation.

- [ ] **Step 3: Write minimal implementation**

Update the edit-mode helper text to reflect the new permission rule.

- [ ] **Step 4: Run check to verify it passes**

Run: `npm run check:frontend`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features/employees/EmployeeView.tsx
git commit -m "style: clarify super admin computer name editing"
```

## Chunk 3: Final verification

### Task 5: Run focused verification

**Files:**
- No code changes expected

- [ ] **Step 1: Run script rails**

Run:
- `node --experimental-strip-types scripts/auth-capabilities.test.ts`
- `node --experimental-strip-types scripts/employee-computer-name-rules.test.ts`

Expected: both PASS

- [ ] **Step 2: Run frontend quality**

Run: `npm run check:frontend`
Expected: PASS

- [ ] **Step 3: Commit final integrated changes**

```bash
git add docs/superpowers/specs/2026-04-13-super-admin-computer-name-edit-design.md docs/superpowers/plans/2026-04-13-super-admin-computer-name-edit.md
git commit -m "docs: add super admin computer name edit design"
```
