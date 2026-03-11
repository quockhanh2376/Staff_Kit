# Staff Kit — Chuyển đổi Desktop → Web App

> **Trạng thái:** Đang lên kế hoạch  
> **Cập nhật lần cuối:** 2026-03-11  
> **Mục tiêu scale:** ~1,000 nhân viên đăng nhập đồng thời

---

## 1. Bối cảnh & Quyết định

Staff Kit hiện tại là **Tauri v2 desktop app** (Rust backend + React frontend + SQLite).  
Mục tiêu: chuyển sang **web app** để deploy tập trung, nhiều người dùng cùng lúc, không cần cài đặt.

### Stack đã chọn

| Layer | Công nghệ | Lý do |
|-------|-----------|-------|
| **Frontend + API** | Next.js 16 (App Router, stable) | Fullstack 1 codebase TypeScript, SSR, API Routes |
| **Database** | PostgreSQL 16 | Multi-user, robust, full-text search, scale tốt |
| **ORM** | Prisma | Type-safe, auto-migration, generate types |
| **Auth** | NextAuth.js v5 | Session/JWT, role-based (Super Admin / Admin / User) |
| **Cache + Realtime** | Redis 7 | Pub/Sub real-time notifications, rate limiting, cache |
| **Deploy** | Docker + Docker Compose | Reproducible, dễ maintain, VPS-ready |
| **Testing** | Vitest (unit) + Playwright (E2E) | Coverage đầy đủ, chạy trong CI |
| **Runtime** | Node.js 20.9+ | Match baseline requirement của Next.js 16 cho dev/CI/prod |

### Tại sao không cần Rust backend?

Staff Kit là **CRUD app nội bộ**. Next.js xử lý dư sức ở quy mô ~1,000 users:
- Import Excel → Node.js `xlsx` package đủ nhanh
- FTS search → PostgreSQL `tsvector` native + GIN index
- Argon2 hashing → `argon2` npm package dùng native binding
- Real-time notifications → Redis Pub/Sub + SSE (Server-Sent Events) — không cần separate WebSocket server

---

## 2. Thông tin triển khai

| Mục | Quyết định |
|-----|-----------|
| **Hosting** | VPS tự quản (Linux) |
| **Domain** | Subdomain / IP nội bộ trong mạng WiFi công ty (không public internet) |
| **SSL** | Self-signed cert hoặc Let's Encrypt nếu có domain |
| **Existing data** | **Migrate toàn bộ** từ SQLite sang PostgreSQL (đầy đủ, không bỏ sót) |
| **Phạm vi** | Convert từng phần theo Phase: Auth + Employee CRUD trước, Import Excel + Backup sau |
| **Real-time** | Redis Pub/Sub + SSE — notifications khi data thay đổi |
| **Testing** | Unit Test (Vitest) + E2E full flows (Playwright) — bắt buộc cho mỗi feature |
| **Language + Theme** | Default app language = English. UI/UX must support bilingual EN/VI and both light/dark mode from the first web baseline |
| **Observability + Engineering Quality** | Mọi feature/function phải có structured logging đủ chi tiết để debug, audit, maintain và scale lâu dài; codebase phải ưu tiên readability, testability, modularity và vận hành ổn định ngay từ đầu |
| **Next.js Baseline** | Use Next.js 16 stable with `typedRoutes: true`, `proxy.ts`, `instrumentation.ts`, and `instrumentation-client.ts` | Follow stable framework conventions; do not build on canary-only behavior |

---

## 3. Phân tích chuyển đổi

### ✅ Giữ lại (~70% frontend code)

- React components (`SettingsView`, login form, employee table, v.v.)
- Tailwind CSS styling
- Type definitions (`staff.ts`, `app.ts`) — bỏ Tauri-specific types
- Business logic trong hooks (`useAuthState`, `useSettingsState`, `useImportState`)
- Constants, utilities (`lib/utils.ts`, `lib/constants.ts`)

### 🔄 Cần thay đổi

