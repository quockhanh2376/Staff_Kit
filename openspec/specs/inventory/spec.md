# Inventory Operations Specification

## Purpose
Define how the system manages inventory-specific operations such as stock intake, storage transfers, inventory verification, and stock-oriented history outside the employee receive and return approval flows.

This is a secondary supporting capability for stock-room operations.
It is not the source of truth for employee handover, employee return, approval gating, or employee-to-asset assignment rules.

Employee assignment, employee return, and approval-driven stock mutation for QR workflows are defined in:
- `employees`
- `receive-flow`
- `return-flow`
- `approval-workflow`

## Requirements

### Requirement: Asset Check-In
The system MUST record asset check-in transactions and associate the asset with an initial warehouse or storage unit.

#### Scenario: Check in an asset successfully
- WHEN an authorized user submits a valid asset check-in transaction
- THEN the system records the check-in transaction
- AND the system updates the current warehouse or storage location of the asset

### Requirement: Asset Transfer
The system MUST support transferring an asset between warehouses, storage units, departments, or managing locations when the transfer is an inventory operation rather than an employee QR receive or return workflow.

#### Scenario: Transfer an asset between storage locations
- WHEN an authorized user submits a valid transfer request between warehouses or storage locations
- THEN the system updates the receiving storage location for the asset
- AND the system stores the transfer history

#### Scenario: Reject an invalid transfer
- WHEN a transfer request is missing the source, destination, or transfer reason
- THEN the system MUST reject the transaction

### Requirement: Inventory Verification
The system SHALL allow users to record inventory verification results and compare them against current system records.

#### Scenario: Record a matching inventory result
- WHEN a user verifies that an asset exists in the expected location and with the expected holder
- THEN the system stores a successful inventory verification result

#### Scenario: Record an inventory discrepancy
- WHEN the inventory result differs from the current record in holder, location, or status
- THEN the system MUST record the discrepancy
- AND the system allows the discrepancy to be tracked for follow-up

### Requirement: Transaction History
The system MUST store the history of inventory-specific check-in, transfer, and inventory verification transactions for each asset.

#### Scenario: View transaction history
- WHEN a user with permission views the history of an asset
- THEN the system displays the transactions in chronological order

### Requirement: Existing Asset Constraint
The system MUST only allow inventory transactions to be performed on assets that exist in the system.

#### Scenario: Reject a transaction for a non-existent asset
- WHEN a user submits an inventory transaction for an asset code that does not exist
- THEN the system MUST reject the transaction
- AND the system returns an asset not found error

