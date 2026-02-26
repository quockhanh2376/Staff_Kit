---
name: nextjs-ui-performance
description: Use when optimizing Next.js App Router pages/components: data fetching strategy, caching/revalidation, avoiding waterfalls, rendering boundaries, and privacy-mode handling.
version: 1.0.0
scope: workspace
tags: [nextjs, performance, caching, app-router, ui]
---

# Next.js UI Performance + Data Fetching Standard

## Goal
Fast, stable UI with correct caching and predictable data refresh.

## When to use
- Building pages with server components / route handlers
- Adding data fetching
- Implementing caching, revalidate, tags
- Implementing “privacy mode” that hides sensitive values

## Rules
1. **Avoid waterfalls**
- Parallelize independent fetches
- Do not chain fetches unnecessarily in render

2. **Caching strategy**
- Decide per endpoint:
  - `no-store` for truly dynamic
  - cached with `revalidate` for semi-static
- If using tags, ensure invalidation is wired correctly

3. **Render boundaries**
- Keep heavy components isolated
- Use loading states appropriately
- Avoid excessive client components; keep logic server-side when possible

4. **Error handling**
- Standardize error UI
- Never leak sensitive errors to UI
- Map API error codes to user-friendly messages

5. **Privacy mode**
- All sensitive values must be masked consistently:
  - total equity, PnL, holdings value
- Ensure privacy toggle does not cause extra API calls if not needed

## Implementation steps
1. Identify data needed and whether it can be cached.
2. Implement fetchers:
   - server-side where possible
   - consistent parsing of API envelope (`ok/data/error`)
3. Add caching directives and/or revalidation.
4. Add loading + error boundaries.
5. Verify with a simple performance check:
   - page load, data refresh behavior, no redundant requests

## Output expectations
- Explain caching choice and expected freshness
- Provide component outline (server/client split)
- Provide how privacy mode is applied (where + how)

## Quick checklist
- [ ] No waterfall fetch
- [ ] Caching choice documented
- [ ] Error mapping consistent
- [ ] Privacy masking consistent everywhere
- [ ] No redundant network calls
