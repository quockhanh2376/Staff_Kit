# Tasks: add-qr-receive

1. Review `project.md` and the source-of-truth specs for receive-flow, approval-workflow, assets, and audit-log.
2. Design the receive session model with session type, token, status, and expiry fields.
3. Implement the `Scan Receive` action for Admin and Super Admin users.
4. Generate a QR code linked to an active receive session.
5. Implement the mobile receive form opened from the QR session.
6. Add employee identity input fields required for receive requests.
7. Implement typed asset-code search against preloaded asset records.
8. Validate asset eligibility for receive submission.
9. Support multi-asset submission in a single receive request.
10. Save submitted receive data as a pending review request.
11. Add IT review UI for approving or rejecting pending receive requests.
12. On approval, create final assignments and update asset status to Assigned.
13. On rejection, keep stock and assignment unchanged.
14. Record audit entries for session creation, submission, approval, and rejection.
15. Add tests for receive session creation, asset validation, duplicate prevention, approval effects, and rejection behavior.
