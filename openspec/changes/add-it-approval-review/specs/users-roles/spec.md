# Delta for users-roles

## ADDED Requirements

### Requirement: IT Approval Permissions
The system MUST allow only authorized management roles to approve or reject pending receive and return requests.

#### Scenario: Admin reviews a pending request
- WHEN an Admin user attempts to approve or reject a pending request
- THEN the system allows the action if the Admin role includes approval permission

#### Scenario: Super Admin reviews a pending request
- WHEN a Super Admin attempts to approve or reject a pending request
- THEN the system allows the action

#### Scenario: Non-authorized user attempts review
- WHEN a user without approval permission attempts to approve or reject a pending request
- THEN the system MUST reject the action
