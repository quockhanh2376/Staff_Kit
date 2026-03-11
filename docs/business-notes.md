# AssetDesk-Pro Business Notes

## 1. Project Overview
AssetDesk-Pro is an internal asset management web application that works on both laptop and mobile web browsers.

The app is mainly used by the IT team to:
- Manage employee asset assignments
- Track which employee is currently holding which asset
- Handle onboarding asset receiving
- Handle offboarding or device return
- Keep stock data accurate through IT review and approval

Employees do not use the app as normal system users.
Employees only interact with the system through QR-based mobile web forms for receiving and returning assets.

---

## 2. User Roles

There are only 2 roles in the system:

### 2.1 Admin
Admin is the main IT team operational role.

Admin can:
- Log in to the app
- Import employee data from HR Excel
- Manage assets
- Track employee-to-asset assignments
- Trigger QR receive sessions
- Trigger QR return sessions
- Review submitted receive and return forms
- Approve or reject submitted forms

### 2.2 Super Admin
Super Admin has full access.

Super Admin can:
- Do everything Admin can do
- Manage admin accounts
- Manage all system settings
- Access all data and records

---

## 3. Platform Scope

The application is a responsive web app that must work on:
- Laptop browser
- Mobile browser

Expected usage:
- IT team mainly uses laptop browser
- Employees use personal phones to scan QR codes and complete forms

The QR pages must be mobile-friendly.

---

## 4. Employee Data

Employee data is provided by HR through Excel import.

### Employee source
- HR prepares an Excel file
- Admin uploads the file
- The system imports employee data

### Unique employee identifier
Each employee has a unique Staff ID.

Example:
- `ASWVN083`

This Staff ID is the main unique employee identifier.

### Employee data fields
Possible fields include:
- Staff ID
- Full name
- Department
- Position
- Email
- Phone number
- Join date
- Employment status

Final import fields should match the HR template.

---

## 5. Asset Data

The IT team manages all assets prepared and assigned to employees.

### Example asset types
- Laptop
- Laptop charger
- Headset
- USB extension
- Monitor

### Multi-asset assignment reality
A single employee may receive multiple assets in one handover session.

Typical examples:
- 1 laptop
- 1 charger
- 1 headset
- 1 USB extension
- Up to 4 monitors

### Asset code format
Each asset must have a unique asset code already preloaded by IT.

Examples:
- `ASWVNLAP400`
- `ASWVNLAP401`
- `ASWVNLAP402`
- `ASWVNWKS300`

### Asset preload rule
IT must add asset records into the system before they can be received or returned through QR workflows.

No asset should affect stock unless it already exists in the system.

### Asset fields
Each asset should contain:
- Asset code
- Asset type
- Status
- Current holder
- Assignment history
- Return history

### Suggested asset statuses
- In Stock
- Ready for Assignment
- Assigned
- Pending Receive Review
- Pending Return Review
- In Maintenance
- Retired

---

## 6. Main Business Principle

The system must not directly change stock or assignment when an employee submits a QR form.

Instead, employee submission should first create a temporary pending record.
The IT team must review and approve that pending record before the system makes final stock updates.

This is important to avoid:
- Wrong employee information
- Wrong asset codes
- Duplicate assignment
- Invalid return
- Incorrect stock plus/minus
- Missing accessories
- Unchecked returned devices

---

## 7. Receive Flow

### IT-side flow
1. IT logs into the app
2. IT clicks `Scan Receive`
3. The app displays a QR code
4. The employee scans the QR code using a personal phone
5. The mobile web receiving form opens

### Employee-side receive form
The employee enters:
- Staff ID
- Full name

The employee also enters asset information.

### Asset input behavior
Asset code input should work like this:
- The employee types the asset code
- The system searches matching preloaded assets
- The system validates the code against existing asset records
- Only valid existing assets can be submitted

This means the asset field is not a simple free-text field with no validation.
It is a search-and-validate input.

### Multi-asset receive
One receive session may include multiple assets.

Example:
- Laptop
- Charger
- Headset
- USB extension
- Monitor(s)

The system should support one employee receiving multiple assets in the same session.

### Receive submission result
After the employee submits the form:
- The system creates a pending receive record
- The system does not yet finalize assignment
- The selected assets are not yet permanently deducted from stock
- The request waits for IT review

### IT review for receive
IT reviews:
- Staff ID
- Employee name
- Asset codes
- Whether assets are valid and available
- Whether the asset bundle matches the actual handover

After review:
- If approved, the system creates the assignment records
- Asset status changes to Assigned
- Stock is updated officially
- Asset holder is linked to the employee

If rejected:
- No stock update is applied
- No final assignment is created
- The pending request is marked rejected

---

## 8. Return Flow

