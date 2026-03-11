# Approval Workflow Specification

## Purpose
Define how the system reviews and approves pending receive and return requests before changing official asset and stock data.

## Requirements

### Requirement: Pending Request Queue
The system MUST maintain a queue of pending receive and return requests for IT review.

#### Scenario: View pending requests
- WHEN an authorized IT user opens the review area
- THEN the system displays pending receive and return requests awaiting review

### Requirement: Request Review Details
The system MUST allow an authorized IT user to inspect the submitted employee and asset details before approval or rejection.

#### Scenario: View request details
- WHEN an authorized IT user opens a pending request
- THEN the system displays the employee information, submitted asset codes, request type, and request status

### Requirement: Approve Pending Requests
The system MUST allow an authorized IT user to approve a pending request.

#### Scenario: Approve a pending receive request
- WHEN an authorized IT user approves a pending receive request
- THEN the system marks the request as approved
- AND the system applies the final receive effects defined by the receive flow

#### Scenario: Approve a pending return request
- WHEN an authorized IT user approves a pending return request
- THEN the system marks the request as approved
- AND the system applies the final return effects defined by the return flow

### Requirement: Reject Pending Requests
The system MUST allow an authorized IT user to reject a pending request.

#### Scenario: Reject a pending request
- WHEN an authorized IT user rejects a pending request
- THEN the system marks the request as rejected
- AND the system does not apply official stock or assignment changes

### Requirement: Approval Gate for Official Data Changes
The system MUST prevent official stock and assignment mutations until a pending request is approved.

#### Scenario: Submitted request remains non-final before approval
- WHEN a request has been submitted but not approved
- THEN the system MUST not finalize assignment, unassignment, or stock updates

### Requirement: Role-Based Approval Permission
The system MUST restrict approve and reject actions to authorized management roles.

#### Scenario: Allow approval by authorized role
- WHEN a user with approval permission attempts to review a pending request
- THEN the system allows the approval or rejection action

#### Scenario: Reject approval by unauthorized role
- WHEN a user without approval permission attempts to approve or reject a pending request
- THEN the system MUST reject the action

### Requirement: Approval Auditability
The system MUST store the reviewer, review result, and review time for each approved or rejected request.

#### Scenario: Record approval metadata
- WHEN an authorized IT user approves or rejects a pending request
- THEN the system stores the reviewer identity, review outcome, and review timestamp
