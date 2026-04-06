# QR Return Submit Slice Design

## Goal
Add the first meaningful `ST 2.0.2` return-flow slice by letting employees submit a QR-based return request that becomes a pending request for later IT review, without exposing a new visible return review UI yet.

## Scope
- Accept return submissions as pending requests with `request_type = "return"`.
- Validate submitted return assets against the employee's active loans.
- Keep the current desktop review UI borrow-only for now.
- Hard-block borrow approval logic from processing non-borrow requests.
- Keep the current pending borrow queue borrow-only until the later shared review slice lands.
- Audit log the return submission.

## Out of Scope
- Return approval or rejection workflow
- Desktop return review UI
- Separate return navigation or settings surface
- Reviewed payload model
- Stock or asset-loan mutation on return submit

## Design

### 1. Data Model
- Reuse the existing `borrow_requests` / `borrow_request_items` tables.
- Use `request_type` to distinguish `borrow` from `return`.
- Keep `BorrowRequestRecord` as the current cross-request DTO for this slice.
- Treat `asset_loans.returned_at IS NULL` as the source of truth for active return eligibility.

### 2. Submit Validation
- `submittedEmployeeId` must resolve to a real employee.
- Submitted asset codes must be unique within the request.
- Each submitted asset code must exist.
- Each submitted asset must currently have an active loan for the submitted employee.
- Assets assigned to a different employee, already returned, or not on loan are rejected.

### 3. Submission Flow
- Add a backend path that inserts:
  - one pending request row with `request_type = "return"`
  - matching request-item snapshot rows
  - one audit log entry for `return_request.submit`
- Submission must not mutate:
  - asset status
  - loan `returned_at`
  - assignment state

### 4. Safety Guard
- Existing approve/reject backend logic must reject non-`borrow` requests until the dedicated return review slice lands.
- This prevents the current borrow-only admin surface from applying the wrong business behavior to return requests.
- Existing pending borrow queue queries must stay borrow-only so hidden return requests do not leak into the shipped borrow admin UI.

### 5. Public Surface
- Expose a LAN API endpoint for pending return submit.
- Keep the first slice backend-oriented; if a public `/return` page is needed for a thin vertical path, it should stay minimal and not require desktop UI changes.

## Files Expected
- `src-tauri/src/db/borrow.rs`
- `src-tauri/src/lan_server.rs`
- `src-tauri/src/lib.rs` if new command exposure is needed
- `src/types/staff.ts`
- `src/services/staff-api.ts` only if a typed TS contract is needed in this slice

## Testing Strategy
- Add Rust service tests first for:
  - valid return submit creates a pending `return` request
  - submit rejects unknown employee
  - submit rejects assets not actively loaned to that employee
  - submit rejects duplicate asset codes
  - borrow approve/reject actions reject non-borrow requests
  - pending borrow queue excludes hidden return requests
  - `return_request.submit` audit log is written
- Add LAN endpoint tests for valid and invalid return submit payloads.
