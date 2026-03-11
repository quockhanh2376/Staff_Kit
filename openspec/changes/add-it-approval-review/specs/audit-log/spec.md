# Delta for audit-log

## ADDED Requirements

### Requirement: Audit IT Review Decisions
The system MUST record audit events for approval and rejection decisions on pending receive and return requests.

#### Scenario: Log an approval decision
- WHEN an authorized IT user approves a pending request
- THEN the system records an audit event with the request type, reviewer, and decision result

#### Scenario: Log a rejection decision
- WHEN an authorized IT user rejects a pending request
- THEN the system records an audit event with the request type, reviewer, and decision result

### Requirement: Audit Denied Review Attempts
The system SHALL record denied approval or rejection attempts for traceability.

#### Scenario: Log an unauthorized review attempt
- WHEN a user without approval permission attempts to approve or reject a pending request
- THEN the system SHOULD record an audit event for the denied review attempt
