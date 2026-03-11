---
name: testing-vitest-playwright
description: >
  Use when writing or running tests for Staff Kit web app.
  Covers Vitest unit tests for API routes and utilities, Playwright E2E tests for
  full user flows, test setup, mocking Prisma, and CI configuration.
---

# Testing — Vitest + Playwright (Staff Kit)

## Test philosophy

- **Unit tests (Vitest)**: Fast, isolated. Test API route logic, utility functions, auth helpers. Mock Prisma.
- **E2E tests (Playwright)**: Slow, real browser. Test complete user flows against a test DB.
- Coverage target: **>80% API layer** (unit), **100% critical user paths** (E2E)

---

## Vitest — Unit Tests

### Setup

```typescript
// vitest.config.ts
import { defineConfig } from "vitest/config"
import path from "path"

export default defineConfig({
  test: {
    environment: "node",
    globals: true,
    setupFiles: ["./tests/unit/setup.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/app/api/**", "src/lib/**"],
      exclude: ["src/lib/prisma.ts"],
      thresholds: { lines: 80, functions: 80, branches: 75 },
    },
  },
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
})
```

### Mock Prisma

```typescript
// tests/unit/setup.ts
import { vi } from "vitest"

vi.mock("@/lib/prisma", () => ({
  prisma: {
    employee: {
      findMany: vi.fn(),
      findUnique: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
      count: vi.fn(),
      upsert: vi.fn(),
    },
    localAccount: {
      findUnique: vi.fn(),
      findMany: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
    },
    team: { findMany: vi.fn(), create: vi.fn(), update: vi.fn(), delete: vi.fn() },
    $transaction: vi.fn((fn) => fn(prisma)),
    $queryRaw: vi.fn(),
  },
}))
```

### Mock NextAuth session

```typescript
// tests/unit/helpers.ts
import { vi } from "vitest"

export function mockAdminSession() {
  vi.mock("@/lib/auth", () => ({
    requireSession: vi.fn().mockResolvedValue({
      user: { id: 1, username: "testadmin", role: "ADMIN" },
    }),
    requireRole: vi.fn(),  // noop
  }))
}

export function mockSuperAdminSession() {
  vi.mock("@/lib/auth", () => ({
    requireSession: vi.fn().mockResolvedValue({
      user: { id: 1, username: "adman", role: "SUPER_ADMIN" },
    }),
    requireRole: vi.fn(),
  }))
}

export function mockUnauthenticated() {
  vi.mock("@/lib/auth", () => ({
    requireSession: vi.fn().mockRejectedValue(new Error("Unauthorized")),
  }))
}
```

### Example unit test (API route)

```typescript
// tests/unit/api/employees.test.ts
import { describe, it, expect, vi, beforeEach } from "vitest"
import { POST } from "@/app/api/employees/route"
import { prisma } from "@/lib/prisma"
import { mockAdminSession } from "../helpers"

mockAdminSession()

describe("POST /api/employees", () => {
  beforeEach(() => vi.clearAllMocks())

  it("creates employee with valid payload", async () => {
    const mockEmployee = { id: 1, employeeId: "EMP001", fullName: "John Doe" }
    vi.mocked(prisma.employee.create).mockResolvedValue(mockEmployee as any)

    const req = new Request("http://localhost/api/employees", {
      method: "POST",
      body: JSON.stringify({ employeeId: "EMP001", fullName: "John Doe" }),
    })
    const res = await POST(req as any)
    expect(res.status).toBe(201)
    const data = await res.json()
    expect(data.employeeId).toBe("EMP001")
  })

  it("returns 400 for missing required fields", async () => {
    const req = new Request("http://localhost/api/employees", {
      method: "POST",
      body: JSON.stringify({ employeeId: "" }),
    })
    const res = await POST(req as any)
    expect(res.status).toBe(400)
  })
})
```

### Run unit tests

```bash
npx vitest run                     # one-shot
npx vitest                         # watch mode
npx vitest run --coverage          # with coverage report
```

---

## Playwright — E2E Tests

### Setup

```typescript
// tests/e2e/playwright.config.ts
import { defineConfig, devices } from "@playwright/test"

export default defineConfig({
  testDir: "./tests/e2e",
  testMatch: "**/*.spec.ts",
  fullyParallel: false,          // Sequential for DB consistency
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["html", { open: "never" }]],
  use: {
    baseURL: process.env.E2E_BASE_URL ?? "http://localhost:3000",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
    trace: "on-first-retry",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
  ],
  globalSetup: "./tests/e2e/global-setup.ts",
  globalTeardown: "./tests/e2e/global-teardown.ts",
})
```