| Hiện tại (Tauri) | Web app (Next.js) |
|------------------|-------------------|
| `invoke("command")` trong `staff-api.ts` (~40 commands) | `fetch("/api/...")` REST endpoints |
| SQLite + rusqlite (raw SQL) | PostgreSQL + Prisma ORM |
| `@tauri-apps/plugin-dialog` (file picker) | `<input type="file">` + server upload |
| Local filesystem backup/restore | Server-side `pg_dump` / restore |
| Shared DB Location (OneDrive sync) | Không cần — DB đã centralized |
| `#[tauri::command]` Rust functions | Next.js API Route handlers |
| SQLCipher encryption at-rest | PostgreSQL với TLS + encrypted volume |

### ❌ Bỏ hoàn toàn

- `src-tauri/` (Rust backend, Tauri config, Cargo.toml)
- Desktop installer (MSI/EXE), WebView2 runtime
- `move_database_to` / shared DB path feature

---

## 4. Kiến trúc & Cấu trúc thư mục

```
staff-kit-web/
├── docker-compose.yml           # PostgreSQL + App + Nginx
├── docker-compose.prod.yml      # Production overrides
├── Dockerfile                   # Multi-stage build
├── nginx.conf                   # Reverse proxy config
├── next.config.ts               # Next.js config (`typedRoutes: true`, stable options)
├── .env.example                 # Template biến môi trường
│
├── prisma/
│   ├── schema.prisma            # DB schema
│   ├── migrations/              # Auto-generated migrations
│   └── seed.ts                  # Default adman super_admin seed
│
├── src/
│   ├── app/
│   │   ├── layout.tsx
│   │   ├── page.tsx             # Redirect to /dashboard hoặc /login
│   │   ├── login/page.tsx
│   │   ├── dashboard/           # Protected pages
│   │   └── api/
│   │       ├── auth/[...nextauth]/  # NextAuth handler
│   │       ├── employees/           # CRUD + search + import
│   │       ├── teams/               # Team management
│   │       ├── accounts/            # User account management
│   │       ├── columns/             # Dynamic column config
│   │       └── backup/              # pg_dump download + restore
│   │
│   ├── components/              # Shared UI components
│   │   ├── ui/                  # Base components (button, input, table...)
│   │   └── features/            # Feature-specific (EmployeeTable, SettingsPanel...)
│   ├── hooks/                   # React hooks (migrate từ Tauri hooks)
│   ├── instrumentation.ts       # Server instrumentation / OpenTelemetry bootstrap
│   ├── instrumentation-client.ts # Client instrumentation hooks
│   ├── proxy.ts                 # Minimal edge proxy checks; not primary auth layer
│   ├── services/
│   │   └── api-client.ts        # fetch() wrapper thay staff-api.ts
│   ├── lib/
│   │   ├── auth.ts              # NextAuth config + role helpers
│   │   ├── prisma.ts            # Prisma singleton
│   │   ├── utils.ts
│   │   └── constants.ts
│   └── types/                   # TypeScript types (bỏ Tauri types)
│
├── tests/
│   ├── unit/                    # Vitest unit tests
│   │   ├── api/                 # API route handler tests
│   │   └── lib/                 # Utility function tests
│   └── e2e/                     # Playwright E2E tests
│       ├── auth.spec.ts         # Login, logout, role access
│       ├── employees.spec.ts    # CRUD, search, import
│       ├── settings.spec.ts     # Account management, backup
│       └── playwright.config.ts
│
└── .github/
    └── workflows/
        └── ci.yml               # Test + type-check on push
```

---

## 5. Docker Setup

```yaml
# docker-compose.yml (development)
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
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U staffkit"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    ports:
      - "127.0.0.1:6379:6379"   # localhost only
    volumes:
      - redisdata:/data
    command: redis-server --save 60 1 --loglevel warning

  app:
    build:
      context: .
      target: development
    environment:
      DATABASE_URL: postgresql://staffkit:${DB_PASSWORD}@db:5432/staff_kit
      REDIS_URL: redis://redis:6379
      NEXTAUTH_SECRET: ${AUTH_SECRET}
      NEXTAUTH_URL: ${APP_URL:-http://localhost:3000}
    ports:
      - "3000:3000"
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_started
    volumes:
      - .:/app
      - /app/node_modules
      - /app/.next

volumes:
  pgdata:
  redisdata:
```

