---
name: security-web
description: >
  Use when implementing or reviewing any security-sensitive feature in Staff Kit web app.
  Covers authentication, authorization, input validation, rate limiting, CSRF, secrets
  management, and common web vulnerabilities relevant to an internal HR web app.
---

# Web Security — Staff Kit

## Authentication (NextAuth.js)

```typescript
// src/lib/auth.ts
import NextAuth from "next-auth"
import Credentials from "next-auth/providers/credentials"
import { prisma } from "@/lib/prisma"
import { verify } from "argon2"
import { z } from "zod"

export const { handlers, auth, signIn, signOut } = NextAuth({
  providers: [
    Credentials({
      async authorize(credentials) {
        const parsed = z.object({
          username: z.string().min(1).max(64),
          password: z.string().min(1).max(128),
        }).safeParse(credentials)
        if (!parsed.success) return null

        const account = await prisma.localAccount.findUnique({
          where: { username: parsed.data.username },
        })
        if (!account) return null  // Don't reveal whether username exists

        const valid = await verify(account.passwordHash, parsed.data.password)
        if (!valid) return null

        return {
          id: String(account.id),
          name: account.displayName,
          role: account.role,
          forcePasswordReset: account.forcePasswordReset,
        }
      },
    }),
  ],
  callbacks: {
    jwt({ token, user }) {
      if (user) {
        token.id = user.id
        token.role = (user as any).role
        token.forcePasswordReset = (user as any).forcePasswordReset
      }
      return token
    },
    session({ session, token }) {
      session.user.id = token.id as string
      session.user.role = token.role as string
      session.user.forcePasswordReset = token.forcePasswordReset as boolean
      return session
    },
  },
  session: { strategy: "jwt", maxAge: 8 * 60 * 60 },  // 8-hour session
  pages: { signIn: "/login" },
})
```

## Authorization helpers

```typescript
// src/lib/auth.ts (additions)
import { auth } from "@/lib/auth"
import { NextResponse } from "next/server"

export async function requireSession() {
  const session = await auth()
  if (!session?.user) {
    throw NextResponse.json({ error: "Unauthorized" }, { status: 401 })
  }
  return session
}

export function requireRole(session: Session, allowedRoles: string[]) {
  if (!allowedRoles.includes(session.user.role)) {
    throw NextResponse.json({ error: "Forbidden" }, { status: 403 })
  }
}

// Usage in route:
// const session = await requireSession()
// requireRole(session, ["ADMIN", "SUPER_ADMIN"])
```

## Role hierarchy

| Role | Can do |
|------|--------|
| `SUPER_ADMIN` | Everything + restore DB, reset all data |
| `ADMIN` | CRUD employees, manage accounts, backup |
| `USER` | Read-only employee view |

Enforce in middleware AND in each API route (defense in depth).

## Input validation (Zod — mandatory)

```typescript
// Every API route must validate with Zod before touching DB
const EmployeeCreateSchema = z.object({
  employeeId: z.string().min(1).max(50).regex(/^[A-Za-z0-9_-]+$/),
  fullName: z.string().min(1).max(200).trim(),
  email: z.string().email().optional().nullable(),
  // ...
})

// In route handler:
const parsed = EmployeeCreateSchema.safeParse(await req.json())
if (!parsed.success) {
  return NextResponse.json(
    { error: "Invalid input", details: parsed.error.flatten() },
    { status: 400 }
  )
}
```

## Rate limiting (login endpoint)

```typescript
// src/lib/rate-limit.ts
// Simple in-memory rate limiter (sufficient for internal network app)
// For production: use upstash/ratelimit with Redis if needed

const attempts = new Map<string, { count: number; resetAt: number }>()

export function checkRateLimit(key: string, limit: number, windowMs: number): boolean {
  const now = Date.now()
  const record = attempts.get(key)

  if (!record || record.resetAt < now) {
    attempts.set(key, { count: 1, resetAt: now + windowMs })
    return true
  }
  if (record.count >= limit) return false
  record.count++
  return true
}

// Apply to login API:
// if (!checkRateLimit(ip, 10, 60_000)) {
//   return NextResponse.json({ error: "Too many attempts" }, { status: 429 })
// }
```

## Password security

```typescript
// src/lib/password.ts
import { hash, verify } from "argon2"

// Hash settings for argon2id
const HASH_OPTIONS = {
  type: argon2.argon2id,
  memoryCost: 65536,   // 64MB
  timeCost: 3,
  parallelism: 1,
}

export async function hashPassword(password: string): Promise<string> {
  if (password.length < 6) throw new Error("Password too short")
  if (password.length > 128) throw new Error("Password too long")
  return hash(password, HASH_OPTIONS)
}

export async function verifyPassword(password: string, hash: string): Promise<boolean> {
  return verify(hash, password)
}
```

## Security headers (next.config.ts)

```typescript
// next.config.ts
const securityHeaders = [
  { key: "X-DNS-Prefetch-Control", value: "on" },
  { key: "X-Frame-Options", value: "SAMEORIGIN" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
  {
    key: "Content-Security-Policy",
    value: [
      "default-src 'self'",
      "script-src 'self' 'unsafe-eval' 'unsafe-inline'",  // Next.js needs these
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob:",
      "connect-src 'self'",
    ].join("; "),
  },
]

export default {
  async headers() {
    return [{ source: "/(.*)", headers: securityHeaders }]
  },
}
```

## File upload security (Excel import)

```typescript
// Validate uploaded files
const MAX_FILE_SIZE = 50 * 1024 * 1024  // 50MB

export function validateUploadedFile(file: File) {
  if (file.size > MAX_FILE_SIZE) throw new Error("File too large (max 50MB)")

  const allowed = [
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",  // .xlsx
    "application/vnd.ms-excel",  // .xls
  ]
  if (!allowed.includes(file.type)) throw new Error("Only .xlsx and .xls files are allowed")
}
```

## Secrets checklist

- [ ] `AUTH_SECRET`: minimum 32 random bytes (`openssl rand -base64 32`)
- [ ] `DB_PASSWORD`: minimum 20 chars, random
- [ ] Both in `.env` only — never hardcoded, never in source
- [ ] `.env` in `.gitignore`
- [ ] Only `.env.example` committed (with placeholder values)
- [ ] VPS: use Docker secrets or env file with restricted permissions (`chmod 600 .env`)

## DO NOT

- Do NOT use `MD5` or `bcrypt` — use `argon2id`
- Do NOT log passwords, tokens, or full error stacks to client
- Do NOT trust `Content-Type` header alone for file validation
- Do NOT expose internal error messages to API responses — log server-side, return generic message
- Do NOT use `dangerouslySetInnerHTML` — never needed in this app
- Do NOT store session data in localStorage — NextAuth uses httpOnly cookies by default
