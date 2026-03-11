## 1. Web workspace bootstrap

- [ ] 1.1 Create the new Staff Kit web workspace with Next.js 16 stable, TypeScript strict, Tailwind, the agreed base folder structure, and a Node.js 20.9+ baseline
- [ ] 1.2 Add runtime environment validation, shared config modules, `next.config.ts` with `typedRoutes: true`, and baseline project scripts for local development
- [ ] 1.3 Add Docker Compose baseline services for app, PostgreSQL, and Redis with documented local startup flow aligned with the Node.js 20.9+ runtime baseline

## 2. Auth and access foundation

- [ ] 2.1 Model the baseline account and role schema in Prisma aligned with the current Staff Kit account model
- [ ] 2.2 Implement credentials-based sign-in with NextAuth v5 and seeded bootstrap admin access
- [ ] 2.3 Add protected shell routing, minimal `proxy.ts` gating where justified, and server-side API role guards for the baseline web app

## 3. Shell UX foundation

- [ ] 3.1 Build the base public login route and protected app shell with root-route redirects
- [ ] 3.2 Add localization-ready text structure with English default and Vietnamese support path
- [ ] 3.3 Add shared theme tokens and persistence for both light mode and dark mode in the app shell

## 4. Ops and observability foundation

- [ ] 4.1 Implement the structured logging contract with request ID, actor context, action context, safe error logging, and baseline `instrumentation.ts` / `instrumentation-client.ts`
- [ ] 4.2 Add health/readiness endpoint coverage for the app and its required dependencies
- [ ] 4.3 Add deployment-aligned configuration placeholders for Nginx and future production hardening, including PgBouncer planning hooks

## 5. Quality gates and validation

- [ ] 5.1 Add explicit `eslint`, `tsc --noEmit`, unit test, smoke E2E, and `next build` baseline commands for the web workspace
- [ ] 5.2 Configure CI to run the explicit quality gates on the web workspace without relying on `next lint` or implicit build linting
- [ ] 5.3 Update web migration docs to reference the OpenSpec change and document how to validate the new foundation
