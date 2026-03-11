---
name: nextjs-fullstack
description: >
  Use when building or modifying the Staff Kit Next.js 15 fullstack web app.
  Covers App Router structure, API Routes, Server Components, data fetching patterns,
  middleware, and integration with Prisma + NextAuth.
---

# Next.js 15 Fullstack — Staff Kit

## Project conventions

- **Framework**: Next.js 15, App Router only. No Pages Router.
- **Language**: TypeScript strict mode (`"strict": true` in tsconfig).
- **Styling**: Tailwind CSS v3. No inline styles.
- **State**: TanStack Query (React Query) v5 for server state. Zustand for UI-only global state.
- **Forms**: React Hook Form + Zod resolver.
- **Icons**: Lucide React (already used in desktop app).

## Directory structure rules

```
src/
  app/                  # Next.js App Router
    (auth)/             # Route group — unauthenticated
      login/page.tsx
    (app)/              # Route group — authenticated, layout wraps auth check
      dashboard/
      employees/
      settings/
    api/                # API Routes (route handlers)
      auth/
      employees/
      teams/
      accounts/
      columns/
      backup/
      health/route.ts
    layout.tsx           # Root layout (fonts, providers)
  components/
    ui/                  # Primitive, stateless components (Button, Input, Badge...)
    features/            # Feature-specific composed components
  hooks/                 # Client-side React hooks
  services/
    api-client.ts        # fetch() wrapper — centralized base URL, error handling, auth headers
  lib/
    auth.ts              # NextAuth config (DO NOT put in app/api/auth)
    prisma.ts            # Prisma singleton (prevent hot-reload leak)
    utils.ts
    constants.ts
    validations/         # Zod schemas — one file per domain
      employee.ts
      account.ts
  types/                 # TypeScript types (no Tauri-specific stuff)
  middleware.ts          # Route protection (reads NextAuth session)
```

## API Route conventions

Every API route handler must follow this pattern:

```typescript
// src/app/api/employees/route.ts
import { NextRequest, NextResponse } from "next/server"
import { requireSession, requireRole } from "@/lib/auth"
import { prisma } from "@/lib/prisma"
import { z } from "zod"

const CreateSchema = z.object({
  employeeId: z.string().min(1),
  fullName: z.string().min(1),
  // ...
})

export async function POST(req: NextRequest) {
  // 1. Auth guard
  const session = await requireSession()
  requireRole(session, ["ADMIN", "SUPER_ADMIN"])

  // 2. Parse + validate input
  const body = await req.json()
  const parsed = CreateSchema.safeParse(body)
  if (!parsed.success) {
    return NextResponse.json({ error: "Invalid input", details: parsed.error.flatten() }, { status: 400 })
  }

  // 3. Business logic
  try {
    const employee = await prisma.employee.create({ data: parsed.data })
    return NextResponse.json(employee, { status: 201 })
  } catch (error) {
    return handleApiError(error)
  }
}
```

## Standard error response shape

```typescript
// Always return this shape for errors
{ error: string; code?: string; details?: unknown }
```

HTTP status codes:
- `400` — Validation error / bad input
- `401` — Not authenticated
- `403` — Authenticated but insufficient role
- `404` — Resource not found
- `409` — Conflict (duplicate key)
- `500` — Internal server error (log it, don't expose internals)

## Middleware — route protection

```typescript
// src/middleware.ts
import { auth } from "@/lib/auth"

export default auth((req) => {
  const isLoggedIn = !!req.auth
  const isAuthPage = req.nextUrl.pathname.startsWith("/login")
  const isApiRoute = req.nextUrl.pathname.startsWith("/api")

  if (isApiRoute) return  // API routes handle their own auth
  if (!isLoggedIn && !isAuthPage) {
    return Response.redirect(new URL("/login", req.url))
  }
  if (isLoggedIn && isAuthPage) {
    return Response.redirect(new URL("/dashboard", req.url))
  }
})

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
}
```

## Server vs Client Components

- Default to **Server Components**. Add `"use client"` only when needed (event handlers, hooks, browser APIs).
- Data fetching: prefer `async` Server Components with `await prisma.xxx()` directly.
- For interactive tables/forms: use Client Components with React Query.

## Prisma singleton (prevent connection leak in dev)

```typescript
// src/lib/prisma.ts
import { PrismaClient } from "@prisma/client"

const globalForPrisma = globalThis as unknown as { prisma: PrismaClient }

export const prisma =
  globalForPrisma.prisma ??
  new PrismaClient({
    log: process.env.NODE_ENV === "development" ? ["query", "error", "warn"] : ["error"],
  })

if (process.env.NODE_ENV !== "production") globalForPrisma.prisma = prisma
```

## React Query setup

```typescript
// Use for all client-side data fetching
const { data, isLoading, error } = useQuery({
  queryKey: ["employees", filters],
  queryFn: () => apiClient.employees.list(filters),
  staleTime: 30_000,  // 30 seconds
})
```

## DO NOT

- Do NOT use `fetch` directly in components — always use `api-client.ts`
- Do NOT use `useEffect` for data fetching — use React Query
- Do NOT put secrets in client components or client-side code
- Do NOT use `any` TypeScript type — use `unknown` and narrow
- Do NOT skip Zod validation in API routes
