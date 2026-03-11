# Proposal: add-qr-receive

## Why
The system needs a QR-based receive workflow so new employees can confirm asset handover using their personal phones.

Without this workflow:
- IT handover is harder to standardize
- employee-entered confirmation is not captured in a structured way
- assignment preparation and approval are mixed together
- asset tracking can become inconsistent

## What Changes
This change adds:
- QR receive session creation from the IT management app
- mobile receive form opened by scanning the QR code
- employee identity input for receive workflow
- typed asset-code search and validation
- multi-asset receive submission
- pending receive review before final approval
- approval application into official assignment data

## Impact
Affected specs:
- receive-flow
- approval-workflow
- assets
- audit-log

Affected code areas:
- QR session generation
- mobile receive form UI
- asset search API or service
- pending request persistence
- review and approval workflow
- assignment creation logic
- audit logging
