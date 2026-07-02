---
description: "Use when editing React, TypeScript, hooks, views, drawers, tables, styling, or frontend state under src/. Covers Staff Kit desktop frontend boundaries and validation habits."
applyTo: "src/**"
---

# Staff Kit Frontend Guidelines

- Preserve the existing feature-oriented structure under `src/features/` and shared helpers under `src/lib/`, `src/services/`, and `src/types/`.
- Prefer extending existing hooks and view modules over moving state upward into `App.tsx` unless the state is truly app-global.
- Keep UI copy and state transitions consistent with nearby screens; do not rewrite established terminology without a task-specific reason.
- Reuse existing utility functions and typed models before introducing new ad-hoc shapes.
- Avoid broad visual churn. Match the current desktop-first Tauri UI unless the task explicitly requests a redesign.
- When changing frontend behavior, validate with the narrowest relevant check first, then widen only if needed.
