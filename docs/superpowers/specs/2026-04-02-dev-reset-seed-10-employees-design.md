# Dev Reset Auto-Seed 10 Employees Design

## Goal
Make the desktop app easier to test during development by automatically reseeding a small, fixed employee dataset after `Reset All Data`, without changing production release behavior.

## Scope
- Apply to the `Staff_Kit` desktop app only.
- Trigger only from the existing `Reset All Data (Temporary)` flow in `Settings`.
- Seed exactly `10` employee records after reset.
- Return enough reset metadata for the frontend to show whether dev reseeding happened.
- Seed enough related data to make the employee workspace immediately testable:
  - team membership
  - multiple staff groups
  - search/filter values
  - a few dynamic fields that exercise the column drawer

## Non-Goals
- Do not change `Reset All Data` behavior in release builds.
- Do not add a new manual seed button for this slice.
- Do not seed borrow requests, return requests, or asset records.
- Do not import from external files in this slice.

## Design

### 1. Reset Behavior
- `Reset All Data` keeps its current two-step confirmation flow.
- The reset still clears employees, teams, and dynamic employee columns first.
- After the reset transaction rebuilds the clean employee schema state, the app should seed a small deterministic dataset only when the Rust binary is compiled with `cfg!(debug_assertions)`.
- The clear phase and optional debug reseed phase must run inside the same SQLite transaction.
- This means:
  - local `tauri:dev` runs and debug-profile Rust tests should use the reseed path
  - packaged release builds and `--release` Rust runs must not reseed
- In non-debug builds, reset should remain empty exactly as it is today.
- If debug reseeding fails for any reason, the whole reset must roll back and return an error so the app is not left in a half-cleared state.

### 2. Dev-Only Guard
- The seed path must be protected by a Rust-side `cfg!(debug_assertions)` guard so packaged production builds are unaffected.
- The backend reset command must return `Result<ResetAllDataResult, String>` instead of a bare boolean.
- `ResetAllDataResult` must contain:
  - `cleared: bool`
  - `seededTestEmployees: i64`
- Successful reset responses must be:
  - `{ cleared: true, seededTestEmployees: 10 }` for debug/dev reseed runs
  - `{ cleared: true, seededTestEmployees: 0 }` for release-mode resets
- Failure responses must use the existing Tauri `Err(String)` path and must not commit partial clear/reseed work.
- The frontend should keep the existing reload behavior after reset, then show a success message based on `seededTestEmployees`.

### 3. Seed Dataset Shape
- The dataset should be fixed and deterministic, not random.
- It should create:
  - `10` employees
  - `4` teams reused across those employees
    - `IT Support`
    - `Finance`
    - `People Ops`
    - `Operations`
  - records across these exact staff-group minimums:
    - `employee_list`: `4`
    - `onboarding`: `2`
    - `offboarding`: `2`
    - `internal_movement`: `2`
- Each employee should include enough realistic fields to exercise core desktop flows:
  - `employeeId`
  - `fullName`
  - `teamName`
  - `project`
  - `jobTitle`
  - `email`
  - `computerName`
  - `staffGroup`
- At least `6` of the `10` seeded employees should include dynamic field values.
- The dataset should recreate at least these dynamic column keys:
  - `computer_name_2`
  - `endpoint_agent`
  - `recruiter`
  - `offer`
- At least one seeded employee should leave each dynamic field blank so the column drawer shows useful mixed-state data after reset.

### 4. Seed Source And Ownership
- The seed dataset should live in Rust backend code near the reset flow, not in frontend code.
- The reset path should use the same employee upsert path already used by the app so validation, team creation, dynamic field definition creation, and FTS state stay consistent with normal writes.
- The seed helper should be private to the DB layer because it is a dev/test convenience, not a public app feature.

### 5. Settings UX
- The existing reset button label can stay unchanged for this slice.
- After a successful reset, the settings state should surface a success message based on backend metadata:
  - when `seededTestEmployees > 0`: mention that test employees were reloaded
  - when `seededTestEmployees === 0`: keep a generic reset-complete message
- The frontend should not hardcode its own debug-mode detection for this slice.

## Module Boundaries
- `src-tauri/src/db/mod.rs`
  - Owns reset orchestration, the debug-only reseed guard, and the reset result metadata returned to Tauri.
- `src-tauri/src/db/employee.rs`
  - Continues to own employee payload normalization and upsert rules reused by the reset reseed helper.
- `src/services/staff-api.ts`
  - Owns the updated TypeScript contract for the reset result payload.
- `src/features/settings/useSettingsState.ts`
  - Owns the reset action message shown after the backend completes.

## Data Rules
- Seeded employee IDs, emails, and computer names must be unique inside the sample dataset.
- The dataset should be safe to run repeatedly after every reset and should not rely on pre-existing rows.
- Running reset multiple times in debug mode must still end at exactly `10` employees, not duplicates or drifted teams/columns.
- Team rows and dynamic field definitions should be recreated through normal backend write paths, not direct ad-hoc SQL inserts, unless the normal path would materially complicate the reset transaction.

## Testing Strategy
- Add backend coverage proving that:
  - reset still clears employee-side data correctly
  - debug/dev reset reseeds exactly `10` employees
  - release-mode reset seeds `0` employees
  - running debug reset repeatedly is idempotent and still ends at exactly `10` employees
  - seeded teams are recreated
  - seeded dynamic columns are recreated
  - if the reseed helper fails, the transaction rolls back and pre-reset rows remain intact
- Add frontend coverage for the reset success message because the command result contract changes from a bare boolean to structured metadata.
- Run full project quality rails after implementation.

## Acceptance Criteria
- In a debug/dev desktop run, after clicking `Reset All Data (Temporary)` and confirming twice, the app reloads with exactly `10` seeded employees.
- The seeded employees are visible across multiple staff groups and teams.
- The column drawer still has dynamic fields to test after reset.
- In production/release builds, `Reset All Data` does not auto-seed this test dataset and reports `seededTestEmployees = 0`.
