---
project_name: StaffKit
project_type: existing-project
last_updated: 2026-03-15
status: desktop-native
---

# Project Context (StaffKit)

## Current State

StaffKit is a native desktop application at the repository root.

Primary stack:
- Tauri v2
- Rust backend
- React + TypeScript frontend
- Vite
- SQLite

## Product Direction

- Keep StaffKit as a desktop-first native application.
- Do not mix this workspace with external web-project planning or web runtime concerns.
- Treat `src/` and `src-tauri/` as the only active application code paths.

## Source Of Truth

- `README.md`
- `Note.md`
- `QUALITY.md`
- `CheckSecurity.md`

## Core Business Rules

- Staff data is grouped into Employee list, Onboarding, Offboarding, and Internal Movement.
- Excel import is the main input path.
- `EE.ID` is the unique business key for merge and update.
- Dynamic columns must be normalized before saving.
- Column preferences are saved per logged-in user profile.
- Bulk edit and group move are admin-only flows.

## Engineering Rules

- Preserve native desktop behavior and Tauri IPC patterns unless a task explicitly changes them.
- Prefer small, safe refactors around feature boundaries.
- Do not introduce web-only architecture into this repo.
- Keep SQLite as the active app database unless the user explicitly requests a data-layer change.
- Keep secrets out of commits.
- Run desktop quality checks when changes are substantial.

## Important Runtime Paths

- Frontend app: `src/`
- Desktop backend: `src-tauri/`
- Tauri config: `src-tauri/tauri.conf.json`
- Desktop API bridge: `src/services/staff-api.ts`

## Execution Style

- Small changes: implement directly after reading nearby code.
- Feature-level work: read `Note.md`, inspect relevant frontend and Rust modules, then implement end-to-end.
- Preserve existing naming and UI behavior unless the user asks for a redesign.

## Review Guidance

- Review is expected for cross-file, user-visible, database, import, auth, backup, or security-sensitive changes.
- Validate feedback against the current desktop codebase and the source-of-truth docs above.
