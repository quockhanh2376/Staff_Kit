---
name: code-quality
description: >
  Use when writing, reviewing, or refactoring any code in the Staff Kit web app.
  Enforces TypeScript strict mode, ESLint rules, consistent error handling,
  naming conventions, and maintainability standards for a scalable codebase.
---

# Code Quality Standards — Staff Kit Web App

## TypeScript rules (mandatory)

- `"strict": true` in `tsconfig.json` — no exceptions
- No `any` type. Use `unknown` + type narrowing, or explicit types
- No `as` type assertions except when narrowing from `unknown`
- Every function must have explicit return type annotation
- Prefer `type` over `interface` for data shapes; use `interface` for OOP extension
- Use `readonly` for arrays/objects that shouldn't be mutated

```typescript
// ❌ Bad
const getEmployee = async (id: any) => {
  return await prisma.employee.findUnique({ where: { id } }) as any
}

// ✅ Good
async function getEmployee(id: number): Promise<Employee | null> {
  return prisma.employee.findUnique({ where: { id } })
}
```

## ESLint config (.eslintrc.json)

```json
{
  "extends": [
    "next/core-web-vitals",
    "next/typescript",
    "plugin:@typescript-eslint/recommended-type-checked"
  ],
  "rules": {
    "@typescript-eslint/no-explicit-any": "error",
    "@typescript-eslint/no-unused-vars": ["error", { "argsIgnorePattern": "^_" }],
    "@typescript-eslint/explicit-function-return-type": "warn",
    "@typescript-eslint/no-floating-promises": "error",
    "no-console": ["warn", { "allow": ["warn", "error"] }],
    "prefer-const": "error",
    "no-var": "error"
  }
}
```

## Naming conventions

| Item | Convention | Example |
|------|-----------|---------|
| Files (components) | PascalCase | `EmployeeTable.tsx` |
| Files (hooks, utils) | camelCase | `useEmployeeList.ts` |
| Files (API routes) | kebab-case folder | `employees/route.ts` |
| React components | PascalCase | `function EmployeeTable()` |
| Hooks | `use` prefix | `useEmployeeList` |
| Constants | SCREAMING_SNAKE | `DEFAULT_PAGE_SIZE = 50` |
| Database models | PascalCase singular | `Employee`, `LocalAccount` |
| API response types | `XxxResponse` | `EmployeeListResponse` |
| Zod schemas | `XxxSchema` | `CreateEmployeeSchema` |

## Error handling pattern

### API routes
```typescript
// src/lib/api-error.ts
export class ApiError extends Error {
  constructor(public status: number, message: string, public code?: string) {
    super(message)
  }
}

export function handleApiError(error: unknown): Response {
  console.error("[API Error]", error)  // Log full error server-side

  if (error instanceof ApiError) {
    return NextResponse.json(
      { error: error.message, code: error.code },
      { status: error.status }
    )
  }
  if (error instanceof Prisma.PrismaClientKnownRequestError) {
    if (error.code === "P2002") {
      return NextResponse.json({ error: "Duplicate value" }, { status: 409 })
    }
  }
  // Don't expose internal errors to client
  return NextResponse.json({ error: "Internal server error" }, { status: 500 })
}
```

### Client-side
```typescript
// src/services/api-client.ts
async function call<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`/api/${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new ApiClientError(res.status, body.error ?? "Request failed")
  }
  return res.json() as Promise<T>
}
```

## Component architecture

```
components/
  ui/              # Dumb, reusable, no business logic
    Button.tsx
    Input.tsx
    Table.tsx
    Badge.tsx
  features/        # Smart, connected to API/state
    EmployeeTable.tsx   ← uses useQuery, calls api-client
    SettingsPanel.tsx
    ImportModal.tsx
```

Rules:
- `ui/` components: no `useQuery`, no `fetch`, no business logic
- `features/` components: no raw HTML/CSS concerns — delegate to `ui/`
- Props always typed, no `any`

## API client pattern

```typescript
// src/services/api-client.ts — organized by domain
export const apiClient = {
  employees: {
    list: (params: EmployeeListParams) =>
      call<EmployeeListResponse>("employees?" + new URLSearchParams(params as any)),
    create: (data: CreateEmployeeInput) =>
      call<Employee>("employees", { method: "POST", body: JSON.stringify(data) }),
    update: (id: number, data: UpdateEmployeeInput) =>
      call<Employee>(`employees/${id}`, { method: "PATCH", body: JSON.stringify(data) }),
    delete: (id: number) =>
      call<void>(`employees/${id}`, { method: "DELETE" }),
  },
  accounts: {
    list: () => call<LocalAccount[]>("accounts"),
    // ...
  },
}
```

## File size limits

| File type | Max lines | Action if exceeded |
|-----------|-----------|-------------------|
| React component | 300 | Split into sub-components |
| API route handler | 100 | Extract service functions |
| Hook | 150 | Split into focused hooks |
| Utility module | 200 | Split by concern |

## Commit message convention

```
type(scope): short description

Types: feat | fix | refactor | test | chore | docs
Scope: auth | employees | teams | api | db | docker | tests

Examples:
feat(employees): add full-text search endpoint
fix(auth): handle password reset redirect correctly
test(employees): add E2E spec for import flow
chore(docker): add pgbouncer to prod compose
```

## Checklist before commit

- [ ] `npm run type-check` passes (no TypeScript errors)
- [ ] `npm run lint` passes (no ESLint errors)
- [ ] Unit tests pass for any new/modified API route
- [ ] No `console.log` left in code (only `console.error` allowed in catch)
- [ ] No hardcoded credentials or secrets
- [ ] No `TODO` without a GitHub issue number

## DO NOT

- Do NOT merge code with TypeScript errors
- Do NOT use `// @ts-ignore` or `// @ts-expect-error` without a comment explaining why
- Do NOT write business logic in React components — extract to hooks or services
- Do NOT duplicate validation logic between client and server — server is authoritative
