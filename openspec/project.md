# AssetDesk-Pro Project Context

## Overview
AssetDesk-Pro is an internal company asset management web application.

The system is used by the IT team to manage:
- employee records imported from HR
- company IT assets
- asset assignment to employees
- asset return from employees
- asset tracking throughout the employee lifecycle

The app must work as a responsive web application on both:
- laptop browsers
- mobile phone browsers

Primary usage:
- IT team uses the management app on laptop and mobile
- employees use their personal phones only for QR-based receive and return flows

Employees are not normal authenticated management users in the system.
Employees interact with the system through QR-code-based forms only.

---

## Business Goal
The main goal of the project is to ensure that the IT team can always know:
- which employee is currently holding which asset
- which assets are available in stock
- which assets were returned
- which assets are pending review
- which actions were approved by IT

The system must keep asset stock and employee-to-asset mapping accurate.

The system must avoid:
- invalid asset codes
- duplicate assignments
- duplicate returns
- wrong employee-to-asset linkage
- direct stock corruption caused by user input mistakes

---

## Main Actors

### Admin
Admin is the main operational role used by the IT team.

Admin can:
- log in to the app
- upload employee Excel files from HR
- manage employee records
- manage asset records
- preload and maintain asset codes
- trigger QR receive sessions
- trigger QR return sessions
- review submitted receive requests
- review submitted return requests
- approve or reject requests
- track employee asset history
- track asset assignment and return history

### Super Admin
Super Admin has full access.

Super Admin can:
- do everything Admin can do
- manage admin accounts
- manage role access
- manage system settings and global configuration
- access all records and history

### Employee
Employees do not log in to the management application as standard users.

Employees only interact with the system through QR flows:
- receive asset flow
- return asset flow

Employees use their personal phones to scan QR codes shown by IT on the app screen.

---

## Product Scope

### Included in current scope
- Admin and Super Admin authentication
- employee import from HR Excel
- employee list and detail views
- asset creation and management
- asset status tracking
- employee-to-asset tracking
- QR receive workflow
- QR return workflow
- multi-asset receive and return in one session
- pending review workflow
- IT approval before official stock changes
- assignment history
- return history
- basic audit trail

### Out of current scope unless explicitly added later
- employee self-service portal
- employee login account
- digital signature workflow
- image upload for returned device condition
- advanced repair/maintenance flow
- approval chains with multiple approvers
- external system integrations unless specified

---

## Platform and UX Assumptions

### Platform
This is a responsive web app, not a native mobile app.

### Expected device usage
- IT team will mostly use laptop browsers
- employees will mostly use mobile browsers after scanning a QR code

### UX implications
The following pages must be mobile-friendly:
- QR receive form
- QR return form
- confirmation or result page after submission

The following pages are primarily desktop-oriented:
- employee management
- asset management
- request review and approval
- admin dashboards

---

## Employee Data Model Assumptions

Employee data originates from HR.

### Source of truth for employee import
HR provides an Excel file.
The system imports employee records from that file.

### Employee unique identifier
Each employee has a unique Staff ID.

Example:
- `ASWVN083`

Staff ID is the primary employee identifier in the business domain.

### Expected employee fields
Typical fields may include:
- staff ID
- full name
- department
- position
- email
- phone number
- join date
- employment status

The final imported schema must match the actual HR Excel template.

### Employee validation rule
When an employee submits a receive or return form, the system should validate the Staff ID against imported HR data.

If the business confirms it later, employee full name may also be matched against the imported record.

---

## Asset Data Model Assumptions

The IT team manages all assets that may be assigned to employees.

### Common asset types
- laptop
- laptop charger
- headset
- USB extension
- monitor
- desktop
- workstation

### Asset identification
Each trackable asset should have a unique asset code already preloaded by IT before it is used in QR workflows.

Examples:
- `ASWVNLAP400`
- `ASWVNLAP401`
- `ASWVNLAP402`
- `ASWVNWKS300`

### Asset preload rule
No asset should be received or returned through QR flows unless it already exists in the system.

This is a strict business rule.

### Reason
The app must update stock correctly by adding or subtracting only known asset records.
Unknown asset codes must never affect stock.

### Asset fields
At minimum, an asset should support:
- asset code
- asset type
- status
- current holder
- assignment history
- return history

