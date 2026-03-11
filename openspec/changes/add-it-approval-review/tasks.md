# Tasks: add-it-approval-review

1. Review `project.md` and the source-of-truth specs for approval-workflow, users-roles, receive-flow, return-flow, and audit-log.
2. Define the pending review queue model for both receive and return requests.
3. Implement the IT review queue UI with request type, status, and summary data.
4. Implement request detail views showing employee information, asset codes, and request metadata.
5. Restrict approve and reject actions to Admin and Super Admin users with server-side checks.
6. Implement approve and reject actions for pending receive requests.
7. Implement approve and reject actions for pending return requests.
8. Keep official stock and assignment data unchanged until approval succeeds.
9. Record reviewer identity, review outcome, and review timestamp for each decision.
10. Record audit entries for approval, rejection, and denied review attempts.
11. Add tests for queue visibility, permission checks, approval effects, rejection effects, and audit behavior.
