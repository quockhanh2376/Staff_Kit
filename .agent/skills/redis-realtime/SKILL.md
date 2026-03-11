---
name: redis-realtime
description: >
  Use when implementing Redis caching, real-time notifications (Pub/Sub + SSE),
  or rate limiting in Staff Kit web app. Covers ioredis setup, Pub/Sub patterns,
  Server-Sent Events in Next.js App Router, and cache invalidation strategies.
---

# Redis — Real-time & Caching (Staff Kit)

## Setup (ioredis)

```bash
npm install ioredis
```

```typescript
// src/lib/redis.ts
import Redis from "ioredis"

const globalForRedis = globalThis as unknown as { redis: Redis }

export const redis =
  globalForRedis.redis ??
  new Redis(process.env.REDIS_URL!, {
    maxRetriesPerRequest: 3,
    lazyConnect: true,
  })

if (process.env.NODE_ENV !== "production") globalForRedis.redis = redis

// Separate client for subscriptions (can't share one connection)
export function createSubscriber(): Redis {
  return new Redis(process.env.REDIS_URL!)
}
```

---

## Real-time Notifications (Pub/Sub + SSE)

### Architecture

```
Admin saves employee
       ↓
API Route: redis.publish("notifications", JSON.stringify(event))
       ↓
All SSE connections receive message
       ↓
Each client's EventSource fires → UI updates instantly
```

### SSE Route (Next.js App Router)

```typescript
// src/app/api/notifications/stream/route.ts
import { NextRequest } from "next/server"
import { requireSession } from "@/lib/auth"
import { createSubscriber } from "@/lib/redis"

export async function GET(req: NextRequest) {
  const session = await requireSession()

  const encoder = new TextEncoder()
  const subscriber = createSubscriber()

  const stream = new ReadableStream({
    start(controller) {
      // Subscribe to all notifications channel
      subscriber.subscribe("notifications", (err) => {
        if (err) controller.close()
      })

      subscriber.on("message", (_channel, message) => {
        // Send as SSE format
        controller.enqueue(encoder.encode(`data: ${message}\n\n`))
      })

      // Heartbeat every 30s to keep connection alive
      const heartbeat = setInterval(() => {
        controller.enqueue(encoder.encode(`: heartbeat\n\n`))
      }, 30_000)

      req.signal.addEventListener("abort", () => {
        clearInterval(heartbeat)
        subscriber.disconnect()
        controller.close()
      })
    },
  })

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache, no-transform",
      Connection: "keep-alive",
    },
  })
}
```

### Publishing events from API routes

```typescript
// src/lib/notifications.ts
import { redis } from "@/lib/redis"

export type NotificationEvent =
  | { type: "employee_updated"; employeeId: number; updatedBy: string }
  | { type: "employee_deleted"; employeeId: number }
  | { type: "import_completed"; count: number; importedBy: string }
  | { type: "account_role_changed"; username: string; newRole: string }
  | { type: "data_restored"; restoredBy: string }

export async function publishNotification(event: NotificationEvent): Promise<void> {
  await redis.publish("notifications", JSON.stringify({
    ...event,
    timestamp: new Date().toISOString(),
  }))
}

// Usage in an API route after saving:
// await publishNotification({ type: "employee_updated", employeeId: 42, updatedBy: session.user.name })
```

### Client-side hook

```typescript
// src/hooks/useNotifications.ts
import { useEffect } from "react"
import { useQueryClient } from "@tanstack/react-query"

export function useNotifications() {
  const queryClient = useQueryClient()

  useEffect(() => {
    const es = new EventSource("/api/notifications/stream")

    es.onmessage = (event) => {
      const notification = JSON.parse(event.data)

      // Invalidate relevant React Query cache based on event type
      switch (notification.type) {
        case "employee_updated":
        case "employee_deleted":
          queryClient.invalidateQueries({ queryKey: ["employees"] })
          break
        case "import_completed":
          queryClient.invalidateQueries({ queryKey: ["employees"] })
          break
        case "account_role_changed":
          queryClient.invalidateQueries({ queryKey: ["accounts"] })
          break
      }
    }

    es.onerror = () => {
      // Auto-reconnect: EventSource retries automatically
    }

    return () => es.close()
  }, [queryClient])
}

// Mount in root layout or dashboard layout:
// useNotifications() — runs once, all users auto-refresh on changes
```

---

## Rate Limiting (Redis-backed)

```typescript
// src/lib/rate-limit.ts
import { redis } from "@/lib/redis"

export async function checkRateLimit(
  key: string,
  limit: number,
  windowSeconds: number
): Promise<{ allowed: boolean; remaining: number }> {
  const redisKey = `rate:${key}`
  const count = await redis.incr(redisKey)

  if (count === 1) {
    await redis.expire(redisKey, windowSeconds)
  }

  return {
    allowed: count <= limit,
    remaining: Math.max(0, limit - count),
  }
}

// Usage in login API:
// const ip = req.headers.get("x-forwarded-for") ?? "unknown"
// const { allowed } = await checkRateLimit(`login:${ip}`, 10, 60)
// if (!allowed) return NextResponse.json({ error: "Too many attempts" }, { status: 429 })
```

---

## Caching (employee list, teams)

```typescript
// src/lib/cache.ts
import { redis } from "@/lib/redis"

const DEFAULT_TTL = 60  // seconds

export async function getCached<T>(key: string): Promise<T | null> {
  const val = await redis.get(key)
  return val ? (JSON.parse(val) as T) : null
}

export async function setCached<T>(key: string, value: T, ttl = DEFAULT_TTL): Promise<void> {
  await redis.setex(key, ttl, JSON.stringify(value))
}

export async function invalidateCache(pattern: string): Promise<void> {
  const keys = await redis.keys(pattern)
  if (keys.length > 0) await redis.del(...keys)
}

// Cache keys — centralize to avoid typos
export const CACHE_KEYS = {
  teams: "cache:teams",
  employeeList: (group: string, page: number) => `cache:employees:${group}:${page}`,
}

// Usage in API route:
// const cached = await getCached<TeamListResponse>(CACHE_KEYS.teams)
// if (cached) return NextResponse.json(cached)
// const teams = await prisma.team.findMany(...)
// await setCached(CACHE_KEYS.teams, teams, 300)  // 5-min cache

// After any team mutation:
// await invalidateCache("cache:teams*")
```

---

## Notification types for Staff Kit

| Event | Triggered when | Who sees it |
|-------|---------------|------------|
| `employee_updated` | Employee edited | All logged-in users |
| `employee_deleted` | Employee removed | All logged-in users |
| `import_completed` | Excel import done | All logged-in users |
| `account_role_changed` | Admin changes role | All logged-in users |
| `data_restored` | Super admin restores DB | All logged-in users |

---

## Docker (prod with password)

```yaml
redis:
  image: redis:7-alpine
  restart: unless-stopped
  volumes:
    - redisdata:/data
  command: redis-server --save 60 1 --requirepass ${REDIS_PASSWORD}

# .env
REDIS_URL=redis://:${REDIS_PASSWORD}@redis:6379
```

## DO NOT

- Do NOT share one Redis connection for both publish and subscribe — they block each other
- Do NOT cache user-specific sensitive data (password hashes, session tokens) in Redis
- Do NOT forget to call `subscriber.disconnect()` when SSE connection closes
- Do NOT use Redis as primary data store — PostgreSQL is authoritative