```yaml
# docker-compose.prod.yml (production override)
services:
  app:
    build:
      target: production
    restart: unless-stopped
    environment:
      NODE_ENV: production
      REDIS_URL: redis://redis:6379

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - redisdata:/data
    command: redis-server --save 60 1 --requirepass ${REDIS_PASSWORD}
    # NO port mapping — only accessible by other containers

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/conf.d/default.conf
      - ./certs:/etc/nginx/certs
    depends_on:
      - app
```

---

## 6. Database Schema (Prisma)

```prisma
// prisma/schema.prisma
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

enum Role {
  SUPER_ADMIN
  ADMIN
  USER
}

model LocalAccount {
  id                 Int      @id @default(autoincrement())
  accountKey         String   @unique @default(cuid())
  displayName        String
  username           String   @unique
  passwordHash       String
  recoveryCodeHash   String?
  role               Role     @default(USER)
  forcePasswordReset Boolean  @default(true)
  isActive           Boolean  @default(false)
  createdAt          DateTime @default(now())
  updatedAt          DateTime @updatedAt

  @@map("local_accounts")
  @@index([username])
}

model Employee {
  id                   Int      @id @default(autoincrement())
  employeeId           String   @unique
  fullName             String
  nickName             String?
  teamId               Int?
  team                 Team?    @relation(fields: [teamId], references: [id], onDelete: SetNull)
  project              String?
  jobTitle             String?
  email                String?
  cellphone            String?
  dateOfBirth          DateTime?
  gender               String?
  aswStartDate         DateTime?
  clientStartDate      DateTime?
  contractEndDate      DateTime?
  clientYearOfServices String?
  startDate            DateTime?
  computerName         String?
  notes                String?
  staffGroup           String   @default("employee_list")
  dynamicValues        EmployeeDynamicValue[]
  searchVector         Unsupported("tsvector")?   // GIN index cho FTS
  createdAt            DateTime @default(now())
  updatedAt            DateTime @updatedAt

  @@map("employees")
  @@index([staffGroup])
  @@index([teamId])
}

model Team {
  id        Int        @id @default(autoincrement())
  name      String     @unique
  parentId  Int?
  parent    Team?      @relation("TeamHierarchy", fields: [parentId], references: [id])
  children  Team[]     @relation("TeamHierarchy")
  employees Employee[]

  @@map("teams")
}

model EmployeeDynamicField {
  id     Int    @id @default(autoincrement())
  key    String @unique
  label  String
  values EmployeeDynamicValue[]

  @@map("employee_dynamic_fields")
}

model EmployeeDynamicValue {
  id         Int                  @id @default(autoincrement())
  employeeId Int
  fieldId    Int
  value      String
  employee   Employee             @relation(fields: [employeeId], references: [id], onDelete: Cascade)
  field      EmployeeDynamicField @relation(fields: [fieldId], references: [id], onDelete: Cascade)

  @@unique([employeeId, fieldId])
  @@map("employee_dynamic_values")
}

model AppSetting {
  key   String @id
  value String

  @@map("app_settings")
}
```

---

## 7. Scale Architecture (~1,000 users)

Với 1,000 nhân viên login đồng thời, cần chú ý:

### Connection Pooling
```
Next.js → PgBouncer → PostgreSQL
```
- Prisma mặc định mở quá nhiều connection → dùng **PgBouncer** hoặc **Prisma Accelerate**
- `DATABASE_URL` trỏ vào PgBouncer port (6432), không trỏ thẳng Postgres

### Caching
- **Server-side**: Redis cho session store + cache employee list
- **Client-side**: TanStack Query (React Query) với `staleTime` phù hợp

### Index tối ưu
- `employees.staffGroup` — filter theo nhóm
- `employees.searchVector` (GIN) — full-text search
- `employees.teamId` — join teams

### Stateless app server
- Next.js App Router sessions → JWT (stateless), không dùng server-side session store mặc định
- Có thể scale horizontal (multiple app containers) mà không cần sticky session

---

## 8. Testing Strategy

### Unit Tests (Vitest)
- API route handlers (mock Prisma)
- Auth logic (role checks, permission guards)
- Utility functions (Excel parser, validators)
- Coverage target: **>80%** cho API layer

