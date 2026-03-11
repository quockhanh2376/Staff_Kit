---
name: data-migration
description: >
  Use when migrating existing SQLite data (Staff Kit desktop) to PostgreSQL (Staff Kit web).
  Covers the migration script structure, data transformation rules, FK ordering,
  verification steps, and rollback procedure.
---

# Data Migration — SQLite → PostgreSQL (Staff Kit)

## Overview

One-time migration from the existing Tauri desktop app SQLite database to the new
PostgreSQL database for the web app. Run this **before going live**, on a dedicated
migration window.

## Migration script

```typescript
// scripts/migrate-sqlite-to-postgres.ts
import Database from "better-sqlite3"
import { PrismaClient } from "@prisma/client"
import { hash } from "argon2"
import path from "path"

const sqlite = new Database(process.env.SQLITE_DB_PATH!)
const prisma = new PrismaClient()

async function main() {
  console.log("🚀 Starting migration...")

  await migrateSettings()
  await migrateAccounts()
  await migrateTeams()
  await migrateDynamicFields()
  await migrateEmployees()
  await migrateDynamicValues()
  await rebuildSearchVectors()

  console.log("✅ Migration complete!")
}

// ── 1. Settings ───────────────────────────────────────────
async function migrateSettings() {
  const rows = sqlite.prepare("SELECT setting_key, setting_value FROM app_settings").all()
  await prisma.$transaction(
    rows.map((row: any) =>
      prisma.appSetting.upsert({
        where: { key: row.setting_key },
        create: { key: row.setting_key, value: row.setting_value ?? "" },
        update: { value: row.setting_value ?? "" },
      })
    )
  )
  console.log(`  ✓ Settings: ${rows.length}`)
}

// ── 2. Local Accounts ─────────────────────────────────────
async function migrateAccounts() {
  const rows = sqlite.prepare("SELECT * FROM app_local_accounts").all()
  for (const row of rows as any[]) {
    const role = mapRole(row.role)
    await prisma.localAccount.upsert({
      where: { username: row.username },
      create: {
        accountKey: row.account_key,
        displayName: row.display_name,
        username: row.username,
        passwordHash: row.password_hash,     // already argon2 — reuse as-is
        recoveryCodeHash: row.recovery_code_hash ?? null,
        role,
        forcePasswordReset: Boolean(row.force_password_reset),
      },
      update: { role, displayName: row.display_name },
    })
  }
  console.log(`  ✓ Accounts: ${rows.length}`)
}

// ── 3. Teams (parent-first ordering) ─────────────────────
async function migrateTeams() {
  const rows: any[] = sqlite.prepare(
    "SELECT * FROM teams ORDER BY parent_id NULLS FIRST"
  ).all()
  for (const row of rows) {
    await prisma.team.upsert({
      where: { name: row.name },
      create: {
        name: row.name,
        parentId: row.parent_id ? await resolveTeamId(row.parent_id) : null,
      },
      update: {},
    })
  }
  console.log(`  ✓ Teams: ${rows.length}`)
}

// ── 4. Dynamic Fields ─────────────────────────────────────
async function migrateDynamicFields() {
  const rows: any[] = sqlite.prepare("SELECT * FROM employee_dynamic_fields").all()
  for (const row of rows) {
    await prisma.employeeDynamicField.upsert({
      where: { key: row.key },
      create: { key: row.key, label: row.label },
      update: { label: row.label },
    })
  }
  console.log(`  ✓ Dynamic fields: ${rows.length}`)
}

// ── 5. Employees ──────────────────────────────────────────
async function migrateEmployees() {
  const rows: any[] = sqlite.prepare("SELECT * FROM employees").all()
  let count = 0
  const BATCH = 100
  for (let i = 0; i < rows.length; i += BATCH) {
    const batch = rows.slice(i, i + BATCH)
    await prisma.$transaction(
      batch.map((row) =>
        prisma.employee.upsert({
          where: { employeeId: row.employee_id },
          create: {
            employeeId: row.employee_id,
            fullName: row.full_name,
            nickName: row.nick_name ?? null,
            teamId: row.team_id ? resolveTeamNewId(row.team_id) : null,
            project: row.project ?? null,
            jobTitle: row.job_title ?? null,
            email: row.email ?? null,
            cellphone: row.cellphone ?? null,
            dateOfBirth: row.date_of_birth ? new Date(row.date_of_birth) : null,
            gender: row.gender ?? null,
            aswStartDate: row.asw_start_date ? new Date(row.asw_start_date) : null,
            clientStartDate: row.client_start_date ? new Date(row.client_start_date) : null,
            contractEndDate: row.contract_end_date ? new Date(row.contract_end_date) : null,
            computerName: row.computer_name ?? null,
            notes: row.notes ?? null,
            staffGroup: row.staff_group ?? "employee_list",
          },
          update: {},
        })
      )
    )
    count += batch.length
    console.log(`  ✓ Employees: ${count}/${rows.length}`)
  }
}

// ── 6. Dynamic Values ─────────────────────────────────────
async function migrateDynamicValues() {
  const rows: any[] = sqlite.prepare("SELECT * FROM employee_dynamic_values").all()
  // Batch insert to avoid timeout
  const BATCH = 500
  for (let i = 0; i < rows.length; i += BATCH) {
    const batch = rows.slice(i, i + BATCH)
    await prisma.employeeDynamicValue.createMany({
      data: batch.map((row) => ({
        employeeId: resolveEmployeeNewId(row.employee_id),
        fieldId: resolveFieldNewId(row.field_id),
        value: row.value,
      })),
      skipDuplicates: true,
    })
  }
  console.log(`  ✓ Dynamic values: ${rows.length}`)
}

// ── 7. Rebuild FTS search vectors ─────────────────────────
async function rebuildSearchVectors() {
  await prisma.$executeRaw`
    UPDATE employees SET
      search_vector = to_tsvector('english',
        coalesce(full_name, '') || ' ' ||
        coalesce(employee_id, '') || ' ' ||
        coalesce(email, '') || ' ' ||
        coalesce(job_title, '')
      )
  `
  console.log("  ✓ Search vectors rebuilt")
}

// ── Helpers ───────────────────────────────────────────────
function mapRole(role: string): "SUPER_ADMIN" | "ADMIN" | "USER" {
  if (role === "super_admin") return "SUPER_ADMIN"
  if (role === "admin") return "ADMIN"
  return "USER"
}

main().catch(console.error).finally(() => prisma.$disconnect())
```

