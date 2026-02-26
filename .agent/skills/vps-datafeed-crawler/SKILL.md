---
name: vps-datafeed-crawler
description: Use when building or modifying the VPS price crawler/collector (e.g., runs every ~30s). Enforce reliability: retries, rate-limit, idempotency, logging, and safe writes.
version: 1.0.0
scope: workspace
tags: [crawler, ingestion, vps, reliability]
---

# VPS Datafeed Crawler (Reliability Standard)

## Goal
Collect price data safely and consistently without duplicates, overload, or silent failures.

## When to use
- Adding/updating crawler schedule
- Changing data source endpoints
- Writing to DB / storage
- Adding throttling, retry, backoff

## Core reliability rules
1. **Idempotency**
- Define a unique key per record (e.g. `symbol + timestamp_bucket`)
- Use upsert / on-conflict to avoid duplicates

2. **Rate limiting**
- Respect source limits
- Implement a per-host limiter (token bucket or simple sleep)
- Avoid burst across many symbols

3. **Retries + backoff**
- Retry transient errors (timeouts, 429, 5xx)
- Exponential backoff with jitter
- Cap max retries; log final failure

4. **Observability**
- Structured logs: request_id, source, symbol, duration_ms, status
- Metrics (if available): success/fail counts, lag, duplicates prevented

5. **Safe DB writes**
- Batch writes where possible
- Transaction boundaries clear
- Decimal-safe numeric types

## Implementation steps
1. Specify schedule:
   - interval (e.g. 30s) + timezone + start/stop behavior
2. Implement fetch layer:
   - timeouts, headers, retries, backoff
3. Implement normalize layer:
   - consistent schema (timestamp UTC, fields)
4. Implement write layer:
   - upsert + unique constraints
5. Add integration test:
   - simulate 429/timeout and ensure retry/backoff
6. Add “dry-run mode” (optional) for local debug.

## Output expectations
- Provide the final schema of stored records
- Provide unique constraint / upsert strategy
- Provide failure modes and how they’re handled

## Quick checklist
- [ ] Unique constraint + upsert in DB
- [ ] Timeout + retry/backoff for transient failures
- [ ] Rate-limit protection
- [ ] Logs include symbol + duration + status
- [ ] No duplicate records on rerun
