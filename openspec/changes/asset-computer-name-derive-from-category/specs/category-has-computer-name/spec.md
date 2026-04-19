## ADDED Requirements

### Requirement: Category has-computer-name flag
Each `asset_category` record SHALL carry a `has_computer_name` boolean flag (stored as `INTEGER 0/1`) that declares whether assets in that category derive a network computer name.

#### Scenario: Laptop category has flag enabled
- **WHEN** the `laptop` category is seeded or migrated
- **THEN** `asset_categories.has_computer_name = 1` for that row

#### Scenario: Monitor category has flag disabled
- **WHEN** the `monitor` category is seeded or migrated
- **THEN** `asset_categories.has_computer_name = 0` for that row

#### Scenario: Computer name derived for flagged category
- **WHEN** an asset belongs to a category with `has_computer_name = 1`
- **THEN** the computer name is derived as `"ASW" + UPPER(asset_code)` at query time (never stored in the `assets` table)

#### Scenario: Computer name is null for non-flagged category
- **WHEN** an asset belongs to a category with `has_computer_name = 0`
- **THEN** the computer name field in the API response is `null`

#### Scenario: EE list shows only computer-name-eligible loans
- **WHEN** an employee has active loans containing both a laptop and a monitor
- **THEN** the Employee List Computer Name column shows only the laptop-derived name (not the monitor)

#### Scenario: New category defaults to no computer name
- **WHEN** a new asset category is created without specifying `has_computer_name`
- **THEN** the flag defaults to `0`
