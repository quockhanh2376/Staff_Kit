# Pending Reviews Design

## Goal
Turn `Pending Reviews` into an IT review workstation where admins inspect pending receive/return requests, correct limited fields, and make the final `Approve` or `Reject` decision with full auditability.

## Scope
- Only `PENDING` requests appear in the queue.
- IT can review and correct submitted data before final decision.
- `Submitted snapshot` remains immutable.
- `Approved` and `Rejected` requests leave the queue and are tracked through audit logs, not through this screen.

## Design

### 1. Review Model
- `Pending Reviews` is an operational queue, not a history browser.
- Requests move through three concepts:
  - `Submitted snapshot`: raw employee-submitted data, read-only.
  - `Reviewed payload`: IT-corrected data used for final validation and decision.
  - `Decision`: `APPROVED` or `REJECTED`.
- The queue only shows `PENDING` items.

### 2. Allowed Corrections

#### Receive requests
- IT may edit `employeeId`.
- IT may add or remove `assetCodes`.
- Added assets must already exist in the system.
- Asset creation stays in the separate `Assets` / preload flow.
- IT may edit `review notes`.

#### Return requests
- IT may edit `employeeId`.
- IT may remove incorrect `assetCodes`.
- IT may not add new assets during return review.
- IT may edit `review notes`.

### 3. UI Structure

#### Queue page
- Show only `PENDING` requests.
- Each queue item shows:
  - `requestType`
  - `requestKey`
  - `employeeId`
  - `fullName`
  - asset count
  - submitted timestamp
  - validation hint if the request is currently invalid
  - `Review` action
- Do not expose direct `Approve` / `Reject` actions from the list.

#### Detail page
- Split into three panels:
  - `Submitted snapshot`
  - `IT review form`
  - `Decision panel`
- Show a simple diff between submitted data and reviewed data.
- Highlight changed fields so reviewers and auditors can identify corrections quickly.

### 4. Decision Behavior

#### Approve
- Validate the reviewed payload at approval time.
- If validation passes, apply official stock / assignment effects.
- Remove the request from the queue.

#### Reject
- Do not apply official stock / assignment effects.
- Require a rejection reason in `review notes`.
- Remove the request from the queue.

### 5. Data and Audit
- Never overwrite the submitted snapshot.
- Store reviewed data separately from submitted data.
- Use reviewed data when applying approval effects.
- For each decision, audit logs must capture:
  - request type
  - request key
  - reviewer identity
  - decision
  - submitted payload
  - reviewed payload
  - before/after diff
  - timestamp
- Phase 1 intentionally does not support autosaved drafts.
- Review changes become durable only when IT clicks `Approve` or `Reject`.

### 6. Validation Rules

#### Receive
- Employee must exist.
- Assets must exist.
- Assets must be assignable.
- Reviewed asset set must not contain duplicates.

#### Return
- Employee must exist.
- Remaining assets must still be eligible active assignments.
- Approval is blocked if all assets are removed.
- Review cannot add new assets.

### 7. Testing Strategy
- Start with service-level tests:
  - approve receive with corrected `employeeId`
  - approve receive after valid asset add/remove edits
  - reject receive with notes
  - approve return after removing incorrect asset(s)
  - reject return with notes
  - unauthorized review attempt is rejected
  - invalid reviewed payload blocks approval
- Then add UI tests:
  - queue opens request detail
  - approve removes request from queue
  - reject removes request from queue
  - audit log records the review event

## Out of Scope
- Autosave review drafts
- Reviewing approved/rejected history inside `Pending Reviews`
- Creating new assets from the review screen
- Editing official stock or assignment data directly from the UI
