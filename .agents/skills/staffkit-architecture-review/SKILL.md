---
name: staffkit-architecture-review
description: Use for Staff Kit cross-layer design or diff review when ownership, boundary, dependency, or context scope is unclear. Produces a small evidence-backed review packet.
metadata:
  upstream: https://github.com/affaan-m/ECC
  upstream_commit: c714dc56540ec514271fc18820f228204cd2a3c1
  adaptation: Staff_Kit project-local adapter
---

# Staff Kit Architecture Review

Use this only for cross-feature or cross-layer review. It is not a competing planning process and does not replace Headroom, `daily_log.md`, worktrees, or existing subagent workflow.

## Instruction hierarchy

1. Current user request
2. Safety and Codex harness instructions
3. `AGENTS.md`
4. Feature-specific instructions
5. Approved specification or implementation plan
6. Existing Staff Kit/Superpowers workflow
7. This supporting skill
8. Generic defaults

## Focused method

1. State the review question and inspect the diff before broad reading.
2. Map only the affected ownership path: React feature/state -> `src/services/staff-api.ts` IPC DTO -> Tauri wrapper in `src-tauri/src/lib.rs` -> domain/db/LAN module -> SQLite/backup boundary.
3. Use `AGENTS.md` and nearby code to identify the owner. Keep `web/` read-only and out of the review unless the user explicitly includes it.
4. Check for boundary drift: UI bypassing the database pipeline, thick Tauri wrappers, untyped/changed IPC contracts, duplicated business rules, authorization bypass, import writes before validation, or backup/restore invariants bypassed.
5. Read the minimum necessary files; distinguish verified facts from assumptions. Do not introduce a second memory system or change existing architecture without approval.

## Review packet

Return: intent; diff map (added/modified/relevant untouched surfaces); ownership/data-flow map; risks by behavior/security/tests/docs; and focused follow-ups. Findings require file evidence. A clean review is valid.

## Attribution

Adapted from ECC's Codex navigation/diff-packet and `context-budget` patterns at commit `c714dc56540ec514271fc18820f228204cd2a3c1`; agent-app, memory, hook, and control-plane workflows were excluded.