### E2E Tests (Playwright)
- `auth.spec.ts`: Login đúng/sai, logout, force password reset, role redirect
- `employees.spec.ts`: List, filter, search, create, edit, delete, move group
- `import.spec.ts`: Upload Excel file, preview, confirm import
- `settings.spec.ts`: Account management, role edit, backup download
- `admin.spec.ts`: Super admin reset data, restore backup

### CI Pipeline (GitHub Actions)
```yaml
- `eslint . --max-warnings=0`
- `tsc --noEmit`
- `vitest --run`
- `playwright test` (headless, Docker Compose)
- `next build`
- Build Docker image
```

**Lưu ý Next.js 16:** không dựa vào `next lint`, và không giả định `next build` sẽ tự chạy lint. Lint phải là step riêng trong CI.

---

## 9. Data Migration (SQLite → PostgreSQL)

### Công cụ
Viết script Node.js một lần:

```
scripts/migrate-sqlite-to-postgres.ts
  ├── Read SQLite file (better-sqlite3)
  ├── Transform data (dates, nulls, roles)
  └── Insert via Prisma (batch upsert)
```

### Thứ tự migrate (tránh FK violation)
1. `app_settings` → `AppSetting`
2. `app_local_accounts` → `LocalAccount`
3. `teams` (parent trước, child sau) → `Team`
4. `employee_dynamic_fields` → `EmployeeDynamicField`
5. `employees` → `Employee`
6. `employee_dynamic_values` → `EmployeeDynamicValue`
7. Rebuild FTS `searchVector` sau khi insert

### Kiểm tra sau migrate
- Count records khớp giữa SQLite và PostgreSQL
- Spot-check 10 employees random
- Test login với account `adman`

---

## 10. Kế hoạch thực hiện (Phase Checklist)

### Phase 1 — Khung sườn (~3 ngày)
- [ ] Init Next.js 16 stable project (App Router, TypeScript, Tailwind)
- [ ] Chốt baseline Node.js 20.9+ cho dev, CI, Docker image và production
- [ ] Docker Compose: PostgreSQL + App
- [ ] Prisma schema + `prisma migrate dev`
- [ ] NextAuth.js v5: Credentials provider, JWT, role từ DB
- [ ] `next.config.ts` với `typedRoutes: true` và config baseline tương thích Next.js 16 stable
- [ ] App shell baseline: default locale EN, bilingual EN/VI UI foundation, and light/dark theme system
- [ ] Define app-wide logging contract: log levels, request ID, feature/function context, audit events, redaction rules
- [ ] Add `instrumentation.ts` và `instrumentation-client.ts` cho baseline observability hooks
- [ ] Add `proxy.ts` chỉ cho minimal edge gating khi cần; auth/role enforcement chính vẫn ở server-side route handlers/actions
- [ ] Explicit ESLint CLI + typecheck commands trong package/CI; không rely vào `next lint` hoặc `next build` để lint
- [ ] Seed script: `adman` SUPER_ADMIN account
- [ ] **Unit test**: Auth helpers, role guards

### Phase 2 — Auth & Account Management (~3 ngày)
- [ ] API: login, logout, change password, forgot password, force reset
- [ ] API: list accounts, create, update (role), delete, reset password
- [ ] Frontend: Login page, Settings > Admin Portal
- [ ] **Unit test**: mọi auth API route
- [ ] **E2E**: `auth.spec.ts`, `settings.spec.ts` (accounts)

### Phase 3 — Employee Core (~4 ngày)
- [ ] API: list employees (pagination, filter, sort), search FTS
- [ ] API: create, update, delete employee
- [ ] API: list teams, CRUD teams (hierarchy)
- [ ] API: move employees between groups
- [ ] API: dynamic columns CRUD
- [ ] Frontend: Employee table, filters, inline edit
- [ ] **Unit test**: employee API routes
- [ ] **E2E**: `employees.spec.ts`

### Phase 4 — Import Excel (~2 ngày)
- [ ] API: upload Excel files, parse, preview diff
- [ ] API: confirm import (upsert employees)
- [ ] Frontend: Import flow (upload → preview → confirm)
- [ ] **E2E**: `import.spec.ts`

