---
name: staffkit-security-review
description: Use on request, or when reviewing Staff Kit authentication, authorization, Tauri IPC, Axum LAN, import, filesystem, backup, database, or sensitive-data changes. Review only; do not auto-fix.
metadata:
  upstream: https://github.com/affaan-m/ECC
  upstream_commit: c714dc56540ec514271fc18820f228204cd2a3c1
  adaptation: Staff_Kit project-local adapter
---

# Staff Kit Security Review

This is an on-demand evidence-based review. It supplements, and never replaces, `CheckSecurity.md`, `QUALITY.md`, and Staff Kit instructions. It does not change code unless the user separately asks for a fix.

## Instruction hierarchy

1. Current user request
2. Safety and Codex harness instructions
3. `AGENTS.md`
4. Feature-specific instructions
5. Approved specification or implementation plan
6. Existing Staff Kit/Superpowers workflow
7. This supporting skill
8. Generic defaults

## Review scope

Establish the diff and read only relevant surrounding code. Review applicable boundaries:

- Tauri command registration/wrappers, IPC DTOs, and frontend invokes
- Axum LAN routes, network binding, token/session lifecycle, and error exposure
- local authentication, Argon2 password/recovery-code handling, and role authorization
- SQLCipher setup, rusqlite parameterization, transactions, backup/snapshot/restore, and import staging
- Excel/file input limits and parsing, paths, export/import destinations, and traversal risks
- React dynamic HTML/injection, localStorage sensitive data, logs, secrets, and dependency changes
- Rust `unsafe`, panics on recoverable paths, and untrusted deserialization

For each applicable boundary, trace trust source -> validation/authorization -> persistence or side effect -> returned/logged data. Treat employee data, authentication material, database files, backups, and LAN requests as sensitive.

## Report

Report findings only, ordered `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, with `file:line`, evidence, impact, and a bounded recommendation. Explicitly report clean lanes and lanes not reviewed. Do not claim a scan or command ran when it did not.

No automatic hooks, fixes, dependency installation, telemetry, or external scanners are part of this skill.

## Attribution

Adapted from ECC `skills/security-review` at commit `c714dc56540ec514271fc18820f228204cd2a3c1`; generic web, payment, blockchain, Supabase, and auto-remediation material was removed.
