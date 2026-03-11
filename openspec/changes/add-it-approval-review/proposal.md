# Proposal: add-it-approval-review

## Why
The system needs a dedicated IT approval review workflow so pending receive and return requests can be reviewed consistently before any official stock or assignment mutation is applied.

Without this workflow:
- pending requests are harder to review consistently
- approval permissions can become ambiguous
- receive and return flows can diverge operationally
- auditability of review decisions becomes weaker

## What Changes
This change adds:
- a dedicated IT review queue for pending receive and return requests
- detailed review screens for submitted employee and asset data
- approve and reject actions restricted to authorized management roles
- consistent review state transitions across receive and return
- audit logging for review decisions and denied review attempts

## Impact
Affected specs:
- approval-workflow
- users-roles
- receive-flow
- return-flow
- audit-log

Affected code areas:
- pending request queue and filters
- review detail UI
- role and permission enforcement
- approval decision handlers
- audit logging
