import { getDashboardSnapshot } from "@/lib/admin/admin.service";

const formatDateTime = (value: Date | null) =>
  value ? new Intl.DateTimeFormat("en-GB", { dateStyle: "medium", timeStyle: "short" }).format(value) : "No expiry";

export default async function DashboardPage() {
  const snapshot = await getDashboardSnapshot();
  const statCards = [
    { label: "Employees", value: snapshot.employeeCount, tone: "text-foreground" },
    { label: "Assets", value: snapshot.assetCount, tone: "text-foreground" },
    { label: "Assigned", value: snapshot.assignedAssetCount, tone: "text-accent" },
    { label: "Pending Reviews", value: snapshot.pendingReceiveCount + snapshot.pendingReturnCount, tone: "text-accent" },
  ] as const;

  return (
    <main className="space-y-6">
      <section className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
        <div className="space-y-3">
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
            Admin overview
          </p>
          <h1 className="text-4xl font-semibold tracking-tight">
            Seeded workspace ready for real flow testing.
          </h1>
          <p className="max-w-4xl text-base leading-7 text-muted">
            The web app now has sample employees, assets, receive and return sessions,
            pending requests, reviewed requests, and audit logs. Use this shell to verify
            business flows before building deeper forms and CRUD screens.
          </p>
        </div>

        <div className="mt-8 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          {statCards.map((item) => (
            <div key={item.label} className="rounded-3xl border border-border bg-surface-strong px-5 py-5">
              <p className="text-xs uppercase tracking-[0.22em] text-muted">{item.label}</p>
              <p className={`mt-3 text-3xl font-semibold ${item.tone}`}>{item.value}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="grid gap-6 xl:grid-cols-2">
        <div className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
          <div className="flex items-center justify-between">
            <h2 className="text-2xl font-semibold">Active Receive Sessions</h2>
            <span className="rounded-full border border-border bg-surface-strong px-3 py-1 text-xs font-medium text-muted">
              {snapshot.activeReceiveSessions.length} live
            </span>
          </div>

          <div className="mt-5 space-y-3 text-sm">
            {snapshot.activeReceiveSessions.map((session) => (
              <div key={session.id} className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
                <p className="text-xs uppercase tracking-[0.22em] text-muted">QR token</p>
                <p className="mt-2 font-mono text-sm text-foreground">{session.qrToken}</p>
                <p className="mt-3 text-sm text-muted">Expires: {formatDateTime(session.expiresAt)}</p>
              </div>
            ))}
          </div>
        </div>

        <div className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
          <div className="flex items-center justify-between">
            <h2 className="text-2xl font-semibold">Active Return Sessions</h2>
            <span className="rounded-full border border-border bg-surface-strong px-3 py-1 text-xs font-medium text-muted">
              {snapshot.activeReturnSessions.length} live
            </span>
          </div>

          <div className="mt-5 space-y-3 text-sm">
            {snapshot.activeReturnSessions.map((session) => (
              <div key={session.id} className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
                <p className="text-xs uppercase tracking-[0.22em] text-muted">QR token</p>
                <p className="mt-2 font-mono text-sm text-foreground">{session.qrToken}</p>
                <p className="mt-3 text-sm text-muted">Expires: {formatDateTime(session.expiresAt)}</p>
              </div>
            ))}
          </div>
        </div>
      </section>
    </main>
  );
}
