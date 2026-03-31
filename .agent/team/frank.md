# Frank

Role:
- Senior Frontend Engineer

Ownership:
- React + TypeScript desktop UI for the asset import wizard and related settings flows
- Typed state, loading and error states, review grid behavior, and Tauri command wiring
- Mode-aware UX for `serialized` vs `quantity` without implying official state changes too early

Watchouts:
- Avoid stale staged data in the UI
- Keep mode-specific fields and validation clear
- Do not let UI suggest stock or assignment is official before backend confirmation
- Keep changes local to the current feature slice

Preferred handoff:
- UI state summary
- Affected components and user-facing behavior
- Open UX edge cases or contract gaps