### Phase 5 — Backup & Restore (~2 ngày)
- [ ] API: `pg_dump` → download `.sql` file
- [ ] API: upload `.sql` → restore (super admin only)
- [ ] Frontend: Backup panel, download/upload buttons
- [ ] **E2E**: `settings.spec.ts` (backup)

### Phase 6 — Data Migration Tool (~1 ngày)
- [ ] Script `migrate-sqlite-to-postgres.ts`
- [ ] Verify script trên DB thật
- [ ] Document hướng dẫn chạy migration

### Phase 7 — Production Ready (~2 ngày)
- [ ] Dockerfile multi-stage (dev / production)
- [ ] `docker-compose.prod.yml` + Nginx reverse proxy
- [ ] PgBouncer connection pooling
- [ ] Environment variables, secrets checklist
- [ ] Health check endpoint `/api/health`
- [ ] Centralized structured JSON logging for all features/functions
- [ ] Request tracing: request ID, user context, feature/module, action/function, latency, result status
- [ ] Audit logs for auth, employee CRUD, import, move, backup/restore, settings changes
- [ ] README: hướng dẫn deploy lên VPS
- [ ] Full Playwright E2E pass trên production build

**Tổng ước tính: ~3-4 tuần**

---

## 11. Lợi ích sau chuyển đổi

| Desktop (hiện tại) | Web app (mới) |
|--------------------|---------------|
| Cài MSI/EXE từng máy | Mở browser là dùng |
| Update = build + distribute installer | Deploy 1 lần, tất cả tự thấy bản mới |
| SQLite single-user | PostgreSQL, 1,000+ concurrent users |
| Chỉ Windows | Mọi OS, mọi device |
| Backup file local | Server-side `pg_dump`, automated |
| Code Rust + TypeScript | Chỉ TypeScript |

---

## 12. Tiêu chuẩn chất lượng code

### Mỗi API route phải có:
- Input validation (Zod schema)
- Role-based access guard (server-side; không rely vào `proxy.ts` như lớp auth/authorization chính)
- Proper HTTP status codes
- Structured error response `{ error: string, code?: string }`
- Request/trace context for logs and debugging
- Unit test coverage

### Frontend:
- React Query cho data fetching (cache, loading, error states)
- No `any` TypeScript types
- `typedRoutes: true` trong `next.config.ts`
- Component tách biệt: UI thuần vs có side effects
- All user-facing text must be localization-ready for EN/VI, with English as the default locale
- All screens/components must work consistently in both light mode and dark mode
- Feature modules phải rõ boundary, dễ đọc, dễ test, dễ thay đổi mà không gây side effect chéo

### Database:
- Mọi query phức tạp phải có index phù hợp
- Transactions cho multi-step writes
- Không raw SQL trừ FTS queries

### Observability:
- Mọi feature/function quan trọng phải ghi structured log với tối thiểu: timestamp, level, requestId, userId/role, feature, action/function, entity target, result, latency
- Không được swallow error im lặng; lỗi phải có log đủ ngữ cảnh để debug nhưng không làm lộ secret hoặc sensitive data
- Các thao tác quan trọng phải có audit log riêng: login/logout, password reset, create/update/delete employee, move group, import, backup/restore, settings change
- Cần có health/readiness logging và signal đủ tốt để production support xác định lỗi nhanh
- Dùng `instrumentation.ts` và `instrumentation-client.ts` làm baseline hook cho observability thay vì log ad-hoc rải rác

### Maintainability & Scalability:
- Ưu tiên code dễ đọc, rõ trách nhiệm, module hóa tốt hơn là tối ưu sớm thiếu kiểm soát
- Mọi feature mới phải đi kèm type-safe contract, test phù hợp, logging phù hợp và documentation ngắn gọn cho team maintain
- CI phải chặn merge nếu fail typecheck, lint, unit test hoặc E2E critical flows
- Thiết kế phải đủ sạch để team có thể maintain, edit và scale up sau này mà không cần rewrite lớn theo từng phase ngắn hạn

### Security:
- CSRF protection (NextAuth built-in)
- Rate limiting login endpoint
- SQL injection: không thể (Prisma parameterized)
- XSS: React escapes by default
- Secrets chỉ trong `.env`, không commit
