## Context

Staff Kit is currently a brownfield desktop application built with Tauri, Rust, React, and SQLite. The target direction is a web application that supports centralized deployment, multi-user access, stronger operational visibility, and a cleaner long-term maintenance path without breaking the current desktop workflow during migration.

This change establishes the web foundation only. It prepares the minimum architecture and delivery baseline needed to start implementing web features in controlled phases while using the desktop app as the functional reference. The main planning source remains `ConvertWEB.md`, which already defines the target stack, deployment model, scale target, security expectations, bilingual/theme requirements, structured logging expectations, and the upgraded Next.js 16 stable baseline.

## Goals / Non-Goals

**Goals:**
- Bootstrap the new Staff Kit web application foundation on the agreed stack.
- Establish the base app shell, route structure, environment/config patterns, and shared engineering conventions.
- Establish authentication and role-access foundations required before business features can migrate.
- Establish delivery and operational foundations: Dockerized local stack, health checks, structured logs, test scaffolding, and CI quality gates.
- Bake in English-default localization readiness, EN/VI support, and light/dark support from the start.

**Non-Goals:**
- Re-implement full employee CRUD, import, backup/restore, or team management in this change.
- Execute the full SQLite-to-PostgreSQL migration in this change.
- Retire or rewrite the current desktop application during this change.
- Lock every future implementation detail that can be deferred until later feature slices.

## Decisions

### 1. Build the web app as a separate foundation workspace

The migration foundation SHOULD be bootstrapped in a dedicated web workspace rather than by mutating the existing Tauri code directly. This keeps the desktop product stable, preserves a clean migration boundary, and makes it easier to stage feature parity incrementally.

Alternatives considered:
- Rework the current Tauri repo in place: rejected because it mixes legacy and target concerns too early and increases migration risk.
- Wait to create a web workspace until later phases: rejected because auth, infra, and quality baselines need a real target application to anchor future work.

### 2. Use Next.js 16 stable App Router as the single frontend + API foundation

The web baseline SHOULD use Next.js 16 stable App Router with route handlers for backend endpoints. This matches the planned stack, keeps the system TypeScript-first, reduces cross-service complexity for the initial migration, and aligns with the target operational model described in `ConvertWEB.md`. The baseline SHOULD also adopt `typedRoutes: true` and standardize on Node.js 20.9+ across development, CI, Docker, and production.

Alternatives considered:
- Separate frontend and backend services: rejected for the initial baseline because it adds deployment and contract overhead too early.
- Retain Rust as a separate backend: rejected because the domain does not justify that complexity for the target web deployment.
- Build on Next.js canary behavior: rejected because Staff Kit needs a stable foundation for phased migration and maintenance.

### 3. Establish auth and authorization before feature migration

The foundation MUST implement credentials-based authentication, seeded bootstrap admin access, role-aware route protection, and API-level authorization boundaries before employee features move over. This prevents later web slices from inventing incompatible auth patterns. `proxy.ts` MAY be used for minimal edge gating and redirect shaping, but it MUST NOT become the primary authorization layer; authoritative checks stay in server-side route handlers, actions, and auth utilities.

Alternatives considered:
- Delay auth until employee features are started: rejected because every protected workflow would otherwise need rework.
- Start with mock auth only: rejected because it weakens testability and hides deployment/security issues.

### 4. Make observability a first-class concern from the first web slice

The web foundation MUST define a structured logging contract and trace context from day one. Critical actions and API requests need logs with request ID, actor context, module/action, outcome, and latency so debugging and production support remain manageable as the system scales. `instrumentation.ts` and `instrumentation-client.ts` SHOULD be part of the baseline so observability hooks are established before feature sprawl begins.

Alternatives considered:
- Add logs later after feature parity: rejected because retrofitting consistent logs across many features is slower and less reliable.
- Use ad hoc console logging: rejected because it is not maintainable or operations-friendly.

### 5. Treat localization and theme as foundational primitives, not polish work

The app shell MUST default to English while being localization-ready for EN/VI and compatible with both light and dark modes. Adding these constraints late would force rework across shared layouts, copy, tokens, and persistence behavior.

Alternatives considered:
- Deliver English-only/light-only first: rejected because it would create avoidable rework in the shared shell.

### 6. Keep delivery reproducible with container-first local and production baselines

The foundation SHOULD define Docker Compose-based local development and a production-aligned deployment baseline with PostgreSQL, Redis, app service, and Nginx, while reserving PgBouncer for the production path. This balances developer ergonomics with realistic operations. CI and local verification SHOULD run explicit lint, typecheck, tests, and `next build` rather than relying on removed or implicit framework behavior such as `next lint`.

Alternatives considered:
- Local bare-metal services only: rejected because it drifts from deployment reality.
- Full production hardening in the first implementation slice: partially deferred because the initial foundation should stay small enough to complete.

## Risks / Trade-offs

- [Migration sprawl] -> Mitigation: keep this change strictly limited to foundation concerns and push CRUD/import/data migration into later changes.
- [Desktop and web behavior diverge during migration] -> Mitigation: use the desktop app as the functional reference and call out any intended behavior deviations explicitly in future specs.
- [Auth foundations may need adjustment when more domain details arrive] -> Mitigation: keep role and session layers modular and aligned with the existing account model.
- [Operational overhead from Redis, Nginx, and Docker may slow the first slice] -> Mitigation: keep local bootstrap scripted and reserve production-only concerns such as PgBouncer tuning for later hardening work.
- [Localization and theme support can inflate scope] -> Mitigation: require foundation readiness, not complete translated content for every future feature in this change.

## Migration Plan

1. Create the web foundation workspace and baseline project structure.
2. Add environment validation, shared config, `next.config.ts` baseline, Docker Compose services, and development scripts.
3. Implement auth foundation, seeded bootstrap admin, protected shell routing, server-side role guards, and minimal `proxy.ts` behavior only where justified.
4. Add baseline localization/theme primitives, logging contract, `instrumentation.ts`, `instrumentation-client.ts`, health endpoints, and test/CI scaffolding.
5. Validate the foundation with typecheck, lint, unit tests, and at least one smoke-level E2E path.
6. Keep the desktop app unchanged while future feature changes migrate on top of this web baseline.

Rollback strategy:
- If the foundation is not ready, the desktop app remains the active product and the web workspace can be paused without operational impact.
- Because the change is isolated to the new web foundation, rollback is primarily reverting or shelving the web workspace without touching desktop runtime behavior.

## Open Questions

- Should the web app live as a new folder inside this repository or as a separate repository once initial scaffolding is complete?
- Which localization library should back EN/VI support in the web shell, assuming Next.js 16 and server/client split constraints?
- How much of the initial admin/settings surface should be included in the first web smoke path versus deferred to the next change?
