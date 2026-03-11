## Why

Staff Kit currently runs as a Windows desktop app, which makes centralized deployment, shared updates, and multi-user operations harder than they need to be. We need a web foundation first so the team can migrate feature-by-feature to the target stack without losing product clarity, quality, or operational control.

## What Changes

- Establish the first implementation-ready web foundation for Staff Kit in line with `ConvertWEB.md`.
- Define the baseline web application shell using Next.js 16 stable, TypeScript strict, and the agreed deployment/runtime stack.
- Define authentication and role-access foundations needed before employee-facing features can move to the web app.
- Define the operational baseline for structured logging, testing, health checks, maintainability, and scale readiness.
- Carry forward UX constraints from the start: default English locale, bilingual EN/VI readiness, and light/dark mode support.
- Keep the current desktop app as the behavior reference during migration; this change does not retire the desktop app yet.
- Non-goals for this change: full employee CRUD migration, Excel import implementation, backup/restore implementation, and the final SQLite-to-PostgreSQL migration execution.

## Capabilities

### New Capabilities
- `web-platform-foundation`: Create the base web application structure, shell, configuration, and UI/runtime conventions for the new Staff Kit web app.
- `auth-and-access-foundation`: Define the initial authentication, role model, protected routing, and seeded admin access required for the web baseline.
- `ops-and-quality-foundation`: Define the delivery, observability, health, testing, and maintainability baseline required before scaling implementation.

### Modified Capabilities
- None.

## Impact

- Affects future web app structure, environment configuration, authentication flow, deployment design, and engineering workflow.
- Establishes the first OpenSpec contract for the web migration roadmap.
- Influences future implementation across frontend, backend, database, CI/CD, testing, and production support.