### IT-side flow
1. IT logs into the app
2. IT clicks `Scan Return`
3. The app displays a QR code
4. The employee scans the QR code using a personal phone
5. The mobile web return form opens

### Employee-side return form
The employee enters:
- Staff ID
- Full name
- Asset code(s) being returned

### Return asset input behavior
The employee types asset code, then the system searches and validates it against existing asset records.

The system should also validate:
- The asset exists
- The asset is currently assigned
- The asset is assigned to the correct employee if that rule is required

### Multi-asset return
One return session may include multiple assets.

Example:
- Laptop
- Charger
- Headset
- USB extension
- Monitor(s)

### Return submission result
After employee submission:
- The system creates a pending return record
- The asset is not yet officially added back into stock
- The request waits for IT review

### IT review for return
IT checks:
- Correct employee
- Correct asset codes
- Whether all returned items are physically present
- Whether the returned assets match the real devices
- Whether there is any missing or damaged item

After review:
- If approved, the system closes the active assignment
- The asset status changes to In Stock
- The asset returns to stock officially

If rejected:
- The system keeps the current assignment unchanged
- The pending return record is marked rejected or needs correction

---

## 9. Approval Workflow

### Main rule
Employee submission alone is not enough to change official system data.

All receive and return actions must go through IT approval.

### Approval states
Suggested states:
- Draft Session
- Submitted
- Pending IT Review
- Approved
- Rejected

### Approval outcome
Only after IT approval should the system:
- Update stock
- Update asset status
- Create or close assignment
- Save final business result

---

## 10. Validation Rules

### Employee validation
- Staff ID must exist in imported HR data
- Employee name should match the Staff ID if matching is required

### Asset validation
- Asset code must exist in the system
- Asset code must match company format
- Asset must already be preloaded by IT
- Asset must be in a valid state for the current action

### Receive validation
- Asset cannot already be assigned to another employee
- Asset cannot be duplicated in the same receive form
- Only valid available assets may be approved

### Return validation
- Asset must currently be assigned
- Asset cannot be returned twice
- Asset should match the employee record if required

---

## 11. Stock Update Logic

### Important rule
Do not directly update official stock on employee submit.

### Receive
- On employee submit: create pending receive request
- On IT approve: decrease available stock, mark asset as Assigned, link asset to employee

### Return
- On employee submit: create pending return request
- On IT approve: increase stock, mark asset as In Stock, remove or close assignment

This staged approach is safer and reduces data mistakes.

---

## 12. Main Modules

### 12.1 Authentication and Roles
- Login
- Admin role
- Super Admin role

### 12.2 Employee Management
- HR Excel upload
- Employee import
- Employee list
- Employee details

### 12.3 Asset Management
- Create asset
- Edit asset
- Preload asset codes
- Search asset
- Track stock
- Track status

### 12.4 QR Receive Module
- Scan Receive button
- QR generation
- Mobile receive form
- Search and validate asset code input
- Multi-asset receive submission
- Pending receive records
- IT approval or rejection

### 12.5 QR Return Module
- Scan Return button
- QR generation
- Mobile return form
- Search and validate asset code input
- Multi-asset return submission
- Pending return records
- IT approval or rejection

### 12.6 Assignment Tracking
- View current assets by employee
- View current holder by asset
- View assignment history
- View return history

### 12.7 Audit and Review
- Who created QR session
- Who submitted receive or return form
- Who approved or rejected
- Timestamps for all actions

---

## 13. Recommended Status Model

### Asset statuses
- In Stock
- Ready for Assignment
- Assigned
- Pending Receive Review
- Pending Return Review
- In Maintenance
- Retired

### Request statuses
- Draft
- Submitted
- Pending IT Review
- Approved
- Rejected

---

## 14. Open Questions

Items still needing final confirmation:
1. Should employee full name auto-fill after Staff ID is entered?
2. Should the system force Staff ID to exist before form submission?
3. Should asset search show only assets allowed for the current action?
4. Should chargers, headsets, and USB extensions each have unique asset codes, or only quantity-based stock?
5. Should monitor assets be individually tracked one by one?
6. Should damaged returned devices go to In Stock directly, or be diverted to Maintenance?
7. Should IT approval require a note before approving or rejecting?
8. Should the mobile form allow editing after submit?
9. Should the QR session expire after a few minutes?
10. Should one QR session be used by only one employee at a time?

---

## 15. Recommended Version 1 Scope

Version 1 should include:
- Admin and Super Admin roles
- Employee import from HR Excel
- Preloaded asset management
- Employee-to-asset tracking
- Multi-asset receive flow
- Multi-asset return flow
- Search-and-validate asset input
- Pending receive and pending return review
- IT approval before official stock update
- Basic audit log and history
- Mobile-friendly QR forms