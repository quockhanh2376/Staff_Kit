# Delta for approval-workflow

## ADDED Requirements

### Requirement: Receive Requests in Review Queue
The system MUST place submitted receive requests into the IT review queue.

#### Scenario: Show a pending receive request in the queue
- WHEN an employee submits a valid receive request
- THEN the system adds that request to the pending IT review queue

### Requirement: Receive Review Outcome Application
The system MUST apply official receive effects only through an IT review decision.

#### Scenario: Approve a pending receive request
- WHEN an authorized IT user approves a pending receive request
- THEN the system applies the final receive effects defined by the receive flow
- AND the system records the review outcome

#### Scenario: Reject a pending receive request
- WHEN an authorized IT user rejects a pending receive request
- THEN the system keeps official stock and assignment data unchanged
- AND the system records the review outcome
