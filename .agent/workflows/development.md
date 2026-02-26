---
description: Development workflow for small-to-medium changes, adapted from BMAD Quick Dev for existing projects.
---

# Development Workflow (StaffKit)

> [!IMPORTANT]
> Use this workflow for focused implementation tasks. Escalate to a short tech spec when scope is system-level.

---

## Step 1: Mode and Scope Check

Decide execution mode:

- **Spec mode**: a written task/spec exists -> implement exactly by ordered tasks.
- **Direct mode**: user gives direct instructions -> gather context quickly, then execute.

Escalate to planning first if request touches multiple layers with unclear boundaries.

---

## Step 2: Context Gathering

Before editing:

1. Identify files to touch.
2. Read nearby code for naming/style/error-handling patterns.
3. Note dependencies and config impacts.
4. Write a short execution checklist.

---

## Step 3: Execute End-to-End

Implement tasks in sequence without unnecessary stop points:

1. Edit code following existing patterns.
2. Add tests where appropriate.
3. Run relevant checks/tests.
4. Continue until all scoped tasks are done or a real blocker appears.

Stop only for blocking ambiguity or repeated failures.

---

## Step 4: Self-Check

Verify before handoff:

- all scoped tasks completed
- acceptance criteria satisfied
- tests/checks passed (or explicitly reported if unavailable)
- no obvious pattern regressions

---

## Step 5: Review and Resolve Findings

Run a diff-based review mindset:

1. Inspect all changes since baseline.
2. List findings by severity.
3. Fix real issues or explicitly acknowledge deferred ones.
4. Summarize final state and remaining risks.

---

## Output Criteria

- Implementation matches requested scope
- Verification evidence is clear
- No unnecessary architecture drift
- Ready for commit
