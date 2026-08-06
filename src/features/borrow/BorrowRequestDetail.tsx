import { CheckCircle2, LoaderCircle, XCircle } from "lucide-react"
import type { BorrowState } from "./useBorrowState"
import {
  buildBorrowReviewRejectPlaceholder,
  getBorrowReviewApproveActionLabel,
  getBorrowReviewRejectActionLabel,
} from "./borrowReviewCopy"
import { RequestTypeBadge } from "./RequestTypeBadge"

type BorrowRequestDetailProps = {
  borrow: BorrowState
}

export function BorrowRequestDetail({ borrow }: BorrowRequestDetailProps) {
  const selectedRequestType = borrow.selectedRequest?.requestType

  return (
    <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div className="text-sm font-semibold text-[var(--text-primary)]">Request Detail</div>
          <p className="mt-1 text-xs text-[var(--text-secondary)]">
            Review the exact asset codes before approval. Reject when the employee selected the wrong
            item for this borrow or return request.
          </p>
        </div>
        {borrow.selectedRequest && (
          <div className="flex items-center gap-2">
            <RequestTypeBadge requestType={borrow.selectedRequest.requestType} size="md" />
            <div className="rounded-[999px] border border-[var(--border)] px-3 py-1 text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
              {borrow.selectedRequest.requestKey}
            </div>
          </div>
        )}
      </div>

      {borrow.queueMessage && (
        <div className="mt-4 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
          {borrow.queueMessage}
        </div>
      )}

      {!borrow.selectedRequest && !borrow.isLoadingDetail && (
        <div className="mt-4 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 px-4 py-5 text-sm text-[var(--text-secondary)]">
          Select a pending request from the queue to start review.
        </div>
      )}

      {borrow.isLoadingDetail && (
        <div className="mt-4 flex items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 px-4 py-5 text-sm text-[var(--text-secondary)]">
          <LoaderCircle className="animate-spin" size={15} />
          Loading request detail...
        </div>
      )}

      {borrow.selectedRequest && (
        <div className="mt-4 grid gap-4 lg:grid-cols-[minmax(0,1fr),320px]">
          <div className="space-y-4 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                  Staff ID
                </div>
                <div className="mt-1 text-sm font-semibold text-[var(--text-primary)]">
                  {borrow.selectedRequest.submittedEmployeeId}
                </div>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                  Full Name
                </div>
                <div className="mt-1 text-sm font-semibold text-[var(--text-primary)]">
                  {borrow.selectedRequest.submittedFullName}
                </div>
              </div>
            </div>

            <div>
              <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                Submitted At
              </div>
              <div className="mt-1 text-sm text-[var(--text-primary)]">
                {borrow.selectedRequest.submittedAt}
              </div>
            </div>

            <div>
              <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                Asset Items
              </div>
              <div className="mt-2 flex flex-wrap gap-2">
                {borrow.selectedRequest.assetCodes.map((assetCode) => (
                  <span
                    key={assetCode}
                    className="rounded-[999px] border border-[var(--primary)]/35 bg-[var(--primary)]/10 px-3 py-1 text-xs font-semibold text-[var(--text-primary)]"
                  >
                    {assetCode}
                  </span>
                ))}
              </div>
            </div>
          </div>

          <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/20 p-4">
            <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
              IT Review Decision
            </div>
            <textarea
              className="form-input mt-3 min-h-[160px] resize-y text-sm"
              value={borrow.reviewNote}
              onChange={(event) => borrow.setReviewNote(event.target.value)}
              placeholder={buildBorrowReviewRejectPlaceholder(selectedRequestType)}
              disabled={borrow.isApproving || borrow.isRejecting || borrow.isCancelling}
            />

            <div className="mt-4 flex flex-wrap gap-2">
              <button
                className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                onClick={() => void borrow.handleApproveRequest()}
                type="button"
                disabled={borrow.isApproving || borrow.isRejecting || borrow.isCancelling}
              >
                {borrow.isApproving ? (
                  <LoaderCircle className="animate-spin" size={15} />
                ) : (
                  <CheckCircle2 size={15} />
                )}
                {getBorrowReviewApproveActionLabel(selectedRequestType, borrow.isApproving)}
              </button>
              {borrow.selectedRequest.status === "pending" && (
                <button
                  className="inline-flex items-center gap-2 rounded-[8px] border border-amber-400/50 bg-amber-500/10 px-3 py-2 text-sm font-semibold text-amber-200 disabled:opacity-50"
                  onClick={() => void borrow.handleCancelRequest()}
                  type="button"
                  disabled={borrow.isApproving || borrow.isRejecting || borrow.isCancelling}
                >
                  {borrow.isCancelling ? <LoaderCircle className="animate-spin" size={15} /> : <XCircle size={15} />}
                  {borrow.isCancelling ? "Cancelling..." : "Cancel Request"}
                </button>
              )}
              <button
                className="inline-flex items-center gap-2 rounded-[8px] border border-red-400/50 bg-red-500/10 px-3 py-2 text-sm font-semibold text-red-200 disabled:opacity-50"
                onClick={() => void borrow.handleRejectRequest()}
                type="button"
                disabled={borrow.isApproving || borrow.isRejecting || borrow.isCancelling}
              >
                {borrow.isRejecting ? (
                  <LoaderCircle className="animate-spin" size={15} />
                ) : (
                  <XCircle size={15} />
                )}
                {getBorrowReviewRejectActionLabel(selectedRequestType, borrow.isRejecting)}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
