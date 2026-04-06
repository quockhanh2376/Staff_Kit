import assert from "node:assert/strict"

import {
  buildBorrowReviewApproveSuccessMessage,
  buildBorrowReviewEmptyQueueMessage,
  buildBorrowReviewHeaderDescription,
  buildBorrowReviewHeading,
  buildBorrowReviewRejectPlaceholder,
  buildBorrowReviewRejectSuccessMessage,
  getBorrowReviewApproveActionLabel,
  getBorrowReviewRejectActionLabel,
} from "../src/features/borrow/borrowReviewCopy.ts"

assert.equal(buildBorrowReviewHeading(), "Borrow / Return Review")
assert.equal(
  buildBorrowReviewHeaderDescription(),
  "Employees scan the fixed LAN QR on their phone, submit a pending borrow or return request, then IT reviews the exact asset items here.",
)
assert.equal(
  buildBorrowReviewEmptyQueueMessage(),
  "No pending requests yet. Scan the QR on a phone to create the first borrow or return request.",
)

assert.equal(getBorrowReviewApproveActionLabel("borrow", false), "Approve Borrow")
assert.equal(getBorrowReviewApproveActionLabel("borrow", true), "Approving Borrow...")
assert.equal(getBorrowReviewApproveActionLabel("return", false), "Approve Return")
assert.equal(getBorrowReviewApproveActionLabel("return", true), "Approving Return...")

assert.equal(getBorrowReviewRejectActionLabel("borrow", false), "Reject Borrow")
assert.equal(getBorrowReviewRejectActionLabel("borrow", true), "Rejecting Borrow...")
assert.equal(getBorrowReviewRejectActionLabel("return", false), "Reject Return")
assert.equal(getBorrowReviewRejectActionLabel("return", true), "Rejecting Return...")

assert.equal(
  buildBorrowReviewRejectPlaceholder("borrow"),
  "Add a rejection note when the employee selected the wrong asset type or code for borrowing.",
)
assert.equal(
  buildBorrowReviewRejectPlaceholder("return"),
  "Add a rejection note when the employee selected the wrong asset code or item they are returning.",
)

assert.equal(
  buildBorrowReviewApproveSuccessMessage("borrow", "BR-123"),
  "Approved borrow request BR-123. Stock and loan records were updated.",
)
assert.equal(
  buildBorrowReviewApproveSuccessMessage("return", "RT-123"),
  "Approved return request RT-123. Stock and loan records were updated.",
)

assert.equal(
  buildBorrowReviewRejectSuccessMessage("borrow", "BR-123"),
  "Rejected borrow request BR-123. The employee must resubmit with the correct asset item.",
)
assert.equal(
  buildBorrowReviewRejectSuccessMessage("return", "RT-123"),
  "Rejected return request RT-123. The employee must resubmit with the correct asset item.",
)

console.log("borrow-admin-review-copy tests passed")
