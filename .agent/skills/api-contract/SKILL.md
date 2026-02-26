---
name: api-contract
description: Use when designing/changing API endpoints between Next.js frontend and Python FastAPI backend. Enforce consistent response envelope, error shape, pagination, datetime, and versioning.
version: 1.0.0
scope: workspace
tags: [api, fastapi, nextjs, contract, types]
---

# API Contract (Next.js <-> FastAPI)

## Goal
Keep all HTTP APIs consistent, predictable, and easy to consume from Next.js.

## When to use
- Adding/changing endpoints
- Touching request/response models
- Handling errors, pagination, filters/sorts
- Adding auth-protected routes

## Output expectations
- Define: route, method, auth, request schema, response schema, error schema
- Provide example payloads
- Update server + client types (if any) consistently

## Contract rules
### 1) Response envelope (standard)
Return **one of**:
- Success:
  - `{"ok": true, "data": <T>, "meta": {...optional}}`
- Error:
  - `{"ok": false, "error": {"code": "STRING", "message": "HUMAN", "details": {...optional}}, "meta": {...optional}}`

### 2) Error handling
- Use stable `error.code` (machine readable)
- Do not leak stack traces
- Map errors to HTTP status:
  - 400 validation, 401 unauth, 403 forbidden, 404 not found, 409 conflict, 429 rate-limit, 500 unexpected

### 3) Pagination (list endpoints)
- Request: `page`, `page_size` OR cursor style (choose one per resource)
- Response meta (page style):
  - `meta: { "page": 1, "page_size": 50, "total": 1234 }`

### 4) Datetime + numbers
- Datetime: ISO-8601 in UTC (e.g. `2026-01-17T10:00:00Z`)
- Money/price: use decimal-safe types in DB; serialize as number or string consistently (pick one and document)
- Always document timezone assumptions

### 5) Versioning + compatibility
- Prefer `/api/v1/...`
- Do not break existing clients without a migration plan

## Implementation steps
1. Specify route + auth requirements.
2. Define Pydantic models: request + response + error.
3. Implement endpoint in FastAPI with validation + proper status codes.
4. Add tests:
   - contract test for response shape
   - validation test for bad inputs
5. Update frontend client usage:
   - parse envelope
   - handle `ok=false` codes
6. Add examples to references if needed.

## Quick checklist
- [ ] Envelope used (ok/data/meta or ok/error/meta)
- [ ] Error codes stable + documented
- [ ] Pagination meta correct (if list)
- [ ] Datetime is UTC ISO-8601
- [ ] Tests cover success + failure
