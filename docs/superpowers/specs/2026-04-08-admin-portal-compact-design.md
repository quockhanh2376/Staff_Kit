# Admin Portal Compact Design

**Date:** 2026-04-08

## Goal

Redesign the `Admin Portal (Local Accounts)` card in Settings so it matches the compact visual language of `Database & Backup`, including:

- compact header with icon + chevron
- collapsible body
- auto-collapse after 60 seconds of inactivity inside the card
- import target toolbar rendered as a compact import/dropdown control inside the card body
- unified dark surfaces, borders, spacing, and button treatment

This slice is frontend-only. It must not change account, import, or auth backend behavior.

## Current Context

- `SettingsView.tsx` already contains the current `Admin Portal` card, `Database & Backup`, and `AssetDashboard`.
- `Database & Backup` already uses the compact dark tone and `useIdleCollapse(60000)`.
- The quick-switch `Use` button has already been removed; account changes should now happen through the real login flow.
- Import controls were previously removed from Settings, but this redesign reintroduces only the compact `Import -> Target` toolbar inside `Admin Portal`.

## Desired UI

### Header

The card header should visually align with `Database & Backup`:

- icon on the left
- title `Admin Portal`
- chevron button for collapse/expand
- no helper paragraph under the title

When collapsed, the entire outer frame should shrink to header height only.

### Body

The expanded body should contain three sections:

1. `Import toolbar`
   - compact row at the top of the card body
   - left segment is the import action button
   - right segment includes a small `TO:` label and a dropdown
   - dropdown options:
     - `Employee List`
     - `Onboarding`
     - `Offboarding`
     - `Movement`
   - this uses the existing import target state/behavior

2. `Create Local User`
   - keep the existing fields and add-account behavior
   - compact spacing and surfaces to match the reference

3. `Accounts`
   - list local accounts in a compact scrollable panel
   - keep edit, reset password, and delete actions
   - do not reintroduce the removed quick-switch action

## Interaction Rules

- The card auto-collapses after 60 seconds with no activity inside the card.
- Activity includes click, input, focus, keydown, pointer interaction, and scrolling within the account list.
- Clicking the header toggles collapse/expand.
- Clicking header action controls must not trigger accidental double-toggle behavior.
- Import dropdown changes only the selected import target; it does not alter import backend flow.

## Visual Rules

The card should borrow the same visual system already used by `Database & Backup`:

- outer shell: `#161b22`
- sub-panels: `#1c2128`
- inputs/list rows: `#0d1117`
- slate text hierarchy for headings, body, and muted copy
- emerald primary actions
- compact border radius and spacing

## Non-Goals

- no backend changes
- no account permission redesign
- no new import logic
- no login/session refactor
- no Asset Dashboard layout changes in this slice
