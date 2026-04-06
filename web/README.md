# AssetDesk-Pro Web Bootstrap

This workspace is the isolated web-app baseline for AssetDesk-Pro.
It is intentionally independent from the legacy Staff Kit Tauri desktop app.

## Runtime Baseline

- Next.js 16 stable
- Node.js 20.9+
- Docker Desktop
- PostgreSQL 16
- Redis 7

## Local Development

1. Copy `.env.example` to `.env`.
2. Start the local stack:

```bash
npm run docker:up
```

3. Open `http://localhost:3000`.
4. Check `http://localhost:3000/api/health`.

## Run App on Host Machine

If you want the app process on Windows and only PostgreSQL/Redis in Docker:

```bash
docker compose up -d db redis
npm run dev
```

Keep the `.env` file pointed to the Docker host ports for host-machine development:

- PostgreSQL: `127.0.0.1:55432`
- Redis: `127.0.0.1:56379`

## Quality Gates

```bash
npm run lint
npm run typecheck
npm run build
```

Or run them together:

```bash
npm run check
```

## Current Bootstrap Scope

- Next.js 16 workspace with `typedRoutes: true`
- Docker Compose for app, PostgreSQL, and Redis
- baseline environment validation helpers
- minimal `proxy.ts`, `instrumentation.ts`, and `instrumentation-client.ts`
- `/api/health` for smoke checks
- seeded dev data for employees, assets, requests, approval reviews, and audit logs
- admin preview shell for dashboard, employees, assets, reviews, receive, return, and audit

Business modules, auth, Prisma schema, and production hardening will be layered on top next.
