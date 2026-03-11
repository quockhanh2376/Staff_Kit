## Context

Staff Kit uses a staged workflow where employee QR submissions create pending receive or return requests, and IT must approve or reject them before official stock or assignment data changes. The approval review experience needs to be shared and consistent across both receive and return flows, while remaining restricted to authorized IT management roles.

## Goals / Non-Goals

**Goals:**
- Provide a unified IT review queue for pending receive and return requests
- Show enough employee, asset, and request detail for a reviewer to make a decision
- Restrict review actions to authorized management roles
- Record review outcomes with clear auditability

**Non-Goals:**
- Redesign the QR receive form itself
- Redesign the QR return form itself
- Add multi-step approval chains or non-IT approvers
- Add advanced maintenance or repair workflows

## Decisions

### 1. Use a unified pending review queue
Receive and return requests SHOULD appear in one review area with request-type context so IT can work from a single operational queue.

### 2. Keep approval decisions server-authoritative
Approve and reject actions MUST be enforced server-side based on role and permission checks, not only hidden in the client UI.

### 3. Preserve submitted request data for review
Pending requests MUST keep the submitted employee and asset details intact so reviewers can inspect what was actually submitted before deciding.

### 4. Audit every review outcome
Every approval, rejection, and denied review attempt SHOULD produce audit data suitable for traceability and support.

## Risks / Trade-offs

- [Shared review queue complexity] -> Mitigation: keep request type explicit and show only fields needed for review.
- [Permission mistakes could alter stock incorrectly] -> Mitigation: enforce server-side role checks and test approval permissions directly.
- [Receive and return may need slightly different review details] -> Mitigation: use a shared shell with request-type-specific detail sections.
