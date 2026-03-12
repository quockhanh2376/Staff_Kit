import { getReturnSessionPreview } from "@/lib/admin/admin.service";

const formatDateTime = (value: Date | null) =>
  value ? new Intl.DateTimeFormat("en-GB", { dateStyle: "medium", timeStyle: "short" }).format(value) : "No expiry";

export default async function ReturnPage() {
  const sessions = await getReturnSessionPreview();

  return (
    <main className="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
      <section className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
        <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">Scan Return</p>
        <h1 className="mt-2 text-3xl font-semibold tracking-tight">Return session preview</h1>
        <p className="mt-2 max-w-3xl text-sm leading-6 text-muted">
          This page exposes seeded return sessions and is the landing point for the future QR
          generator, session history, and manual close actions.
        </p>

        <div className="mt-6 space-y-4">
          {sessions.map((session) => (
            <div key={session.id} className="rounded-[24px] border border-border bg-surface-strong px-5 py-5">
              <p className="text-xs uppercase tracking-[0.18em] text-muted">{session.status}</p>
              <p className="mt-2 font-mono text-sm text-foreground">{session.qrToken}</p>
              <p className="mt-3 text-sm text-muted">Expires: {formatDateTime(session.expiresAt)}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
        <h2 className="text-2xl font-semibold">Current API hooks</h2>
        <div className="mt-5 space-y-3 text-sm text-muted">
          <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
            <p className="font-semibold text-foreground">Create session</p>
            <p className="mt-2 font-mono text-xs">POST /api/workflows/return-sessions</p>
          </div>
          <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
            <p className="font-semibold text-foreground">Submit return request</p>
            <p className="mt-2 font-mono text-xs">POST /api/workflows/return-requests</p>
          </div>
        </div>
      </section>
    </main>
  );
}
