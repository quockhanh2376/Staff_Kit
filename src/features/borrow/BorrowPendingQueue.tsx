import { CheckCircle2, ChevronDown, ChevronRight, FileCheck2, LoaderCircle, PenLine, UserRound, XCircle } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import type { BorrowState } from "./useBorrowState"
import { buildBorrowReviewEmptyQueueMessage, buildBorrowReviewRejectPlaceholder } from "./borrowReviewCopy"
import { RequestTypeBadge } from "./RequestTypeBadge"

type BorrowPendingQueueProps = {
  borrow: BorrowState
}

function formatSubmittedTime(value: string) {
  const time = value.split("T")[1]?.replace(/Z$/, "")
  return time?.slice(0, 8) || value
}

function formatAssetSummary(assetCodes: string[]) {
  return assetCodes.length === 1 ? assetCodes[0] : `${assetCodes.length} assets`
}

export function BorrowPendingQueue({ borrow }: BorrowPendingQueueProps) {
  const rejectionNoteRef = useRef<HTMLTextAreaElement>(null)
  const [expandedRequestIds, setExpandedRequestIds] = useState<Set<number>>(
    () => (borrow.selectedRequestId === null ? new Set() : new Set([borrow.selectedRequestId])),
  )
  const needsRejectNote = borrow.queueMessage === "A rejection note is required."
  const selectedDetail = borrow.selectedRequest

  useEffect(() => {
    if (needsRejectNote) rejectionNoteRef.current?.focus()
  }, [needsRejectNote, selectedDetail?.id])

  const pendingIds = new Set(borrow.pendingRequests.map((request) => request.id))
  const visibleExpandedRequestIds = new Set([...expandedRequestIds].filter((id) => pendingIds.has(id)))

  return (
    <div
      data-testid="pending-requests-card"
      className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="text-sm font-semibold text-[var(--text-primary)]">
          Pending Requests ({borrow.pendingRequests.length})
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          <button
            className="inline-flex items-center gap-1.5 rounded-[8px] bg-[var(--primary)] px-2.5 py-1.5 text-xs font-semibold text-[#00131c] disabled:opacity-50"
            disabled={!borrow.selectedRequest || borrow.isApproving || borrow.isRejecting || borrow.isCancelling}
            onClick={() => void borrow.handleApproveRequest()}
            type="button"
          >
            {borrow.isApproving ? <LoaderCircle className="animate-spin" size={13} /> : <CheckCircle2 size={13} />}
            {borrow.isApproving ? "Approving..." : "Approve"}
          </button>
          <button
            className="inline-flex items-center gap-1.5 rounded-[8px] border border-amber-400/50 bg-amber-500/10 px-2.5 py-1.5 text-xs font-semibold text-amber-200 disabled:opacity-50"
            disabled={!borrow.selectedRequest || borrow.selectedRequest.status !== "pending" || borrow.isApproving || borrow.isRejecting || borrow.isCancelling}
            onClick={() => void borrow.handleCancelRequest()}
            type="button"
          >
            {borrow.isCancelling ? <LoaderCircle className="animate-spin" size={13} /> : <XCircle size={13} />}
            {borrow.isCancelling ? "Cancelling..." : "Cancel"}
          </button>
          <button
            className="inline-flex items-center gap-1.5 rounded-[8px] border border-red-400/50 bg-red-500/10 px-2.5 py-1.5 text-xs font-semibold text-red-200 disabled:opacity-50"
            disabled={!borrow.selectedRequest || borrow.isApproving || borrow.isRejecting || borrow.isCancelling}
            onClick={() => void borrow.handleRejectRequest()}
            type="button"
          >
            {borrow.isRejecting ? <LoaderCircle className="animate-spin" size={13} /> : <XCircle size={13} />}
            {borrow.isRejecting ? "Rejecting..." : "Reject"}
          </button>
        </div>
      </div>

      {borrow.queueMessage && !needsRejectNote && (
        <div className="mt-3 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
          {borrow.queueMessage}
        </div>
      )}

      <div data-testid="pending-request-grid" className="mt-3 grid grid-cols-1 gap-2 xl:grid-cols-2">
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
          const isExpanded = visibleExpandedRequestIds.has(request.id)
          const isSelected = borrow.selectedRequestId === request.id
          const detail = isSelected && selectedDetail?.id === request.id ? selectedDetail : request

          return (
            <div
              key={request.id}
              data-testid={`pending-request-row-${request.id}`}
              data-selected={isSelected}
              className={`overflow-hidden rounded-[10px] border transition ${
                isSelected
                  ? "border-[var(--primary)]/55 bg-[var(--primary)]/10"
                  : "border-[var(--border)] bg-[var(--surface-hover)]/20"
              }`}
            >
              <div className="flex items-center gap-1 px-2 py-2.5 hover:bg-[var(--surface-hover)]/30">
                <button
                  aria-expanded={isExpanded}
                  aria-label={`${isExpanded ? "Collapse" : "Expand"} request ${request.id}`}
                  className="shrink-0 rounded p-1 text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                  data-testid={`pending-request-toggle-${request.id}`}
                  onClick={() => {
                    setExpandedRequestIds((current) => {
                      const next = new Set(current)
                      if (next.has(request.id)) next.delete(request.id)
                      else next.add(request.id)
                      return next
                    })
                  }}
                  type="button"
                >
                  {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                </button>
                <button
                  className="min-w-0 flex-1 px-1 text-left"
                  data-testid={`pending-request-summary-${request.id}`}
                  onClick={() => {
                    setExpandedRequestIds((current) => new Set(current).add(request.id))
                    void borrow.handleSelectRequest(request.id)
                  }}
                  type="button"
                >
                  <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs">
                  <span className="font-semibold text-[var(--text-primary)]">{request.submittedEmployeeId}</span>
                  <span className="min-w-0 text-[var(--text-secondary)]">{request.submittedFullName}</span>
                  <RequestTypeBadge requestType={request.requestType} />
                  <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] text-[var(--text-secondary)]">
                    {formatAssetSummary(request.assetCodes)}
                  </span>
                  <span className="flex items-center gap-1" title={request.requestType === "borrow" ? "Handle with Care acknowledged" : "Return confirmation recorded"}>
                    {request.requestType === "borrow" && request.hasAcknowledgment && <FileCheck2 aria-label="Acknowledged" className="text-emerald-400" size={13} />}
                    {request.confirmationMethod && <span className="rounded-[999px] border border-[var(--border)] px-1.5 py-0.5 text-[10px] text-[var(--text-secondary)]">{request.confirmationMethod}</span>}
                    {request.hasSignature && <span title="Signature available"><PenLine aria-label="Signature available" className="text-emerald-400" size={13} /></span>}
                    {request.hasTypedName && <span title="Typed name available"><UserRound aria-label="Typed name available" className="text-sky-400" size={13} /></span>}
                  </span>
                  <span className="ml-auto text-[11px] text-[var(--text-secondary)]">{formatSubmittedTime(request.submittedAt)}</span>
                  </div>
                </button>
              </div>

              {isExpanded && (
                <div data-testid={`pending-request-details-${request.id}`} className="border-t border-[var(--border)] px-3 pb-3 pt-2.5">
                  {borrow.isLoadingDetail && isSelected && selectedDetail === null && (
                    <div className="flex items-center gap-2 text-xs text-[var(--text-secondary)]">
                      <LoaderCircle className="animate-spin" size={14} />
                      Loading request detail...
                    </div>
                  )}

                  {detail && (
                    <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr),280px]">
                      <div className="space-y-2 text-xs">
                        <div className="grid gap-2 sm:grid-cols-2">
                          <div>
                            <div className="text-[10px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">{detail.requestType === "return" ? "Borrowed by" : "Borrower"}</div>
                            <div className="mt-1 font-semibold text-[var(--text-primary)]">{detail.borrowerStaffId || "Historical identity unavailable"}{detail.borrowerName ? ` · ${detail.borrowerName}` : ""}</div>
                          </div>
                          <div>
                            <div className="text-[10px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">{detail.requestType === "return" ? "Returned by" : "Submitted by"}</div>
                            <div className="mt-1 font-semibold text-[var(--text-primary)]">{detail.submittedByStaffId || detail.submittedEmployeeId} · {detail.submittedByName || detail.submittedFullName}</div>
                          </div>
                        </div>
                        <div>
                          <div className="text-[10px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">Asset Items</div>
                          <div className="mt-1 flex flex-wrap gap-1.5">
                            {detail.assetCodes.map((assetCode) => (
                              <span
                                key={assetCode}
                                className="rounded-[999px] border border-[var(--primary)]/35 bg-[var(--primary)]/10 px-2.5 py-1 font-semibold text-[var(--text-primary)]"
                              >
                                {assetCode}
                              </span>
                            ))}
                          </div>
                        </div>
                        <div className="text-[var(--text-secondary)]">
                          <span className="mr-1 uppercase tracking-[0.06em]">Submitted At</span>
                          {detail.submittedAt}
                        </div>
                        <div className="text-[11px] text-[var(--text-secondary)]">
                          <span className="mr-1 uppercase tracking-[0.06em]">Request ID</span>
                          {detail.requestKey}
                        </div>
                        <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-[var(--text-secondary)]">
                          <span>Evidence:</span>
                          {detail.requestType === "borrow" && detail.hasAcknowledgment && <span className="rounded border border-emerald-500/40 px-1.5 py-0.5 text-emerald-300">Acknowledged</span>}
                          {detail.confirmationMethod && <span className="rounded border border-[var(--border)] px-1.5 py-0.5">{detail.confirmationMethod}</span>}
                          {detail.hasSignature && <span className="rounded border border-emerald-500/40 px-1.5 py-0.5 text-emerald-300">Signature</span>}
                          {detail.hasTypedName && <span className="rounded border border-sky-500/40 px-1.5 py-0.5 text-sky-300">Typed name</span>}
                        </div>
                        <button
                          className="rounded border border-[var(--border)] px-2 py-1 text-[11px] font-semibold text-[var(--text-primary)] disabled:opacity-50"
                          disabled={borrow.isLoadingEvidence}
                          onClick={() => void borrow.loadRequestEvidence(detail.id)}
                          type="button"
                        >
                          {borrow.isLoadingEvidence ? "Loading evidence..." : "View evidence"}
                        </button>
                        {borrow.selectedEvidence?.borrowRequestId === detail.id && (
                          <div className="rounded border border-[var(--border)] bg-[var(--surface)] p-2 text-[11px] text-[var(--text-secondary)]">
                            <div>Policy acknowledged: {borrow.selectedEvidence.policyAcknowledged ? "Yes" : "No"}</div>
                            {borrow.selectedEvidence.policyVersion !== null && <div>Policy version: {borrow.selectedEvidence.policyVersion}</div>}
                            {borrow.selectedEvidence.typedName && <div>Typed name: {borrow.selectedEvidence.typedName}</div>}
                            <div>Signature: {borrow.selectedEvidence.hasSignature ? "Available" : "Not provided"}</div>
                            <div>Confirmed: {borrow.selectedEvidence.confirmedAt}</div>
                            {borrow.selectedEvidence.policyTextEnSnapshot && <details className="mt-1"><summary className="cursor-pointer">Policy snapshot</summary><div className="mt-1 whitespace-pre-wrap">{borrow.selectedEvidence.policyTextEnSnapshot}</div>{borrow.selectedEvidence.policyTextViSnapshot && <div className="mt-2 whitespace-pre-wrap">{borrow.selectedEvidence.policyTextViSnapshot}</div>}</details>}
                            {borrow.selectedEvidence.signaturePngBase64 && <img alt="Signature preview" className="mt-2 max-h-24 max-w-full rounded border border-[var(--border)] bg-white object-contain" src={`data:image/png;base64,${borrow.selectedEvidence.signaturePngBase64}`} />}
                          </div>
                        )}
                      </div>

                      <div>
                        <label className="text-[10px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]" htmlFor={`rejection-note-${request.id}`}>
                          Review / rejection note
                        </label>
                        <textarea
                          ref={rejectionNoteRef}
                          id={`rejection-note-${request.id}`}
                          aria-label="Rejection note"
                          aria-invalid={needsRejectNote}
                          className={`form-input mt-1 min-h-[92px] resize-y text-sm ${needsRejectNote ? "border-red-400 ring-1 ring-red-400/50" : ""}`}
                          value={borrow.reviewNote}
                          onChange={(event) => borrow.setReviewNote(event.target.value)}
                          placeholder={buildBorrowReviewRejectPlaceholder(detail.requestType)}
                          disabled={!isSelected || borrow.isApproving || borrow.isRejecting || borrow.isCancelling}
                        />
                        {needsRejectNote && (
                          <div className="mt-1 text-xs text-red-300" role="alert">
                            {borrow.queueMessage}
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
