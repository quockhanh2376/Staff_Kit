import { LoaderCircle } from "lucide-react"
import type { BorrowState } from "./useBorrowState"
import { buildBorrowReviewEmptyQueueMessage } from "./borrowReviewCopy"
import { RequestTypeBadge } from "./RequestTypeBadge"

type BorrowPendingQueueProps = {
  borrow: BorrowState
}

export function BorrowPendingQueue({ borrow }: BorrowPendingQueueProps) {
  return (
    <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
      <div className="text-sm font-semibold text-[var(--text-primary)]">Pending Queue</div>
      <p className="mt-1 text-xs text-[var(--text-secondary)]">
        {borrow.pendingRequests.length} request(s) waiting for IT review.
      </p>

      <div className="mt-4 space-y-2">
        {borrow.isLoadingQueue && (
          <div className="flex items-center gap-2 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/20 px-3 py-3 text-sm text-[var(--text-secondary)]">
            <LoaderCircle className="animate-spin" size={15} />
            Loading borrow queue...
          </div>
        )}

        {!borrow.isLoadingQueue && borrow.pendingRequests.length === 0 && (
          <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/20 px-3 py-3 text-sm text-[var(--text-secondary)]">
            {buildBorrowReviewEmptyQueueMessage()}
          </div>
        )}

        {borrow.pendingRequests.map((request) => {
          const isSelected = borrow.selectedRequestId === request.id
          return (
            <button
              key={request.id}
              className={`w-full rounded-[10px] border px-3 py-3 text-left transition ${
                isSelected
                  ? "border-[var(--primary)]/55 bg-[var(--primary)]/10"
                  : "border-[var(--border)] bg-[var(--surface-hover)]/20 hover:border-[var(--primary)]/30"
              }`}
              onClick={() => void borrow.handleSelectRequest(request.id)}
              type="button"
            >
              <div className="flex items-center gap-2">
                <div className="text-sm font-semibold text-[var(--text-primary)]">
                  {request.submittedEmployeeId}
                </div>
                <RequestTypeBadge requestType={request.requestType} />
                <div className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                  {request.assetCodes.length} asset
                </div>
              </div>
              <div className="mt-1 text-xs text-[var(--text-secondary)]">{request.submittedFullName}</div>
              <div className="mt-2 text-[11px] text-[var(--text-secondary)]">
                {request.assetCodes.join(", ")}
              </div>
            </button>
          )
        })}
      </div>
    </div>
  )
}
