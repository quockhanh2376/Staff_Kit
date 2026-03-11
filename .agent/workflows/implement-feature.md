---
description: Full-cycle feature implementation for Staff Kit web app — research, implement, test, commit
---

# /implement-feature workflow

Use this workflow to implement any new feature for the Staff Kit web app.
Follows the ConvertWEB.md phase plan.

## Steps

1. **Read context**
   - Read `ConvertWEB.md` to understand phase and requirements
   - Read relevant skill files (`nextjs-fullstack`, `security-web`, `code-quality`, etc.)
   - Read existing code in the area you're changing

2. **Create tech spec** (for feature-level work)
   - Brief spec: what endpoints, what DB changes, what UI changes
   - Identify Zod schemas needed
   - Identify role guards needed

3. **Database first**
   - Update `prisma/schema.prisma` if needed
   - Run `npx prisma migrate dev --name feature_name`
   - Run `npx prisma generate`

4. **API routes**
   - Create route handler in `src/app/api/[feature]/route.ts`
   - Add Zod validation + role guard + error handling (per `nextjs-fullstack` skill)
   - Write unit tests before moving to frontend: `tests/unit/api/[feature].test.ts`

// turbo
5. **Run unit tests**
   ```bash
   npx vitest run tests/unit/api/[feature].test.ts
   ```

6. **Frontend**
   - Update `src/services/api-client.ts` with new methods
   - Create/update React components and hooks
   - Use React Query for data fetching

7. **E2E tests**
   - Add Playwright spec in `tests/e2e/[feature].spec.ts`
   - Run: `npx playwright test [feature].spec.ts`

// turbo
8. **Type check + lint**
   ```bash
   npx tsc --noEmit && npm run lint
   ```

9. **Commit**
   - `feat(scope): description` per code-quality skill conventions
   - Push to GitHub

10. **Update ConvertWEB.md**
    - Mark completed tasks with `[x]` in the relevant Phase section
