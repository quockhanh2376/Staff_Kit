# Proposal: add-qr-return

## Why
The system needs a QR-based return workflow so employees can submit asset returns using their personal phones while IT keeps final stock changes under review control.

Without this workflow:
- return handover is harder to standardize
- employee return intent is not captured in a structured way
- asset return and approval are mixed together
- assignment and stock records can become inconsistent

## What Changes
This change adds:
- QR return session creation from the IT management app
- mobile return form opened by scanning the QR code
- employee identity input for return workflow
- typed asset-code search and validation for return
- multi-asset return submission
- pending return review before final approval
- approval application into official return and stock data

## Impact
Affected specs:
- return-flow
- approval-workflow
- assets
- audit-log

Affected code areas:
- QR session generation
- mobile return form UI
- asset search API or service
- pending request persistence
- review and approval workflow
- assignment close-out logic
- audit logging
