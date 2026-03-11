# Asset Specification

## Purpose
Define how the system manages asset identity, core attributes, status, and baseline lifecycle behavior.

## Requirements

### Requirement: Asset Creation
The system MUST allow authorized users to create a new asset when all required fields are provided.

#### Scenario: Create a valid asset
- WHEN an authorized user submits a valid asset creation request with all required fields
- THEN the system creates a new asset record
- AND the system assigns a unique asset code to the asset

#### Scenario: Missing required fields
- WHEN a user submits an asset creation request without the required asset name or asset type
- THEN the system MUST reject the request
- AND the system returns a validation error that identifies the missing fields

### Requirement: Unique Asset Code
The system SHALL ensure that each asset has a unique asset code across the system.

#### Scenario: Generate or validate a unique code
- WHEN a new asset is created successfully
- THEN the system SHALL generate or confirm a non-duplicated asset code

#### Scenario: Reject a duplicated asset code
- WHEN a request attempts to save an asset with an asset code that already exists
- THEN the system MUST reject the request
- AND the system records or returns a duplicate-code error

### Requirement: Core Asset Attributes
The system MUST store the core attributes of an asset, including asset code, asset name, asset type, current status, recorded date, and owning or managing unit.

#### Scenario: View asset details
- WHEN a user with permission opens the asset detail view
- THEN the system displays the stored core asset attributes

### Requirement: Asset Status Management
The system MUST manage asset status using a predefined set of allowed statuses.

#### Scenario: Assign a default status to a new asset
- WHEN a new asset is created
- THEN the system assigns the default initial asset status

#### Scenario: Perform a valid status transition
- WHEN a user with permission submits a valid status transition
- THEN the system updates the asset to the new status

#### Scenario: Reject an invalid status transition
- WHEN a user attempts to move an asset to a status that is not allowed by business rules
- THEN the system MUST reject the update
- AND the system returns the reason for rejection

### Requirement: Asset Update
The system MUST allow authorized users to update asset information.

#### Scenario: Update asset information successfully
- WHEN a user with permission submits valid asset updates
- THEN the system saves the changes
- AND the system updates the last-modified timestamp

#### Scenario: Update a non-existent asset
- WHEN a user submits an update request for an asset code that does not exist
- THEN the system MUST return a not-found error

### Requirement: Asset Search and Filtering
The system SHALL allow users to search and filter assets using basic criteria.

#### Scenario: Search by asset code or asset name
- WHEN a user enters a keyword matching an asset code or asset name
- THEN the system returns matching assets

#### Scenario: Filter by asset status or asset type
- WHEN a user applies a filter by asset status or asset type
- THEN the system returns only the assets that match the selected criteria

### Requirement: Asset Retirement or Disposal
The system MUST support marking an asset as retired or disposed without removing its historical record.

#### Scenario: Dispose an asset
- WHEN a user with permission disposes or retires an asset
- THEN the system updates the asset status to retired or disposed
- AND the system preserves the asset record for historical lookup
