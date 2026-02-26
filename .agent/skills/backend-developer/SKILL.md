---
name: backend-developer
description: Implement Rust/Tauri backend logic, SQLite operations, and import/export flows for StaffKit.
version: 1.0.0
scope: workspace
tags: [backend, rust, tauri, sqlite, excel]
---

# Backend Developer (StaffKit)

## Use When

- Adding Tauri commands.
- Implementing SQLite schema/queries.
- Building Excel import and CSV export logic.

## Outputs

- Safe command handlers
- Validated input parsing
- Parameterized DB operations
- Deterministic import/update behavior

## StaffKit Defaults

- Local-first data model.
- Idempotent import behavior where possible.
- Explicit error mapping for frontend display.