Optional fields may be added later:
- serial number
- model
- location
- procurement data
- condition
- notes

---

## Asset Tracking Model

The system is intended to track which employee currently holds which assets.

This includes:
- current employees
- new employees joining the company
- employees leaving the company

A single employee may hold multiple assets at the same time.

Typical assignment bundle may include:
- 1 laptop
- 1 charger
- 1 headset
- 1 USB extension
- up to 4 monitors

The system must support multi-asset receive and multi-asset return in a single session.

---

## Core Business Principle

Employee submission alone must not directly change official stock or assignment records.

The system must use a staged workflow:

1. employee submits receive or return data
2. system creates a pending request
3. IT reviews the request
4. IT approves or rejects the request
5. only approved requests update official stock and assignment data

This approval step is required to reduce operational mistakes.

---

## Receive Workflow

### Business purpose
Used when a new employee joins the company and receives one or more prepared assets from IT.

### High-level flow
1. IT logs in
2. IT clicks `Scan Receive`
3. system displays a QR code
4. employee scans QR code using personal phone
5. mobile receive form opens
6. employee enters Staff ID and identifying information
7. employee enters or searches asset codes
8. system validates submitted asset codes
9. system creates a pending receive request
10. IT reviews the request
11. if approved, system assigns assets to employee and updates official stock/state

### Receive input rule
Asset code input is not blind free text.

Expected behavior:
- employee types an asset code
- system searches existing asset records
- system validates the asset code against preloaded data
- only valid assets may be submitted successfully

### Receive approval rule
After employee submission:
- request status becomes pending review
- assignment is not yet final
- stock is not yet officially changed

Only after IT approval:
- assignment is finalized
- asset holder is updated
- asset status is updated
- stock or availability is updated officially

---

## Return Workflow

### Business purpose
Used when an employee leaves the company or returns one or more assets to IT.

### High-level flow
1. IT logs in
2. IT clicks `Scan Return`
3. system displays a QR code
4. employee scans QR code using personal phone
5. mobile return form opens
6. employee enters Staff ID and identifying information
7. employee enters or searches returned asset codes
8. system validates submitted asset codes
9. system creates a pending return request
10. IT reviews the request
11. if approved, system closes assignment and returns asset(s) to stock

### Return validation rule
For a return request, the system should validate:
- asset exists
- asset is currently assigned
- asset is eligible for return
- asset matches the expected employee if that rule is enforced

### Return approval rule
After employee submission:
- request status becomes pending review
- asset is not yet returned to stock officially

Only after IT approval:
- active assignment is closed
- asset status becomes `In Stock`
- stock is updated officially

If future business rules require stricter handling, this can later evolve into:
- pending inspection
- maintenance required
- damaged return handling

---

## Request and Approval Model

### Main concept
Receive and return forms create requests, not immediate final transactions.

### Request lifecycle
Suggested request states:
- Draft Session
- Submitted
- Pending IT Review
- Approved
- Rejected

### Approval requirement
All receive and return requests must be reviewed by IT before affecting official data.

### Approval outcomes
If approved:
- system applies the business effect

If rejected:
- system keeps stock and assignment unchanged
- request remains stored for audit/history

---

## Stock Update Rules

### Important rule
Official stock changes happen only on approval, not on employee submission.

### Receive
On submit:
- create pending receive request

On approval:
- finalize assignment
- reduce available stock or move asset out of available pool
- set asset status to `Assigned`

### Return
On submit:
- create pending return request

On approval:
- close assignment
- add asset back into stock
- set asset status to `In Stock`

---

## Validation Rules

### Employee validation
- Staff ID must exist in imported HR data
- employee information should be validated against existing records where possible

### Asset validation
- asset code must already exist in the system
- asset code must follow company naming conventions where applicable
- asset must be valid for the requested action

### Receive validation
- asset cannot already be assigned if exclusive assignment is required
- asset cannot be duplicated in the same submission
- asset must be eligible for receive assignment

### Return validation
- asset must currently be assigned
- asset cannot be returned twice
- returned asset should match the current assignment if enforced

---

## Status Conventions

### Asset statuses
Current suggested statuses:
- In Stock
- Ready for Assignment
- Assigned
- Pending Receive Review
- Pending Return Review
- In Maintenance
- Retired

