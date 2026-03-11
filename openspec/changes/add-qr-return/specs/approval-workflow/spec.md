# Delta for approval-workflow

## ADDED Requirements

### Requirement: Return Requests in Review Queue
The system MUST place submitted return requests into the IT review queue.

#### Scenario: Show a pending return request in the queue
- WHEN an employee submits a valid return request
- THEN the system adds that request to the pending IT review queue

### Requirement: Return Review Outcome Application
The system MUST apply official return effects only through an IT review decision.

#### Scenario: Approve a pending return request
- WHEN an authorized IT user approves a pending return request
- THEN the system applies the final return effects defined by the return flow
- AND the system records the review outcome

#### Scenario: Reject a pending return request
- WHEN an authorized IT user rejects a pending return request
- THEN the system keeps official stock and assignment data unchanged
- AND the system records the review outcome
