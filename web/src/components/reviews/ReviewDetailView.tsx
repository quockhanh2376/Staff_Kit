import Link from "next/link";

import type { ReviewRequestDetail } from "@/lib/workflows/workflows.service";

type ReviewDetailViewProps = {
  detail: ReviewRequestDetail;
  requestTypeSegment: "receive" | "return";
  approveAction?: (formData: FormData) => void | Promise<void>;
  rejectAction?: (formData: FormData) => void | Promise<void>;
  errorCode?: string;
};

function formatDateTime(value: Date) {
  return new Intl.DateTimeFormat("en-GB", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(value);
}

export function ReviewDetailView({
  detail,
  requestTypeSegment,
  approveAction,
  rejectAction,
  errorCode,
}: ReviewDetailViewProps) {
  const reviewedAssetCodes = detail.reviewedDraft.assetCodes.join("\n");
  const hasEmployeeChange = detail.submittedSnapshot.employeeId !== detail.reviewedDraft.employeeId;
  const submittedAssetSet = new Set(detail.submittedSnapshot.assetCodes);
  const reviewedAssetSet = new Set(detail.reviewedDraft.assetCodes);

  return (
    <main className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
            Pending review detail
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-tight">{detail.requestKey}</h1>
          <p className="mt-2 text-sm leading-6 text-muted">
            Submitted {formatDateTime(detail.submittedAt)} · {detail.requestType} · {detail.status}
          </p>
        </div>
        <Link
          href="/reviews"
          className="rounded-full border border-border bg-surface-strong px-4 py-2 text-sm text-muted"
        >
          Back to queue
        </Link>
      </div>

      {errorCode ? (
        <div className="mt-6 rounded-2xl border border-red-500/40 bg-red-500/10 px-4 py-3 text-sm text-red-200">
          Review action failed: {errorCode}
        </div>
      ) : null}

      <div className="mt-6 grid gap-6 xl:grid-cols-[1.1fr_1fr]">
        <section className="rounded-[24px] border border-border bg-surface-strong px-5 py-5">
          <p className="text-xs uppercase tracking-[0.18em] text-muted">Submitted snapshot</p>
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <div className="rounded-2xl border border-border px-4 py-4">
              <p className="text-xs uppercase tracking-[0.18em] text-muted">Employee ID</p>
              <p className="mt-2 text-lg font-semibold">{detail.submittedSnapshot.employeeId}</p>
            </div>
            <div className="rounded-2xl border border-border px-4 py-4">
              <p className="text-xs uppercase tracking-[0.18em] text-muted">Employee name</p>
              <p className="mt-2 text-lg font-semibold">{detail.submittedSnapshot.employeeName}</p>
            </div>
          </div>

          <div className="mt-4 rounded-2xl border border-border px-4 py-4">
            <p className="text-xs uppercase tracking-[0.18em] text-muted">Submitted assets</p>
            <div className="mt-3 space-y-3">
              {detail.submittedSnapshot.items.map((item) => (
                <div
                  key={item.assetCode}
                  className="rounded-2xl border border-border bg-surface px-4 py-3"
                >
                  <div className="flex flex-wrap items-center gap-3">
                    <span className="text-sm font-semibold text-foreground">{item.assetCode}</span>
                    {reviewedAssetSet.has(item.assetCode) ? (
                      <span className="rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-1 text-[11px] uppercase tracking-[0.16em] text-emerald-300">
                        Kept
                      </span>
                    ) : (
                      <span className="rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-1 text-[11px] uppercase tracking-[0.16em] text-amber-300">
                        Removed in review
                      </span>
                    )}
                  </div>
                  <p className="mt-2 text-sm text-muted">{item.assetName ?? "Unnamed asset"}</p>
                  {item.assignmentEmployeeId ? (
                    <p className="mt-2 text-xs text-muted">
                      Assigned to {item.assignmentEmployeeId} · {item.assignmentEmployeeName}
                    </p>
                  ) : null}
                </div>
              ))}
            </div>
          </div>

          {detail.submittedNotes ? (
            <div className="mt-4 rounded-2xl border border-border px-4 py-4">
              <p className="text-xs uppercase tracking-[0.18em] text-muted">Employee notes</p>
              <p className="mt-2 text-sm leading-6 text-muted">{detail.submittedNotes}</p>
            </div>
          ) : null}
        </section>

        <form className="space-y-6 rounded-[24px] border border-border bg-surface-strong px-5 py-5">
          <input type="hidden" name="requestType" value={detail.requestType} />
          <input type="hidden" name="requestTypeSegment" value={requestTypeSegment} />
          <input type="hidden" name="requestKey" value={detail.requestKey} />

          <section>
            <p className="text-xs uppercase tracking-[0.18em] text-muted">IT review form</p>
            <div className="mt-4 space-y-4">
              <label className="block">
                <span className="text-sm font-medium text-foreground">Reviewed employee ID</span>
                <input
                  name="reviewedEmployeeId"
                  defaultValue={detail.reviewedDraft.employeeId}
                  className="mt-2 w-full rounded-2xl border border-border bg-surface px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                />
              </label>

              <label className="block">
                <span className="text-sm font-medium text-foreground">Review asset codes</span>
                <textarea
                  name="reviewedAssetCodesInput"
                  defaultValue={reviewedAssetCodes}
                  rows={Math.max(detail.reviewedDraft.assetCodes.length + 1, 4)}
                  className="mt-2 w-full rounded-2xl border border-border bg-surface px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                />
                <p className="mt-2 text-xs leading-5 text-muted">
                  {detail.rules.canAddAssets
                    ? "One asset code per line. IT can add or remove existing assets for receive review."
                    : "One asset code per line. Return review can only remove submitted assets."}
                </p>
              </label>

              <div className="rounded-2xl border border-border px-4 py-4">
                <p className="text-xs uppercase tracking-[0.18em] text-muted">Review diff</p>
                <div className="mt-3 space-y-2 text-sm text-muted">
                  <p>
                    Employee:{" "}
                    {hasEmployeeChange ? (
                      <span className="text-foreground">
                        {detail.submittedSnapshot.employeeId} → {detail.reviewedDraft.employeeId}
                      </span>
                    ) : (
                      <span>No employee change</span>
                    )}
                  </p>
                  <div className="flex flex-wrap gap-2">
                    {detail.reviewedDraft.assetCodes.map((assetCode) => (
                      <span
                        key={assetCode}
                        className={`rounded-full border px-3 py-1 text-xs ${
                          submittedAssetSet.has(assetCode)
                            ? "border-border text-muted"
                            : "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
                        }`}
                      >
                        {assetCode}
                      </span>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section>
            <p className="text-xs uppercase tracking-[0.18em] text-muted">Decision panel</p>
            <div className="mt-4 space-y-4">
              <label className="block">
                <span className="text-sm font-medium text-foreground">Review notes</span>
                <textarea
                  name="notes"
                  defaultValue={detail.reviewedDraft.notes ?? ""}
                  rows={4}
                  className="mt-2 w-full rounded-2xl border border-border bg-surface px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                />
                <p className="mt-2 text-xs leading-5 text-muted">
                  Reject always requires a reason. Approve will validate the reviewed payload again
                  before changing official data.
                </p>
              </label>

              <div className="flex flex-wrap gap-3">
                <button
                  type="submit"
                  formAction={approveAction}
                  className="rounded-full border border-emerald-500/30 bg-emerald-500/15 px-5 py-3 text-sm font-medium text-emerald-300 transition hover:bg-emerald-500/20"
                >
                  Approve
                </button>
                <button
                  type="submit"
                  formAction={rejectAction}
                  className="rounded-full border border-amber-500/30 bg-amber-500/10 px-5 py-3 text-sm font-medium text-amber-200 transition hover:bg-amber-500/15"
                >
                  Reject
                </button>
              </div>
            </div>
          </section>
        </form>
      </div>
    </main>
  );
}
