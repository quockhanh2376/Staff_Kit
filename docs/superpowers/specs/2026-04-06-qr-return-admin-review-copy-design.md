# QR Return Admin Review Copy Design

## Goal
Make the desktop admin review screen describe the current Borrow/Return behavior accurately, without changing the underlying approval data model.

## Current Problem
- The queue already carries both borrow and return requests.
- Approval logic already branches on `requestType`.
- The UI still frames the feature almost entirely as "Borrow Approval", which is misleading for IT reviewers and creates the impression that return requests are second-class or unsupported.

## Scope
- Update admin/review copy, labels, and success messages to reflect the selected request type.
- Keep the queue, DB model, and approval commands unchanged.
- Add a lightweight regression test that protects the key Borrow/Return copy semantics.

## Approach
1. Introduce small copy helpers in the borrow admin flow instead of hard-coding borrow-only text.
2. Make the review screen title and descriptions describe a shared Borrow/Return review queue.
3. Make the approve/reject button labels, placeholder text, and queue result messages adapt to the selected request type.
4. Protect the slice with a script-level regression test that checks for the expected semantics.

## Non-Goals
- No new backend commands.
- No queue filtering or workflow redesign.
- No audit schema changes in this slice.

## Validation
- `node --experimental-strip-types scripts/borrow-admin-review-copy.test.ts`
- `npm run check:quality`
