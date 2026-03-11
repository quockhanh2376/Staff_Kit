# Delta for audit-log

## ADDED Requirements

### Requirement: Audit Receive Session Creation
The system MUST record an audit event when an IT user creates a QR receive session.

#### Scenario: Log receive session creation
- WHEN an authorized IT user starts a receive session
- THEN the system records an audit log entry for the receive session creation

### Requirement: Audit Receive Submission and Review
The system MUST record audit events for receive request submission and receive review outcomes.

#### Scenario: Log receive request submission
- WHEN an employee submits a receive request
- THEN the system records an audit log entry for the receive submission

#### Scenario: Log receive approval or rejection
- WHEN an authorized IT user approves or rejects a receive request
- THEN the system records an audit log entry with the reviewer identity and review outcome
