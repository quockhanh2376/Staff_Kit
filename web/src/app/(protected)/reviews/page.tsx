import type { Route } from "next";
import Link from "next/link";

import { getPendingReviewPreview } from "@/lib/admin/admin.service";

const formatDateTime = (value: Date) =>
  new Intl.DateTimeFormat("en-GB", { dateStyle: "medium", timeStyle: "short" }).format(value);

type ReviewsPageProps = {
  searchParams?: Promise<{
    updated?: string;
  }>;
};

export default async function ReviewsPage({ searchParams }: ReviewsPageProps) {
  const requests = await getPendingReviewPreview();
  const resolvedSearchParams = searchParams ? await searchParams : undefined;

  return (
    <main className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
            Pending reviews
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-tight">Receive and return queue</h1>
          <p className="mt-2 max-w-3xl text-sm leading-6 text-muted">
            This is the review-first admin screen. It should stay central because official stock
            and assignment mutations only happen after approval.
          </p>
        </div>
        <div className="rounded-full border border-border bg-surface-strong px-4 py-2 text-sm text-muted">
          {requests.length} pending requests
        </div>
      </div>

      {resolvedSearchParams?.updated ? (
        <div className="mt-6 rounded-2xl border border-emerald-500/30 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-200">
          Review completed for {resolvedSearchParams.updated}.
        </div>
      ) : null}

      <div className="mt-6 space-y-4">
        {requests.map((request) => (
          <div key={request.requestKey} className="rounded-[24px] border border-border bg-surface-strong px-5 py-5">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.18em] text-muted">{request.requestType}</p>
                <h2 className="mt-2 text-lg font-semibold text-foreground">{request.requestKey}</h2>
                <p className="mt-2 text-sm text-muted">
                  {request.employee.employeeId} - {request.employee.fullName}
                </p>
              </div>
              <div className="text-sm text-muted">
                Submitted: {formatDateTime(request.submittedAt)}
              </div>
            </div>

            <div className="mt-4 flex flex-wrap gap-2">
              {request.assetCodes.map((assetCode) => (
                <span key={assetCode} className="rounded-full border border-border px-3 py-1 text-xs text-muted">
                  {assetCode}
                </span>
              ))}
            </div>

            <div className="mt-4">
              <Link
                href={
                  `/reviews/${request.requestType.toLowerCase()}/${request.requestKey}` as Route
                }
                className="inline-flex rounded-full border border-border bg-surface px-4 py-2 text-sm text-foreground transition hover:border-accent hover:text-accent"
              >
                Review
              </Link>
            </div>
          </div>
        ))}
      </div>
    </main>
  );
}
