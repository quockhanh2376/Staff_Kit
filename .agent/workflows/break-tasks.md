> **LEGACY REFERENCE ONLY.** Not authoritative for Codex. `AGENTS.md`, `docs/CODEX_WORKFLOW.md`, and project-local Superpowers override this file.

---
description: Orchestrates breaking down requirements into actionable tasks for implementation.
---

# Break Tasks Workflow

> [!IMPORTANT]
> **MANDATORY**: Follow `.agent/rules/documents.md` for all task-related documentation.

---

## MCP Usage Guidelines

| MCP Tool                                     | When to Use                                               |
| :------------------------------------------- | :-------------------------------------------------------- |
| Optional structured reasoning, when available | Break down requirements into atomic tasks                  |
| Official documentation search, when available | Check best practices for specific technologies             |

---

## Step 1: Identify Source Document

1. Locate the source document (PRD, User Story, Feature Spec, or SDD).
2. If multiple versions exist, ask the user for clarification.
3. Relevant folders to check:
   - `docs/020-Requirements/`
   - `docs/022-User-Stories/`
   - `docs/030-Specs/`

---

## Step 2: Analyze Requirements

// turbo

1. **Invoke `[business-analysis]` skill** to extract key features and acceptance criteria.
2. Use an available structured-reasoning capability to:
   - Identify technical dependencies.
   - Separate backend, frontend, and QA requirements.
   - Spot ambiguous or missing details.
3. List any clarifying questions for the user.
4. Pause only when a user decision is required.

---

## Step 3: Atomic Task Breakdown

// turbo

> 💡 An available structured-reasoning capability may be used to keep tasks atomic and manageable.

1. **Invoke `[lead-architect]` skill** to create a structured task list.
2. Group tasks by component or phase (e.g., Database, API, Logic, UI, Testing).
3. For each task, include:
   - Goal/Description.
   - Acceptance Criteria.
   - Estimated complexity (if applicable).
4. Create a `task-breakdown.md` artifact representing the proposed sequence.

---

## Step 4: Finalize Task Documentation

// turbo

1. After user approves the `task-breakdown.md` artifact:
2. Update the `task.md` of the current session or create a new task file in `docs/050-Tasks/`.
3. If creating a new file, follow standard naming: `docs/050-Tasks/Task-{FeatureName}.md`.
4. Update `docs/050-Tasks/Tasks-MOC.md`.
5. Present the finalized task list to the user.

---

## Quick Reference

| Role               | Skill                | Responsibility                          |
| :----------------- | :------------------- | :-------------------------------------- |
| Product Manager    | `product-manager`    | Requirement validation & prioritization |
| Lead Architect     | `lead-architect`     | Technical breakdown & dependencies      |
| Developer          | `backend-developer`  | Backend/API specific tasks              |
| Frontend Developer | `frontend-developer` | UI/UX specific tasks                    |
| QA Tester          | `qa-tester`          | Verification & Edge case tasks          |