### Global setup (seed test DB)

```typescript
// tests/e2e/global-setup.ts
import { execSync } from "child_process"

export default async function globalSetup() {
  // Reset and seed test database
  process.env.DATABASE_URL = process.env.TEST_DATABASE_URL!
  execSync("npx prisma migrate reset --force --skip-seed", { stdio: "inherit" })
  execSync("npx prisma db seed", { stdio: "inherit" })
}
```

### Page Object Model (required for all E2E tests)

```typescript
// tests/e2e/pages/LoginPage.ts
import { Page, Locator } from "@playwright/test"

export class LoginPage {
  readonly page: Page
  readonly usernameInput: Locator
  readonly passwordInput: Locator
  readonly submitButton: Locator
  readonly errorMessage: Locator

  constructor(page: Page) {
    this.page = page
    this.usernameInput = page.getByLabel("Username")
    this.passwordInput = page.getByLabel("Password")
    this.submitButton = page.getByRole("button", { name: "Login" })
    this.errorMessage = page.getByRole("alert")
  }

  async goto() { await this.page.goto("/login") }

  async login(username: string, password: string) {
    await this.usernameInput.fill(username)
    await this.passwordInput.fill(password)
    await this.submitButton.click()
  }
}
```

### Example E2E spec

```typescript
// tests/e2e/auth.spec.ts
import { test, expect } from "@playwright/test"
import { LoginPage } from "./pages/LoginPage"

test.describe("Authentication", () => {
  test("admin can login and see dashboard", async ({ page }) => {
    const loginPage = new LoginPage(page)
    await loginPage.goto()
    await loginPage.login("adman", "Welcome!")
    await expect(page).toHaveURL("/dashboard")
    await expect(page.getByText("Settings")).toBeVisible()
  })

  test("wrong password shows error", async ({ page }) => {
    const loginPage = new LoginPage(page)
    await loginPage.goto()
    await loginPage.login("adman", "wrongpass")
    await expect(loginPage.errorMessage).toContainText("Invalid")
  })

  test("user without admin role cannot access settings", async ({ page }) => {
    const loginPage = new LoginPage(page)
    await loginPage.goto()
    await loginPage.login("admduy", "Welcome!")
    await page.goto("/settings")
    await expect(page).toHaveURL("/dashboard")  // redirect
  })
})
```

### Run E2E tests

```bash
# Requires app running first
npx playwright test                    # headless
npx playwright test --ui               # visual UI mode
npx playwright test auth.spec.ts       # single file
npx playwright show-report             # view last report
```

---

## CI configuration (.github/workflows/ci.yml)

```yaml
name: CI
on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: "20", cache: "npm" }
      - run: npm ci
      - run: npx tsc --noEmit
      - run: npm run lint
      - run: npx vitest run --coverage
      - uses: actions/upload-artifact@v4
        with: { name: coverage, path: coverage/ }

  e2e:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_DB: staff_kit_test
          POSTGRES_USER: staffkit
          POSTGRES_PASSWORD: testpassword
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: "20", cache: "npm" }
      - run: npm ci
      - run: npx playwright install --with-deps chromium
      - run: npx prisma migrate deploy
        env:
          DATABASE_URL: postgresql://staffkit:testpassword@localhost:5432/staff_kit_test
      - run: npx playwright test
        env:
          DATABASE_URL: postgresql://staffkit:testpassword@localhost:5432/staff_kit_test
          TEST_DATABASE_URL: postgresql://staffkit:testpassword@localhost:5432/staff_kit_test
          NEXTAUTH_SECRET: test_secret
          NEXTAUTH_URL: http://localhost:3000
      - uses: actions/upload-artifact@v4
        if: failure()
        with: { name: playwright-report, path: playwright-report/ }
```

## Test file naming convention

```
tests/
  unit/
    api/employees.test.ts
    api/accounts.test.ts
    api/teams.test.ts
    lib/validators.test.ts
    lib/auth-helpers.test.ts
  e2e/
    auth.spec.ts
    employees.spec.ts
    import.spec.ts
    settings.spec.ts
    backup.spec.ts
    pages/              # Page Object Models
```

## DO NOT

- Do NOT test Prisma internals — mock at the service boundary
- Do NOT share state between E2E tests — each test must be independent
- Do NOT hardcode test credentials inline — use constants from `tests/e2e/fixtures.ts`
- Do NOT skip writing tests for API routes with authorization logic
