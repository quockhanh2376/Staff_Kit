# Delta for receive-flow

## ADDED Requirements

### Requirement: Receive Session Tokenized Access
The system MUST create a unique receive session token for each QR-based receive session.

#### Scenario: Create a unique receive session token
- WHEN an authorized IT user starts a receive session
- THEN the system generates a unique token for that receive session
- AND the QR code resolves to that specific session

### Requirement: Receive Session Expiry
The system SHALL support expiring a receive session after a limited period or after successful use.

#### Scenario: Reject an expired receive session
- WHEN an employee opens a receive session that is expired
- THEN the system SHOULD reject access to the receive form
- AND the system returns a session-expired message

### Requirement: Multi-Asset Receive Request Details
The system MUST store all asset entries included in a submitted receive request.

#### Scenario: Store multiple assets in one receive request
- WHEN an employee submits a receive request containing multiple valid asset codes
- THEN the system stores all submitted assets under the same pending receive request

## MODIFIED Requirements

### Requirement: Pending Review Before Final Assignment
The system MUST store employee receive submissions with employee and asset details as pending requests before applying official stock or assignment changes.

#### Scenario: Save a pending receive request with employee and asset details
- WHEN an employee submits a valid receive form
- THEN the system stores the employee identity information and submitted asset codes in a pending review state
- AND the system does not yet finalize assignment
