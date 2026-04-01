import { getAuditTrailPreview } from "@/lib/admin/admin.service";

const formatDateTime = (value: Date) =>
  new Intl.DateTimeFormat("en-GB", { dateStyle: "medium", timeStyle: "short" }).format(value);

export default async function AuditPage() {
  const logs = await getAuditTrailPreview();

  return (
    <main className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
      <div className="space-y-3">
        <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">Audit Logs</p>
        <h1 className="text-3xl font-semibold tracking-tight">Recent audit trail preview</h1>
        <p className="max-w-3xl text-sm leading-6 text-muted">
          Audit is already wired at service level. This page is the first admin surface for review,
          filtering, and later export.
        </p>
      </div>

      <div className="mt-6 space-y-3">
        {logs.map((log) => (
          <div key={log.id} className="rounded-[24px] border border-border bg-surface-strong px-5 py-4">
            <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <p className="text-sm font-semibold text-foreground">
                  {log.actionType} - {log.entityLabel ?? log.entityType}
                </p>
                <p className="mt-1 text-sm text-muted">
                  Actor: {log.actorUsername ?? log.actorAccount?.username ?? "system"}
                </p>
              </div>
              <div className="text-sm text-muted">{formatDateTime(log.occurredAt)}</div>
            </div>
            <p className="mt-3 text-xs uppercase tracking-[0.18em] text-muted">
              Result: {log.result}
            </p>
          </div>
        ))}
      </div>
    </main>
  );
}
