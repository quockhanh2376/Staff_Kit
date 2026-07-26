---
name: staffkit-delivery-verification
description: Use before handoff or release/delivery review to select Staff Kit's narrowest relevant verification and report exact evidence. It never runs automatically or blocks a session.
metadata:
  upstream: https://github.com/affaan-m/ECC
  upstream_commit: c714dc56540ec514271fc18820f228204cd2a3c1
  adaptation: Staff_Kit project-local adapter
---

# Staff Kit Delivery Verification

Use Staff Kit's existing commands; do not add generic ECC commands, hooks, scripts, coverage thresholds, or background checks.

## Instruction hierarchy

1. Current user request
2. Safety and Codex harness instructions
3. `AGENTS.md`
4. Feature-specific instructions
5. Approved specification or implementation plan
6. Existing Staff Kit/Superpowers workflow
7. This supporting skill
8. Generic defaults

## Select checks by changed surface

| Changed surface | First relevant check | Required widening |
| --- | --- | --- |
| `src/` only | `npm run lint` or `npm run typecheck`, whichever is narrower | `npm run check:frontend`; product-code handoff requires `npm run check:quality` |
| `src-tauri/` | `npm run check:tauri` | `npm run test:tauri` when behavior is covered or changed; product-code handoff requires `npm run check:quality` |
| Frontend and backend | narrow checks above | `npm run check:quality` |
| Instructions/docs/config only | static path/link/frontmatter/diff audit | do not run product quality gate solely for appearance |

Use the shared-Cargo-target npm commands rather than inventing direct platform-specific replacements. If a check cannot run, report why and what was inspected instead. Review `git diff` before handoff and confirm scope, no secrets, and no unrelated product-code changes.

## Handoff report

List every command/check as `PASS`, `FAIL`, or `NOT RUN` with a reason; state changed surfaces, remaining risks, and whether the result is ready for the requested delivery. Never declare completion without this evidence.

## Attribution

Adapted from ECC `skills/verification-loop` at commit `c714dc56540ec514271fc18820f228204cd2a3c1`; its Claude hooks, generic commands, and automatic/continuous enforcement were removed.
