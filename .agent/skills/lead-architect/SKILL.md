---
name: lead-architect
description: Define technical scope, boundaries, and architecture decisions for StaffKit features.
version: 1.0.0
scope: workspace
tags: [architecture, tauri, rust, react, sqlite]
---

# Lead Architect (StaffKit)

## Use When

- A feature spans multiple layers (UI + Rust command + DB).
- You need to define boundaries before implementation.

## Outputs

- Clear scope boundaries
- Target files/modules
- Data flow and integration points
- Risk list and decision log

## StaffKit Defaults

- Desktop-first, mobile-ready decisions.
- Prefer simple local-first architecture with SQLite.
- Reuse existing patterns before introducing new abstractions.
