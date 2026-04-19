## MODIFIED Requirements

### Requirement: Asset display name from seed
When assets are seeded from the EE List (`employee_asset_seed`), the `display_name` SHALL be derived by stripping the `"VN"` prefix from the `asset_code` (case-insensitive). The `display_name` SHALL NOT be set equal to the computer name.

#### Scenario: Laptop asset seeded from EE list
- **WHEN** an employee with computer name source `VNLAP293` is seeded
- **THEN** the created asset has `display_name = "LAP293"` and `asset_code = "VNLAP293"`

#### Scenario: MacPro asset seeded from EE list
- **WHEN** an employee with computer name source `VNMACPRO010` is seeded
- **THEN** the created asset has `display_name = "MACPRO010"` and `asset_code = "VNMACPRO010"`

#### Scenario: Asset code without VN prefix seeded
- **WHEN** an asset code does not start with `"VN"` (case-insensitive)
- **THEN** the `display_name` equals the `asset_code` unchanged

#### Scenario: Dedup check uses asset_code only
- **WHEN** the seed process checks whether an asset already exists
- **THEN** the check is performed on `asset_code` only (not `computer_name`, which no longer exists in the assets table)
