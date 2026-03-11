# Delta for assets

## ADDED Requirements

### Requirement: Asset Eligibility for Return Search
The system SHALL expose only valid candidate assets for return search results.

#### Scenario: Show only searchable assets for return
- WHEN an employee searches for an asset code in the return form
- THEN the system returns only matching asset records that exist in the system and are relevant to return

### Requirement: Asset Pending Return Visibility
The system SHALL indicate that an asset is already part of a pending return request to help IT review conflicts.

#### Scenario: Display asset involvement in a pending return request
- WHEN an asset is already included in a pending return request
- THEN the system SHOULD expose that pending relationship to authorized IT users during review
- AND the system SHOULD prevent approval of conflicting return requests for the same asset
