---
name: prisma-postgresql
description: >
  Use when working with the Prisma ORM and PostgreSQL database in Staff Kit web app.
  Covers schema design, migrations, query patterns, connection pooling, transactions,
  and full-text search with tsvector.
---

# Prisma + PostgreSQL — Staff Kit

## Schema conventions

- All model names: **PascalCase** singular (`Employee`, not `Employees`)
- All `@@map()` names: **snake_case** plural (`"employees"`)
- All field names: **camelCase** in schema → auto-maps to snake_case columns
- Always include `createdAt DateTime @default(now())` and `updatedAt DateTime @updatedAt`
- Always add `@@index` for columns used in WHERE or JOIN clauses

## Migration workflow

```bash
# After changing schema.prisma:
npx prisma migrate dev --name descriptive_name   # dev only
npx prisma migrate deploy                         # production (in Dockerfile/CI)
npx prisma generate                               # regenerate client after schema change
```

**Never edit migration files manually** after they are generated.

## Query patterns

### Pagination (always paginate lists)
```typescript
const [items, total] = await Promise.all([
  prisma.employee.findMany({
    where: filters,
    orderBy: { updatedAt: "desc" },
    skip: (page - 1) * pageSize,
    take: pageSize,
    include: { team: true },
  }),
  prisma.employee.count({ where: filters }),
])
return { items, total, page, pageSize }
```

### Full-text search (PostgreSQL tsvector)
```typescript
// FTS via raw query for tsvector column
const employees = await prisma.$queryRaw<Employee[]>`
  SELECT * FROM employees
  WHERE search_vector @@ plainto_tsquery('english', ${query})
  ORDER BY ts_rank(search_vector, plainto_tsquery('english', ${query})) DESC
  LIMIT ${pageSize}
`
```

### Transactions
```typescript
// For multi-step writes — always use transaction
const result = await prisma.$transaction(async (tx) => {
  const employee = await tx.employee.update({ ... })
  await tx.employeeDynamicValue.deleteMany({ where: { employeeId: employee.id } })
  await tx.employeeDynamicValue.createMany({ data: newValues })
  return employee
})
```

### Upsert pattern (import Excel)
```typescript
await prisma.employee.upsert({
  where: { employeeId: row.employeeId },
  create: { ...rowData },
  update: { ...rowData },
})
```

## Connection pooling

For production with ~1,000 users, **do not connect Prisma directly to PostgreSQL**.
Use PgBouncer in transaction mode:

```yaml
# docker-compose.prod.yml
pgbouncer:
  image: bitnami/pgbouncer:latest
  environment:
    PGBOUNCER_DATABASE: staff_kit
    PGBOUNCER_PORT: 6432
    POSTGRESQL_HOST: db
    POSTGRESQL_USERNAME: staffkit
    POSTGRESQL_PASSWORD: ${DB_PASSWORD}
    PGBOUNCER_POOL_MODE: transaction
    PGBOUNCER_MAX_CLIENT_CONN: 1000
    PGBOUNCER_DEFAULT_POOL_SIZE: 20
```

```
# .env.production
DATABASE_URL="postgresql://staffkit:${DB_PASSWORD}@pgbouncer:6432/staff_kit?pgbouncer=true"
```

Add `?pgbouncer=true` to disable Prisma's prepared statements (incompatible with PgBouncer transaction mode).

## Index strategy

```prisma
// Required indexes for Staff Kit
model Employee {
  @@index([staffGroup])              // filter by group tab
  @@index([teamId])                  // join teams
  @@index([updatedAt(sort: Desc)])   // sort by latest
  // searchVector GIN index — add via migration raw SQL
}
```

Add GIN index via raw migration:
```sql
-- In a migration file after creating employees table
CREATE INDEX employees_search_vector_idx ON employees USING GIN (search_vector);
```

## Update searchVector trigger

Add tsvector auto-update via PostgreSQL trigger (in migration):
```sql
CREATE OR REPLACE FUNCTION employees_search_vector_update() RETURNS trigger AS $$
BEGIN
  NEW.search_vector := to_tsvector('english',
    coalesce(NEW.full_name, '') || ' ' ||
    coalesce(NEW.employee_id, '') || ' ' ||
    coalesce(NEW.email, '') || ' ' ||
    coalesce(NEW.job_title, '')
  );
  RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE TRIGGER employees_search_vector_trigger
BEFORE INSERT OR UPDATE ON employees
FOR EACH ROW EXECUTE FUNCTION employees_search_vector_update();
```

## DO NOT

- Do NOT use `prisma.$queryRaw` for anything except FTS — use typed query methods
- Do NOT forget `?pgbouncer=true` in production DATABASE_URL
- Do NOT run `prisma migrate dev` on production — use `prisma migrate deploy`
- Do NOT create N+1 queries — use `include` or batch queries
