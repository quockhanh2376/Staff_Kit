# Audit Log Specification

## Purpose
Define how the system records and exposes audit trails for important actions related to assets, receive and return requests, approval actions, inventory-only transactions, users, and permissions.

## Requirements

### Requirement: Logging of Important Actions
The system MUST record audit logs for important actions related to assets, receive and return requests, approval actions, inventory-only transactions, users, and authorization changes.

#### Scenario: Log asset creation
- WHEN a new asset is created successfully
- THEN the system records an audit log entry for asset creation

#### Scenario: Log request review action
- WHEN an authorized reviewer approves or rejects a receive or return request
- THEN the system records an audit log entry for the review action

#### Scenario: Log role or permission changes
- WHEN an administrator changes a user's roles or permissions
- THEN the system records an audit log entry for the authorization change

### Requirement: Minimum Audit Log Content
The system MUST store at least the timestamp, actor, action type, affected object, and processing result for each audit log entry.

#### Scenario: View audit log details
- WHEN a user with permission opens an audit log entry
- THEN the system displays the minimum stored audit log fields

### Requirement: Logging of Successful and Failed Sensitive Actions
The system SHOULD record both successful and rejected or failed attempts for sensitive operations.

#### Scenario: Record a failed login attempt
- WHEN a user submits invalid login credentials
- THEN the system SHOULD record a failed login event

#### Scenario: Record an access denial
- WHEN a user is denied permission for a sensitive operation
- THEN the system SHOULD record an access-denied event

### Requirement: Audit Log Search and Filtering
The system SHALL allow users with permission to search and filter audit logs by time, actor, action type, or related object.

#### Scenario: Filter logs by actor
- WHEN a user with permission filters audit logs by a specific actor
- THEN the system returns only the matching audit log entries

### Requirement: Relative Immutability of Audit Logs
The system MUST not allow ordinary users to directly modify or delete audit log entries.

#### Scenario: Reject audit log modification by an ordinary user
- WHEN a non-administrative user attempts to modify or delete an audit log entry
- THEN the system MUST reject the request

### Requirement: Audit Log Linking to Business Objects
The system SHALL allow audit log entries to be linked to related assets, requests, inventory transactions, or user accounts for traceability.

#### Scenario: View logs from an asset detail page
- WHEN a user with permission opens the history or audit section for an asset
- THEN the system displays audit log entries related to that asset
### Requirement: Retention of Audit Logs
The system SHOULD retain audit logs for a configurable period to support auditing and compliance needs.
