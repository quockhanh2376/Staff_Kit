import { CheckCircle2, LoaderCircle, RefreshCw, Smartphone, XCircle } from "lucide-react"
import { QRCodeSVG } from "qrcode.react"
import type { AuthState } from "../auth/useAuthState"
import type { BorrowState } from "./useBorrowState"

type BorrowAdminViewProps = {
  auth: AuthState
  borrow: BorrowState
}

export function BorrowAdminView({ auth, borrow }: BorrowAdminViewProps) {
  if (!auth.isAdminAccount) {
    return (
      <section className="px-4 py-7 md:px-8">
        <h2 className="text-[30px] font-bold">Borrow Approval</h2>
        <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4 text-sm text-[var(--text-secondary)]">
          Admin access is required to review pending borrow requests.
        </div>
      </section>
    )
  }

  const borrowUrl = borrow.borrowLanSettings?.borrowUrl ?? ""

  return (
    <section className="px-4 py-7 md:px-8">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-[30px] font-bold">Borrow Approval</h2>
          <p className="mt-1 text-sm text-[var(--text-secondary)]">
            Employees scan the fixed LAN QR on their phone, submit a pending request, then IT approves the exact asset items here.
          </p>
        </div>
        <button
          className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
          onClick={() => void borrow.refreshQueue(borrow.selectedRequestId)}
          type="button"
          disabled={borrow.isLoadingQueue}
        >
          <RefreshCw className={borrow.isLoadingQueue ? "animate-spin" : ""} size={14} />
          Refresh Queue
        </button>
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-[minmax(0,1.4fr),360px]">
        <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
          <div className="text-sm font-semibold text-[var(--text-primary)]">LAN QR Flow</div>
          <div className="mt-4 flex flex-col gap-4 lg:flex-row lg:items-start">
            <div className="order-2 space-y-4 lg:order-1 lg:flex-1">
              <p className="text-xs leading-5 text-[var(--text-secondary)]">
                Keep Staff Kit open. The local LAN server is available while the app is running. If you changed the port, restart the app before testing the new QR URL.
              </p>

              {borrow.lanServerAlive === null && (
                <div className="flex items-center gap-2 text-xs text-[var(--text-secondary)]">
                  <LoaderCircle className="animate-spin" size={13} />
                  Checking LAN server…
                </div>
              )}
              {borrow.lanServerAlive === true && (
                <div className="flex items-center gap-2 text-xs text-emerald-400">
                  <CheckCircle2 size={13} />
                  LAN server active on port {borrow.borrowLanSettings?.port}.
                </div>
              )}
              {borrow.lanServerAlive === false && (
                <div className="space-y-1 rounded-[8px] border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-300">
                  <div className="flex items-center gap-2 font-semibold">
                    <XCircle size={13} />
                    LAN server not responding on port {borrow.borrowLanSettings?.port ?? "?"}.
                  </div>
                  <div>
                    Port may be in use by a previous session, or Windows Firewall is blocking inbound connections.
                    Restart the app — if it still fails, add a Firewall exception:{" "}
                    <span className="font-medium text-amber-200">
                      Windows Security → Firewall → Allow an app through firewall → Staff Kit
                    </span>
                  </div>
                </div>
              )}

              <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                  Borrow URL
                </div>
                <div className="mt-2 break-all rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-sm text-[var(--text-primary)]">
                  {borrowUrl || "Set host and port in Settings first."}
                </div>
                <div className="mt-3 flex items-start gap-2 rounded-[8px] border border-[var(--primary)]/30 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                  <Smartphone size={14} className="mt-0.5 shrink-0 text-[var(--primary)]" />
                  <span>
                    Phone and ST machine must stay on the same Wi-Fi/LAN. The employee form is intentionally narrow and public inside the local network only.
                  </span>
                </div>
              </div>
            </div>

            <div className="order-1 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3 lg:order-2 lg:w-[280px] lg:shrink-0 lg:self-start">
              <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                QR Preview
              </div>
              <div className="mt-3 flex items-center justify-center rounded-[10px] border border-[var(--border)] bg-white p-3">
                {borrowUrl ? (
                  <QRCodeSVG value={borrowUrl} size={180} bgColor="#ffffff" fgColor="#111827" includeMargin />
                ) : (
                  <div className="px-3 py-12 text-center text-xs text-slate-500">
                    Borrow URL is empty.
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

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
                No pending requests yet. Scan the QR on a phone to create the first request.
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
                    <div className="text-sm font-semibold text-[var(--text-primary)]">{request.submittedEmployeeId}</div>
                    {request.requestType === "return" ? (
                      <div className="rounded-[999px] border border-amber-500/40 bg-amber-500/10 px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-amber-300">
                        Return
                      </div>
                    ) : (
                      <div className="rounded-[999px] border border-emerald-500/40 bg-emerald-500/10 px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-emerald-300">
                        Borrow
                      </div>
                    )}
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
      </div>

      <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="text-sm font-semibold text-[var(--text-primary)]">Request Detail</div>
            <p className="mt-1 text-xs text-[var(--text-secondary)]">
              Review the exact asset codes before approval. Reject when the employee selected the wrong item.
            </p>
          </div>
          {borrow.selectedRequest && (
            <div className="flex items-center gap-2">
              {borrow.selectedRequest.requestType === "return" ? (
                <div className="rounded-[999px] border border-amber-500/40 bg-amber-500/10 px-3 py-1 text-[11px] uppercase tracking-[0.06em] text-amber-300">
                  Return
                </div>
              ) : (
                <div className="rounded-[999px] border border-emerald-500/40 bg-emerald-500/10 px-3 py-1 text-[11px] uppercase tracking-[0.06em] text-emerald-300">
                  Borrow
                </div>
              )}
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
                  <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">Staff ID</div>
                  <div className="mt-1 text-sm font-semibold text-[var(--text-primary)]">{borrow.selectedRequest.submittedEmployeeId}</div>
                </div>
                <div>
                  <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">Full Name</div>
                  <div className="mt-1 text-sm font-semibold text-[var(--text-primary)]">{borrow.selectedRequest.submittedFullName}</div>
                </div>
              </div>

              <div>
                <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">Submitted At</div>
                <div className="mt-1 text-sm text-[var(--text-primary)]">{borrow.selectedRequest.submittedAt}</div>
              </div>

              <div>
                <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">Asset Items</div>
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
                placeholder="Add a rejection note when the employee selected the wrong asset type or code."
                disabled={borrow.isApproving || borrow.isRejecting}
              />

              <div className="mt-4 flex flex-wrap gap-2">
                <button
                  className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                  onClick={() => void borrow.handleApproveRequest()}
                  type="button"
                  disabled={borrow.isApproving || borrow.isRejecting}
                >
                  {borrow.isApproving ? <LoaderCircle className="animate-spin" size={15} /> : <CheckCircle2 size={15} />}
                  {borrow.isApproving ? "Approving..." : "Approve Request"}
                </button>
                <button
                  className="inline-flex items-center gap-2 rounded-[8px] border border-red-400/50 bg-red-500/10 px-3 py-2 text-sm font-semibold text-red-200 disabled:opacity-50"
                  onClick={() => void borrow.handleRejectRequest()}
                  type="button"
                  disabled={borrow.isApproving || borrow.isRejecting}
                >
                  {borrow.isRejecting ? <LoaderCircle className="animate-spin" size={15} /> : <XCircle size={15} />}
                  {borrow.isRejecting ? "Rejecting..." : "Reject With Note"}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </section>
  )
}
