# Super Admin Computer Name Edit Design

**Goal:** Allow only `super_admin` accounts to edit the employee-table `Computer Name` field while keeping the existing employee table edit flow for all other columns.

## Scope

- Employee table only.
- `super_admin` can edit `Computer Name`.
- `admin` and `user` cannot edit `Computer Name`.
- Existing edit permissions for other employee columns stay unchanged.

## Behavior

- Add a dedicated frontend capability for `Computer Name` editing instead of reusing the generic `canEditEmployeeTable` flag.
- When a `super_admin` enters table edit mode, the `Computer Name` cell becomes editable.
- For non-`super_admin` accounts, `Computer Name` remains read-only.
- Saving edits persists the raw stored `computerName` field through the existing employee update API.

## Data Semantics

- The employee table currently displays `Computer Name` as:
  - active laptop-derived names first
  - stored employee fallback second
- This change only makes the stored fallback value editable.
- If an employee currently has active laptop-derived computer names, those derived values still take precedence in display after save.

## Implementation Notes

- Keep the permission check in frontend auth capabilities.
- Update employee-table edit helpers so `computerName` is editable only when explicitly allowed.
- Keep duplicate highlighting logic unchanged; it should continue to inspect displayed values, not the editable fallback source.
- Update the edit-mode helper copy so the UI states that only `Super Admin` can edit `Computer Name`.

## Testing

- Add/extend script tests for:
  - auth capability derivation
  - editable-column rules
  - save payload behavior for `computerName`
- Verify the employee table still treats `Computer Name` as read-only for non-`super_admin` accounts.
