# Delta for approval-workflow

## ADDED Requirements

### Requirement: Unified IT Review Queue
The system MUST provide a single IT review queue for pending receive and return requests.

#### Scenario: View mixed pending request types
- WHEN an authorized IT user opens the review queue
- THEN the system displays both pending receive and pending return requests with their request type

### Requirement: Review Decision Persistence
The system MUST persist the reviewer identity, review outcome, and review time for each decision.

#### Scenario: Save review decision data
- WHEN an authorized IT user approves or rejects a pending request
- THEN the system stores the reviewer identity, decision result, and decision timestamp
