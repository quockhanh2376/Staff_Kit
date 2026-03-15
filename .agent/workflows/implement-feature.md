---
description: Full-cycle feature implementation for the Staff Kit desktop app — research, implement, test, verify
---

# /implement-feature workflow

Use this workflow to implement any new feature for the native Staff Kit desktop app.

## Steps

1. **Read context**
   - Read `Note.md` for business rules and current implementation state
   - Read `README.md` and `QUALITY.md` for runtime and quality expectations
   - Read the existing code in the feature area before editing

2. **Define touch points**
   - Frontend files under `src/`
   - Tauri/Rust files under `src-tauri/src/`
   - Shared types/constants used by both layers

3. **Implement backend changes first when needed**
   - Update Rust database or command code in `src-tauri/src/`
   - Preserve parameterized SQL and existing command naming patterns
   - Keep IPC contracts stable unless the task explicitly changes them

4. **Implement frontend changes**
   - Update `src/services/staff-api.ts` only when the Tauri command contract changes
   - Update feature hooks, views, and UI components in `src/`
   - Preserve desktop behavior and existing feature boundaries where practical

5. **Verify data and role rules**
   - Respect `EE.ID` as the unique merge key
   - Keep admin-only actions protected
   - Keep import, edit, move, and reset flows explicit and auditable

6. **Run checks**
   ```bash
   npm run check:quality
   ```

7. **Finish cleanly**
   - Review the diff for regressions
   - Update docs only if source-of-truth behavior changed
   - Keep the desktop repo free of unrelated web-planning artifacts
