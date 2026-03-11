# Delta for return-flow

## ADDED Requirements

### Requirement: Return Session Tokenized Access
The system MUST create a unique return session token for each QR-based return session.

#### Scenario: Create a unique return session token
- WHEN an authorized IT user starts a return session
- THEN the system generates a unique token for that return session
- AND the QR code resolves to that specific session

### Requirement: Return Session Expiry
The system SHALL support expiring a return session after a limited period or after successful use.

#### Scenario: Reject an expired return session
- WHEN an employee opens a return session that is expired
- THEN the system SHOULD reject access to the return form
- AND the system returns a session-expired message

### Requirement: Multi-Asset Return Request Details
The system MUST store all asset entries included in a submitted return request.

#### Scenario: Store multiple assets in one return request
- WHEN an employee submits a return request containing multiple valid asset codes
- THEN the system stores all submitted assets under the same pending return request

## MODIFIED Requirements

### Requirement: Pending Review Before Final Return
The system MUST store employee return submissions with employee and asset details as pending requests before adding assets back to official stock.

#### Scenario: Save a pending return request with employee and asset details
- WHEN an employee submits a valid return form
- THEN the system stores the employee identity information and submitted asset codes in a pending review state
- AND the system does not yet return the asset to official stock
