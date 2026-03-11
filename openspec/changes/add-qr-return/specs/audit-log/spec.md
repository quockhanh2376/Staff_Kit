# Delta for audit-log

## ADDED Requirements

### Requirement: Audit Return Session Creation
The system MUST record an audit event when an IT user creates a QR return session.

#### Scenario: Log return session creation
- WHEN an authorized IT user starts a return session
- THEN the system records an audit log entry for the return session creation

### Requirement: Audit Return Submission and Review
The system MUST record audit events for return request submission and return review outcomes.

#### Scenario: Log return request submission
- WHEN an employee submits a return request
- THEN the system records an audit log entry for the return submission

#### Scenario: Log return approval or rejection
- WHEN an authorized IT user approves or rejects a return request
- THEN the system records an audit log entry with the reviewer identity and review outcome
