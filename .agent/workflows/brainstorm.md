> **LEGACY REFERENCE ONLY.** Not authoritative for Codex. `AGENTS.md`, `docs/CODEX_WORKFLOW.md`, and project-local Superpowers override this file.

---
description: Analyze ideas with the user and create preliminary high-level documents (Roadmap, PRD).
---

# Brainstorm Workflow

> [!IMPORTANT]
> **MANDATORY**: Read `.agent/rules/documents.md` before creating any document.

---

## MCP Usage Guidelines

| MCP Tool                                     | When to Use                                            | Example                                 |
| :------------------------------------------- | :----------------------------------------------------- | :-------------------------------------- |
| Optional structured reasoning, when available | Analyze requirements, feature dependencies, trade-offs | Break down ambiguous requests           |
| Official documentation search, when available | Find library documentation                              | "mermaid js"                            |
| Available research capability                 | Research library patterns, APIs, best practices         | "How to structure a Vite React feature" |
| `search_web`                                 | Proactive research for implementation patterns         | "best architecture for agentic systems" |

---

## Step 1: Deep Research

// turbo

> 💡 **MANDATORY**: Follow `.agent/rules/research.md` before starting any ideation.

1. **Invoke `[research]`** (via `search_web` + `read_url_content`) to:
   - Identify 5-10 key trends in the project's domain.
   - Find "best-in-class" examples of similar products.
   - Identify common pitfalls and modern "Wow Factors".
2. Create `research-insights.md` artifact in `docs/050-Research/`.
3. Pause only when a user decision is required.

---

## Document Priority Order

```
Priority 0: Roadmap       ← Project Planning & Timeline
Priority 1: PRD           ← Strategic Overview
```

---

## Step 2: Clarification & Understanding

**Role: Product Manager**

> [!NOTE]
> This step is **MANDATORY**. Do NOT proceed without user confirmation.

> 💡 Use an available structured-reasoning capability for ambiguous or complex requests.

1. **Invoke `[product-manager]` skill** to:
   - Summarize understanding
   - Create clarification questions
2. Create `clarification-questions.md` artifact
3. Pause only when a user decision is required.

---

## Step 3: Create Roadmap

// turbo

> 💡 Use an available structured-reasoning capability for phased planning and risk assessment.

1. **Invoke `[product-manager]` skill** to draft:
   - Project timeline and milestones
   - Phase breakdown (MVP, v1.0, v2.0)
   - Key deliverables per phase
2. Create `draft-roadmap.md` artifact
3. After approval → Save to `docs/010-Planning/Roadmap-{ProjectName}.md`
4. Pause only when a user decision is required.

---

## Step 4: Create PRD

// turbo

1. **Invoke `[product-manager]` skill** to draft:
   - Business objectives and success metrics
   - Target audience/user personas
   - Feature prioritization (MoSCoW)
2. Create `draft-prd.md` artifact
3. After approval → Save to `docs/020-Requirements/PRD-{ProjectName}.md`
4. Pause only when a user decision is required.

---

## Step 5: Transition to Documentation

1. Present summary of created artifacts (Roadmap, PRD).
2. Suggest next step: Run `/documentation` to generate detailed specifications (SDD, Epics, Stories).
