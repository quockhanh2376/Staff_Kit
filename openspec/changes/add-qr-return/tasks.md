# Tasks: add-qr-return

1. Review `project.md` and the source-of-truth specs for return-flow, approval-workflow, assets, and audit-log.
2. Design the return session model with session type, token, status, and expiry fields.
3. Implement the `Scan Return` action for Admin and Super Admin users.
4. Generate a QR code linked to an active return session.
5. Implement the mobile return form opened from the QR session.
6. Add employee identity input fields required for return requests.
7. Implement typed asset-code search against existing assigned asset records.
8. Validate asset eligibility for return submission.
9. Support multi-asset submission in a single return request.
10. Save submitted return data as a pending review request.
11. Add IT review UI for approving or rejecting pending return requests.
12. On approval, close active assignments and update asset status to In Stock.
13. On rejection, keep stock and assignment unchanged.
14. Record audit entries for session creation, submission, approval, and rejection.
15. Add tests for return session creation, asset validation, duplicate prevention, approval effects, and rejection behavior.
