## ADDED Requirements

### Requirement: Credentials-based authentication baseline
The system MUST support credentials-based sign-in for active Staff Kit accounts using the agreed web authentication stack.

#### Scenario: Active account signs in successfully
- **WHEN** a user submits valid credentials for an active account
- **THEN** the system creates an authenticated session and grants access based on the account role

#### Scenario: Invalid or inactive account attempts sign-in
- **WHEN** a user submits invalid credentials or an inactive account attempts sign-in
- **THEN** the system denies access and returns a safe authentication error without exposing sensitive details

### Requirement: Role-protected routes and APIs
The system MUST enforce role-aware access control for protected pages and backend endpoints so users can access only the capabilities allowed by their assigned role.

#### Scenario: Unauthorized user accesses an admin-only route
- **WHEN** a signed-in user without the required role requests an admin-only page or API
- **THEN** the system denies access according to the route or API contract

#### Scenario: Authorized user accesses a protected route
- **WHEN** a signed-in user with the required role requests a protected page or API
- **THEN** the system allows access and evaluates the request within the authenticated context

### Requirement: Bootstrap administrator access
The system MUST provide a deterministic bootstrap administrator account path for first-time environment setup so the team can securely access and configure the web app after deployment.

#### Scenario: Fresh environment is initialized
- **WHEN** the application stack is initialized for a new environment
- **THEN** the system provisions or documents the bootstrap admin path needed to sign in and continue setup