These names may be refined later, but the staged review concept must remain.

### Request statuses
Suggested request statuses:
- Draft
- Submitted
- Pending IT Review
- Approved
- Rejected

---

## Module Boundaries

### Authentication and Roles
Responsible for:
- login
- admin access
- super admin access
- role checks

### Employee Management
Responsible for:
- HR Excel upload
- employee import
- employee list
- employee detail pages

### Asset Management
Responsible for:
- creating asset records
- editing asset records
- asset search
- asset preload
- status management
- stock visibility
- assignment linkage

### QR Receive
Responsible for:
- creating receive sessions
- generating QR codes
- mobile receive form
- asset search and validation
- pending receive requests

### QR Return
Responsible for:
- creating return sessions
- generating QR codes
- mobile return form
- asset search and validation
- pending return requests

### Review and Approval
Responsible for:
- pending request queue
- request detail review
- approve or reject action
- applying official business updates

### Inventory Operations
Responsible for:
- asset stock intake or check-in
- warehouse or storage transfer
- inventory verification and discrepancy follow-up

This capability is secondary in the current product scope.
It must not redefine employee receive, employee return, request approval, or employee-to-asset assignment rules.

### History and Audit
Responsible for:
- assignment history
- return history
- review actions
- approval/rejection traceability

---

## Naming and Writing Conventions

### Domain terms
Use these terms consistently in specs, changes, UI labels, and code where practical:
- Employee
- Staff ID
- Asset
- Asset Code
- Receive
- Return
- Assignment
- In Stock
- Assigned
- Pending Review
- Approval
- Rejection

### Role terms
Use:
- Admin
- Super Admin

Do not introduce a Standard User role unless business requirements change later.

### UI action labels
Preferred labels:
- `Scan Receive`
- `Scan Return`

If UI wording changes later, update all references consistently.

---

## OpenSpec Conventions for This Project

### Role of project.md
This file provides stable project-wide context, business rules, vocabulary, and implementation boundaries.

### Role of specs
`openspec/specs/` should describe source-of-truth behavior for each capability.

Recommended capabilities include:
- employees
- assets
- users-roles
- receive-flow
- return-flow
- approval-workflow
- audit-log

The `inventory` capability is optional and secondary for this project.
If it exists, it is limited to inventory-only operations such as stock intake, storage transfer, and verification.
`inventory` must not be treated as the source of truth for employee receive, employee return, approval gating, or employee-to-asset assignment behavior.

### Role of changes
`openspec/changes/` should be used for new features or modifications before implementation.

Each change should contain, as needed:
- `proposal.md`
- `design.md`
- `tasks.md`
- spec deltas under `specs/<capability>/spec.md`

---

## Current Open Questions

These questions still need final answers and should not be silently guessed during implementation:

1. Should employee full name auto-fill after Staff ID input?
2. Should the employee be allowed to edit the full name manually?
3. Should the asset search field show suggestions as the employee types?
4. Should all accessories have unique asset codes, or should some items be tracked only by quantity?
5. Should monitors be individually tracked one by one?
6. Should an approval note be required when IT approves or rejects?
7. Should QR sessions expire after a short period?
8. Should one QR session be limited to one employee submission?
9. Should rejected requests be editable and resubmittable?
10. Should damaged returns go directly to `In Stock`, or later move to a maintenance flow?

---

## Non-Goals for Initial Implementation

Avoid assuming these are in scope unless explicitly requested:
- deep maintenance workflows
- procurement workflows
- accounting depreciation
- vendor management
- e-signature integration
- employee self-service dashboard
- complex multi-step approval chains
- SSO integrations
- native mobile applications

---

## Guidance for AI Assistants
Before proposing or implementing changes:
1. read this `project.md` first
2. read relevant source-of-truth specs in `openspec/specs/`
3. do not invent business rules not stated here or in specs
4. prefer validation and staged approval over direct stock mutation
5. preserve clear auditability for assignment and return operations
6. treat asset-code validation as a critical business rule
7. when a workflow concerns employee handover or employee return, prefer `receive-flow`, `return-flow`, and `approval-workflow` over `inventory`
8. ask for clarification when a workflow could affect stock accuracy or employee-to-asset mapping
9. maintain consistent terminology across all outputs
