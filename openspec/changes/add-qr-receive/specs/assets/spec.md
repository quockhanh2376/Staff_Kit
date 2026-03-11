# Delta for assets

## ADDED Requirements

### Requirement: Asset Eligibility for Receive Search
The system SHALL expose only valid candidate assets for receive search results.

#### Scenario: Show only searchable assets for receive
- WHEN an employee searches for an asset code in the receive form
- THEN the system returns only matching asset records that exist in the system

### Requirement: Asset Reservation Visibility During Pending Receive
The system SHALL indicate that an asset is part of a pending receive request to help IT review conflicts.

#### Scenario: Display asset involvement in a pending receive request
- WHEN an asset is already included in a pending receive request
- THEN the system SHOULD expose that pending relationship to authorized IT users during review
- AND the system SHOULD prevent approval of conflicting receive requests that would assign the same asset to multiple employees
