---
project_name: StaffKit
project_type: existing-project
last_updated: 2026-02-26
---

# Project Context (StaffKit)

## Stack

- Tauri v2
- Rust (backend commands, data import)
- React + TypeScript (UI)
- SQLite (local database)
- Excel input as primary source data

## Critical Rules

- Treat `Note.md` and `Tech_tags.md` as product/technical baseline.
- Keep implementation minimal and incremental; avoid introducing new frameworks unless required.
- `ExSource/` contains local Excel input data and must not be committed.
- `.env` is local-only; do not commit secrets.
- Prefer schema-safe DB changes and explicit migrations/upgrade paths.

## Data Boundaries

- Input: `ExSource/*.xlsx` (local source files)
- Runtime DB: local SQLite files
- Export: CSV output for users

## Execution Style

- For small scoped tasks: use quick implementation loop (context -> execute -> self-check -> review).
- For system-level changes: create a short tech spec first, then implement.
