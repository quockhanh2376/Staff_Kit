export type BorrowReviewRequestType = "borrow" | "return"

export function normalizeBorrowReviewRequestType(
  requestType?: string | null,
): BorrowReviewRequestType {
  return requestType === "return" ? "return" : "borrow"
}

export function buildBorrowReviewHeading() {
  return "Borrow / Return Review"
}

export function buildBorrowReviewHeaderDescription() {
  return "Employees scan the fixed LAN QR on their phone, submit a pending borrow or return request, then IT reviews the exact asset items here."
}

export function buildBorrowReviewEmptyQueueMessage() {
  return "No pending requests yet. Scan the QR on a phone to create the first borrow or return request."
}

export function getBorrowReviewApproveActionLabel(
  requestType: string | null | undefined,
  isApproving: boolean,
) {
  const typeLabel = normalizeBorrowReviewRequestType(requestType) === "return" ? "Return" : "Borrow"
  return isApproving ? `Approving ${typeLabel}...` : `Approve ${typeLabel}`
}

export function getBorrowReviewRejectActionLabel(
  requestType: string | null | undefined,
  isRejecting: boolean,
) {
  const typeLabel = normalizeBorrowReviewRequestType(requestType) === "return" ? "Return" : "Borrow"
  return isRejecting ? `Rejecting ${typeLabel}...` : `Reject ${typeLabel}`
}

export function buildBorrowReviewRejectPlaceholder(
  requestType: string | null | undefined,
) {
  return normalizeBorrowReviewRequestType(requestType) === "return"
    ? "Add a rejection note when the employee selected the wrong asset code or item they are returning."
    : "Add a rejection note when the employee selected the wrong asset type or code for borrowing."
}

export function buildBorrowReviewApproveSuccessMessage(
  requestType: string | null | undefined,
  requestKey: string,
) {
  const typeLabel = normalizeBorrowReviewRequestType(requestType)
  return `Approved ${typeLabel} request ${requestKey}. Stock and loan records were updated.`
}

export function buildBorrowReviewRejectSuccessMessage(
  requestType: string | null | undefined,
  requestKey: string,
) {
  const typeLabel = normalizeBorrowReviewRequestType(requestType)
  return `Rejected ${typeLabel} request ${requestKey}. The employee must resubmit with the correct asset item.`
}
