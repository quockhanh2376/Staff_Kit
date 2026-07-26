---
name: staffkit-research-first
description: Use for version-sensitive Staff Kit investigations, new dependencies, Tauri/React/Rust APIs, or when local evidence does not settle an implementation decision.
metadata:
  upstream: https://github.com/affaan-m/ECC
  upstream_commit: c714dc56540ec514271fc18820f228204cd2a3c1
  adaptation: Staff_Kit project-local adapter
---

# Staff Kit Research First

Use this supporting skill only when research is needed to make a decision. It does not replace Staff Kit instructions, plans, or validation.

## Instruction hierarchy

1. Current user request
2. Safety and Codex harness instructions
3. `AGENTS.md`
4. Feature-specific instructions
5. Approved specification or implementation plan
6. Existing Staff Kit/Superpowers workflow
7. This supporting skill
8. Generic defaults

## Method

1. Establish the question and the decision it must unblock.
2. Read the smallest relevant local surface first: nearby code, tests, `package.json`, `src-tauri/Cargo.toml`, and existing Staff Kit guidance.
3. Record the exact installed version before researching an API. Prefer official documentation for Tauri, Rust crates, React, TypeScript, Vite, and SQLite/rusqlite.
4. Separate each conclusion into **fact** (with file/version/source), **inference**, or **hypothesis**. Do not present a hypothesis as an API guarantee.
5. Compare practical options only when a decision remains open; include maintenance, license, dependency cost, Windows/PowerShell compatibility, and impact on the desktop data pipeline.
6. State the smallest verification step that would confirm the recommendation.

Do not add an MCP, dependency, external service, or global configuration merely to perform research. State unavailable search channels honestly. Do not send secrets, employee data, recovery codes, or database contents to external tools.

## Output

Return: question, local evidence, external sources (if used), facts/inferences/hypotheses, recommendation, and verification step. Stop once the implementation decision is clear.

## Attribution

Adapted from ECC `skills/search-first` and `skills/documentation-lookup` at commit `c714dc56540ec514271fc18820f228204cd2a3c1`; rewritten for Staff Kit without ECC agents or MCP requirements.
