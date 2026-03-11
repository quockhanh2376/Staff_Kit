# Receive Flow Specification

## Purpose
Define how the system handles QR-based asset receiving for employees joining the company.

## Requirements

### Requirement: Receive Session Creation
The system MUST allow an authorized IT user to start a receive session.

#### Scenario: Create a receive session
- WHEN an authorized user clicks `Scan Receive`
- THEN the system creates a receive session
- AND the system displays a QR code for that session

### Requirement: QR-Based Receive Access
The system MUST allow an employee to access the mobile receive form by scanning the QR code for an active receive session.

#### Scenario: Open the receive form from QR code
- WHEN an employee scans a valid active receive QR code
- THEN the system opens the receive form associated with that session

### Requirement: Multi-Asset Receive Submission
The system MUST allow an employee to submit multiple assets in a single receive request.

#### Scenario: Submit a receive request with multiple assets
- WHEN an employee submits a valid receive form containing one or more asset codes
- THEN the system creates one receive request containing all submitted assets

### Requirement: Asset Search and Validation in Receive Form
The system MUST support typed asset-code input with search and validation against preloaded asset records.

#### Scenario: Search for an existing asset code
- WHEN an employee types part or all of an asset code in the receive form
- THEN the system returns matching preloaded asset records

#### Scenario: Validate a submitted asset code
- WHEN an employee submits an asset code that exists and is eligible for assignment
- THEN the system accepts the asset for the receive request

#### Scenario: Reject an unknown asset code
- WHEN an employee submits an asset code that does not exist in the system
- THEN the system MUST reject that asset entry

### Requirement: Eligibility for Receive
The system MUST only allow assets that are valid for assignment to be included in a receive request.

#### Scenario: Accept an available asset
- WHEN a submitted asset exists and is in an assignable status
- THEN the system allows the asset in the pending receive request

#### Scenario: Reject a non-assignable asset
- WHEN a submitted asset is already assigned, retired, or otherwise not eligible for assignment
- THEN the system MUST reject that asset entry

### Requirement: Pending Review Before Final Assignment
The system MUST store employee receive submissions as pending requests before applying official stock or assignment changes.

#### Scenario: Save a pending receive request
- WHEN an employee submits a valid receive form
- THEN the system stores the request in a pending review state
- AND the system does not yet finalize assignment

### Requirement: Approval Required for Final Receive
The system MUST apply official assignment changes only after IT approval.

#### Scenario: Approve a receive request
- WHEN an authorized IT user approves a pending receive request
- THEN the system creates the final assignment records
- AND the system links the approved assets to the employee
- AND the system updates the asset statuses to Assigned

#### Scenario: Reject a receive request
- WHEN an authorized IT user rejects a pending receive request
- THEN the system keeps official stock and assignment data unchanged

### Requirement: Prevent Duplicate Assets in the Same Request
The system MUST prevent the same asset from being submitted more than once within a single receive request.

#### Scenario: Detect a duplicate asset in one receive form
- WHEN the same asset code is added multiple times in one receive submission
- THEN the system MUST reject the duplicated entry
