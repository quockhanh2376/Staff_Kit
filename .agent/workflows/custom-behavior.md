> **LEGACY REFERENCE ONLY.** Not authoritative for Codex. `AGENTS.md`, `docs/CODEX_WORKFLOW.md`, and project-local Superpowers override this file.

---
description: Workflow for safely customizing Agent rules and workflows with impact analysis and user confirmation.
---

# Custom Behavior Workflow

## Tool Usage Guidelines

| Tool/Action         | When to Use                                         | Example Command / Action                     |
| ------------------- | --------------------------------------------------- | -------------------------------------------- |
| `rg` / file search  | Step 1: find matching rule/workflow                 | `rg -n "security" .agent/rules .agent/workflows` |
| read file           | Step 2: inspect current content                     | open target file and summarize               |
| user confirmation   | Step 3: confirm impact before overwrite             | explain current vs proposed vs impact        |
| write/edit file     | Step 4: create or update the selected file          | apply patch                                  |

## Step 1: Identification & Search

// turbo

> 💡 **Tip**: Don't assume the file doesn't exist. Always search first.

1.  Analyze the user's request to identify the _intent_ (e.g., "Add stricter linting", "Skip tests in deployment").
2.  Search for existing Rules or Workflows that might already cover this.
    - Rules: search in `.agent/rules/`
    - Workflows: search in `.agent/workflows/`

## Step 2: Impact Analysis

> 💡 **Tip**: If a file exists, you MUST read it and compare it with the request.

**Condition A: Target does NOT exist:**

1.  Verify if a template exists in `.agent/assets/` or `references/` that could be used as a base.
2.  Draft the new content in your memory.

**Condition B: Target ALREADY exists:**

1.  **Read** the current content of the file.
2.  **Compare** the User's request vs the Current Content.
3.  **Identify Conflicts**:
    - Will this break existing constraints?
    - Is this a "Breaking Change" or just an "Enhancement"?
4.  **Formulate Recommendation**:
    - _Adapt_: "I recommend creating a new file `custom-X.md` to avoid breaking standard X."
    - _Override_: "This helps matches your specific need, but removes the safety check Y."

## Step 3: User Confirmation

> 💡 **Tip**: You must be explicit about what will change.

1.  **Notify User** with a summary of your analysis.
    - If **New**: "I will create a new rule `<filename>` that `<does X>`."
    - If **modifying**: "I will modify `<filename>`. \n**Current**: `<summary>`\n**Proposed**: `<summary>`\n**Impact**: `<side effects>`"
2. Pause only when a user decision is required.

## Step 4: Execution

1.  Perform the file operation (create or update target file).
2.  **Validate**: Read the file back to ensure syntax is correct (Markdown/YAML frontmatter).
3.  **Register**: If it's a rule, remind the user if they need to manually activate it (unless it's `always_on`).

## Step 5: Verification

1. Check if the customization works as expected (if possible, by running a dry-run or asking user to test).
