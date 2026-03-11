# Return Flow Specification

## Purpose
Define how the system handles QR-based asset return for employees returning company assets.

## Requirements

### Requirement: Return Session Creation
The system MUST allow an authorized IT user to start a return session.

#### Scenario: Create a return session
- WHEN an authorized user clicks `Scan Return`
- THEN the system creates a return session
- AND the system displays a QR code for that session

### Requirement: QR-Based Return Access
The system MUST allow an employee to access the mobile return form by scanning the QR code for an active return session.

#### Scenario: Open the return form from QR code
- WHEN an employee scans a valid active return QR code
- THEN the system opens the return form associated with that session

### Requirement: Multi-Asset Return Submission
The system MUST allow an employee to submit multiple returned assets in a single return request.

#### Scenario: Submit a return request with multiple assets
- WHEN an employee submits a valid return form containing one or more asset codes
- THEN the system creates one return request containing all submitted assets

### Requirement: Asset Search and Validation in Return Form
The system MUST support typed asset-code input with search and validation against existing asset records.

#### Scenario: Search for an assigned asset code
- WHEN an employee types part or all of an asset code in the return form
- THEN the system returns matching asset records relevant to the return workflow

#### Scenario: Reject an unknown asset code in return
- WHEN an employee submits an asset code that does not exist in the system
- THEN the system MUST reject that asset entry

### Requirement: Eligibility for Return
The system MUST only allow assets that are currently assigned and eligible for return to be included in a return request.

#### Scenario: Accept a valid assigned asset
- WHEN a submitted asset exists and is currently assigned
- THEN the system accepts the asset in the pending return request

#### Scenario: Reject an asset that is not assigned
- WHEN a submitted asset is not currently assigned
- THEN the system MUST reject that asset entry

### Requirement: Pending Review Before Final Return
The system MUST store employee return submissions as pending requests before adding assets back to official stock.

#### Scenario: Save a pending return request
- WHEN an employee submits a valid return form
- THEN the system stores the request in a pending review state
- AND the system does not yet return the asset to official stock

### Requirement: Approval Required for Final Return
The system MUST apply official return effects only after IT approval.

#### Scenario: Approve a return request
- WHEN an authorized IT user approves a pending return request
- THEN the system closes the active assignment
- AND the system updates the asset status to In Stock
- AND the system returns the asset to official stock

#### Scenario: Reject a return request
- WHEN an authorized IT user rejects a pending return request
- THEN the system keeps the current assignment and stock data unchanged

### Requirement: Prevent Duplicate Returns
The system MUST prevent the same asset from being submitted more than once within a single return request.

#### Scenario: Detect a duplicate asset in one return form
- WHEN the same asset code is added multiple times in one return submission
- THEN the system MUST reject the duplicated entry