## Run migration

```bash
# Install dependencies
npm install better-sqlite3 @types/better-sqlite3

# Set env vars
export SQLITE_DB_PATH="/path/to/staff_kit.sqlite3"
export DATABASE_URL="postgresql://staffkit:password@localhost:5432/staff_kit"

# Run
npx tsx scripts/migrate-sqlite-to-postgres.ts
```

## Verification steps (run after migration)

```bash
# Compare record counts
npx tsx scripts/verify-migration.ts
```

```typescript
// scripts/verify-migration.ts
const sqliteCounts = {
  accounts: sqlite.prepare("SELECT COUNT(*) as c FROM app_local_accounts").get(),
  employees: sqlite.prepare("SELECT COUNT(*) as c FROM employees").get(),
  teams: sqlite.prepare("SELECT COUNT(*) as c FROM teams").get(),
}
const pgCounts = {
  accounts: await prisma.localAccount.count(),
  employees: await prisma.employee.count(),
  teams: await prisma.team.count(),
}
// Assert all counts match
```

## Rollback

```bash
# If anything goes wrong, reset PostgreSQL and re-run
npx prisma migrate reset --force
npx tsx scripts/migrate-sqlite-to-postgres.ts
```

The desktop SQLite app stays **read-only / untouched** during migration.
Only switch users to the web app **after verification passes**.

## DO NOT

- Do NOT run migration against production PostgreSQL without a verified test run first
- Do NOT truncate the SQLite file during migration
- Do NOT skip the verification step
