---
project_name: StaffKit
project_type: existing-project
last_updated: 2026-03-11
status: migrating-to-web
---

# Project Context (StaffKit)

## Current State

StaffKit has two phases:
- **Desktop app (current/legacy)**: Tauri v2 + Rust + React + SQLite at `e:\Staff_Kit`
- **Web app (upcoming)**: Next.js 16 stable + PostgreSQL + Docker (to be built in new folder)

## Web App Stack (target)

| Layer | Tech |
|-------|------|
| Frontend + API | Next.js 16 stable, App Router, TypeScript strict |
| Styling | Tailwind CSS v3 |
| ORM | Prisma |
| Database | PostgreSQL 16 |
| Auth | NextAuth.js v5 (JWT, credentials) |
| State | TanStack Query v5 (server), Zustand (UI) |
| Cache + Realtime | Redis 7 (Pub/Sub + SSE notifications, rate limiting, cache) |
| Testing | Vitest (unit) + Playwright (E2E) |
| Deploy | Docker Compose + Nginx on self-managed VPS |
| CI | GitHub Actions |
| Runtime | Node.js 20.9+ |

## Critical Rules

- Target scale: **~1,000 concurrent users** (internal employee login)
- Use **PgBouncer** in production — do NOT let Prisma connect directly to PostgreSQL
- Every API route: Zod validation + role guard (defense in depth)
- Use `typedRoutes: true` in `next.config.ts`
- Use `proxy.ts` sparingly for minimal edge gating; do NOT make Proxy the primary auth/authorization layer
- Add `instrumentation.ts` and `instrumentation-client.ts` in the initial web baseline for observability hooks
- TypeScript strict mode — no `any`, no `@ts-ignore` without justification
- Node.js **20.9+** in dev, CI, Docker, and production
- CI must run explicit `eslint`, `tsc --noEmit`, tests, and `next build`; do NOT rely on `next lint` or assume `next build` lints automatically
- Unit tests for all API routes. E2E tests for all critical user paths
- Secrets only in `.env` — never committed
- VPS deploy — nginx reverse proxy, internal network only

## Deployment target

- Hosting: self-managed VPS (Linux)
- Domain: internal network subdomain (not public internet)
- SSL: self-signed cert or Let's Encrypt if domain available

## Data

- **Migrate ALL data** from SQLite → PostgreSQL (one-time migration before go-live)
- Excel imports: `.xlsx` only, max 50MB, server-side parsing
- Backups: `pg_dump` → download (Super Admin only)

## Roles

| Role | Permissions |
|------|-------------|
| `SUPER_ADMIN` | Everything + restore DB, reset all data |
| `ADMIN` | CRUD employees, manage accounts, backup |
| `USER` | Read-only employee view |

Default super admin account: `adman`

## Active Skills (use for web app work)

- `nextjs-fullstack` — App Router, API routes, proxy/instrumentation patterns
- `prisma-postgresql` — Schema, migrations, query patterns, PgBouncer
- `docker-vps-deploy` — Dockerfile, docker-compose, Nginx, deploy procedure
- `testing-vitest-playwright` — Unit tests, E2E tests, CI config
- `security-web` — Auth, authorization, validation, rate limiting, secrets
- `code-quality` — TypeScript, ESLint, naming, error handling
- `data-migration` — SQLite → PostgreSQL migration script
- `redis-realtime` — Redis Pub/Sub, SSE notifications, rate limiting, cache
- `shadcn-ui` — UI components
- `vercel-react-best-practices` — React/Next.js performance patterns
- `requesting-code-review` — Request review after major task batches, feature milestones, or before merge/release
- `receiving-code-review` — Process review feedback rigorously before implementing suggestions

## Execution style

- Break work by phase (see `ConvertWEB.md` for full plan)
- Small changes: implement directly
- Feature-level work: create brief tech spec → implement → test → review
- Each feature must include unit tests before marking complete

## Additional Superpowers Skills

### `requesting-code-review`

Use this skill in Staff Kit when:
- A major feature slice is complete, especially Auth, Employee Core, Import Excel, Backup/Restore, migration tooling, or release work
- An OpenSpec task group or phase milestone has been implemented and verified locally
- A complex bugfix is done and you want an independent technical check before continuing
- Before merging to `main`, cutting a release, or handing off a deployment-sensitive change
- After changes touching security, database schema/migrations, Docker/Nginx, Redis, CI, or production config

Review is optional for very small low-risk edits, but it is expected for anything cross-file, user-visible, security-sensitive, or deployment-sensitive.

### `receiving-code-review`

Use this skill in Staff Kit when:
- The user gives review comments in chat
- A reviewer or tool reports issues on a diff, plan, PR, or code change
- Feedback seems partially unclear, conflicting, or technically questionable
- A suggested fix may break existing desktop parity, web migration constraints, or previously approved architecture

For Staff Kit, do not blindly apply review feedback. First verify it against:
- `ConvertWEB.md`
- `openspec/` artifacts for the active change
- existing project conventions in `.agent/`
- actual codebase behavior and tests

If feedback is unclear, clarify before changing code. If feedback is wrong for this stack or conflicts with approved architecture, push back with technical reasoning instead of implementing it mechanically.
