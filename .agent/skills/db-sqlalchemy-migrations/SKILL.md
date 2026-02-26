---
name: db-sqlalchemy-migrations
description: Use when creating/modifying SQLAlchemy models, Postgres schema, indexes, and migrations. Enforce naming, constraints, precision, timezone, and auditability.
version: 1.0.0
scope: workspace
tags: [database, postgres, sqlalchemy, migrations, alembic]
---

# DB + SQLAlchemy + Migrations Standard

## Goal
Keep schema changes safe, reversible, and correct for financial data.

## When to use
- Adding/changing tables/columns/indexes
- Changing numeric precision or timestamps
- Implementing audit logs, history, constraints

## Schema rules
1. **Timestamps**
- Store timestamps in UTC
- Use `created_at`, `updated_at` consistently
- If you bucket data (e.g. 30s), store bucket start time explicitly

2. **Financial numeric precision**
- Use DECIMAL/NUMERIC for prices and money
- Define scale/precision explicitly (no float)

3. **Constraints**
- Use NOT NULL where appropriate
- Add unique constraints for idempotency keys (crawler data)
- Add foreign keys for integrity (unless you have a clear reason not to)

4. **Indexes**
- Index query paths (e.g. `(symbol, ts)` for time-series)
- Avoid over-indexing write-heavy tables

5. **Migrations**
- Every schema change via migration
- Include downgrade path
- For large tables, prefer online-safe operations when possible

## Implementation steps
1. Update SQLAlchemy models with types + constraints.
2. Create migration (e.g., Alembic) with upgrade + downgrade.
3. Add/adjust indexes for main query patterns.
4. Add tests:
   - model constraints
   - basic CRUD and uniqueness behavior
5. Document any breaking changes.

## Output expectations
- Provide model snippet + migration snippet
- Provide index/constraint rationale
- Provide rollback plan

## Quick checklist
- [ ] UTC timestamps
- [ ] NUMERIC for price/money
- [ ] Unique constraints for time-series keys
- [ ] Migration has downgrade
- [ ] Index aligns with queries
