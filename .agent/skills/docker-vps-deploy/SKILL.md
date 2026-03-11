---
name: docker-vps-deploy
description: >
  Use when setting up Docker infrastructure, writing Dockerfiles, docker-compose configs,
  or deploying Staff Kit web app to a self-managed VPS. Covers multi-stage builds,
  Nginx reverse proxy, health checks, secrets management, and deployment procedures.
---

# Docker + VPS Deploy — Staff Kit

## Docker architecture

```
Internet → Nginx (80/443) → Next.js app (3000) → PgBouncer (6432) → PostgreSQL (5432)
```

## Dockerfile (multi-stage)

```dockerfile
# ── Stage 1: deps ────────────────────────────────────────
FROM node:20-alpine AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --frozen-lockfile

# ── Stage 2: builder ─────────────────────────────────────
FROM node:20-alpine AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN npx prisma generate
ENV NEXT_TELEMETRY_DISABLED=1
RUN npm run build

# ── Stage 3: runner (production) ─────────────────────────
FROM node:20-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs

COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
COPY --from=builder /app/prisma ./prisma
COPY --from=builder /app/node_modules/.prisma ./node_modules/.prisma

USER nextjs
EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"

CMD ["node", "server.js"]
```

> Requires `output: "standalone"` in `next.config.ts`

## docker-compose.yml (development)

```yaml
services:
  db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_DB: staff_kit
      POSTGRES_USER: staffkit
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    ports:
      - "127.0.0.1:5432:5432"   # localhost only, not exposed to network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U staffkit -d staff_kit"]
      interval: 10s
      timeout: 5s
      retries: 5

  app:
    build:
      context: .
      target: deps       # dev: mount source, not full build
    command: npm run dev
    environment:
      DATABASE_URL: postgresql://staffkit:${DB_PASSWORD}@db:5432/staff_kit
      NEXTAUTH_SECRET: ${AUTH_SECRET}
      NEXTAUTH_URL: http://localhost:3000
    ports:
      - "3000:3000"
    depends_on:
      db:
        condition: service_healthy
    volumes:
      - .:/app
      - /app/node_modules
      - /app/.next

volumes:
  pgdata:
```

## docker-compose.prod.yml (production)

```yaml
services:
  db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_DB: staff_kit
      POSTGRES_USER: staffkit
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    # NO port mapping — only accessible by other containers

  pgbouncer:
    image: bitnami/pgbouncer:latest
    restart: unless-stopped
    environment:
      PGBOUNCER_DATABASE: staff_kit
      PGBOUNCER_PORT: 6432
      POSTGRESQL_HOST: db
      POSTGRESQL_USERNAME: staffkit
      POSTGRESQL_PASSWORD: ${DB_PASSWORD}
      PGBOUNCER_POOL_MODE: transaction
      PGBOUNCER_MAX_CLIENT_CONN: 1000
      PGBOUNCER_DEFAULT_POOL_SIZE: 20
    depends_on:
      db:
        condition: service_healthy

  app:
    build:
      context: .
      target: runner
    restart: unless-stopped
    environment:
      DATABASE_URL: postgresql://staffkit:${DB_PASSWORD}@pgbouncer:6432/staff_kit?pgbouncer=true
      NEXTAUTH_SECRET: ${AUTH_SECRET}
      NEXTAUTH_URL: ${APP_URL}
      NODE_ENV: production
    depends_on:
      - pgbouncer

  nginx:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/conf.d/default.conf:ro
      - ./nginx/certs:/etc/nginx/certs:ro
    depends_on:
      - app

volumes:
  pgdata:
```

## Nginx config (nginx/nginx.conf)

```nginx
upstream nextjs {
    server app:3000;
    keepalive 64;
}

server {
    listen 80;
    server_name _;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name _;

    ssl_certificate     /etc/nginx/certs/cert.pem;
    ssl_certificate_key /etc/nginx/certs/key.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    client_max_body_size 50M;    # Allow Excel file uploads

    location / {
        proxy_pass http://nextjs;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }
}
```

## .env.example

```bash
# Database
DB_PASSWORD=change_this_strong_password

# NextAuth
AUTH_SECRET=generate_with_openssl_rand_base64_32
APP_URL=https://staff.company.internal

# Optionally override
NEXTAUTH_URL=${APP_URL}
DATABASE_URL=postgresql://staffkit:${DB_PASSWORD}@pgbouncer:6432/staff_kit?pgbouncer=true
```

## Deploy procedure (VPS)

```bash
# 1. First deploy
git clone https://github.com/org/staff-kit-web.git /opt/staff-kit
cd /opt/staff-kit
cp .env.example .env
# Edit .env with real values
docker compose -f docker-compose.prod.yml up -d --build

# 2. Run migrations
docker compose -f docker-compose.prod.yml exec app npx prisma migrate deploy

# 3. Subsequent updates
git pull
docker compose -f docker-compose.prod.yml up -d --build --no-deps app

# 4. Rollback
git checkout <previous-tag>
docker compose -f docker-compose.prod.yml up -d --build --no-deps app
```

## Health check endpoint

```typescript
// src/app/api/health/route.ts
import { NextResponse } from "next/server"
import { prisma } from "@/lib/prisma"

export async function GET() {
  try {
    await prisma.$queryRaw`SELECT 1`
    return NextResponse.json({ status: "ok", db: "connected" })
  } catch {
    return NextResponse.json({ status: "error", db: "disconnected" }, { status: 503 })
  }
}
```

## DO NOT

- Do NOT expose PostgreSQL port to the network in production
- Do NOT commit `.env` — only `.env.example`
- Do NOT use `latest` tag for PostgreSQL image — pin to `postgres:16-alpine`
- Do NOT run migrations automatically on container start — run manually or in a migration job
