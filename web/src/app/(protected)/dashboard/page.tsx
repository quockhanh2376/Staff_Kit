import { auth } from "@/auth";

const foundationItems = [
  "Prisma schema and initial Postgres migration",
  "Bootstrap SUPER_ADMIN seed account",
  "NextAuth v5 credentials login with JWT session",
  "Protected dashboard shell with server-side guards",
] as const;

export default async function DashboardPage() {
  const session = await auth();

  return (
    <main className="grid gap-6 lg:grid-cols-[1.15fr_0.85fr]">
      <section className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
        <div className="space-y-3">
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
            Phase 1
          </p>
          <h1 className="text-4xl font-semibold tracking-tight">
            Auth foundation is live.
          </h1>
          <p className="max-w-3xl text-base leading-7 text-muted">
            The web workspace now has a real PostgreSQL-backed account model,
            seeded bootstrap access, and a protected shell ready for the next
            features: employee CRUD, asset modules, and approval workflows.
          </p>
        </div>

        <div className="mt-8 grid gap-4 sm:grid-cols-2">
          {foundationItems.map((item) => (
            <div
              key={item}
              className="rounded-3xl border border-border bg-surface-strong px-5 py-4"
            >
              <p className="text-sm font-medium text-foreground">{item}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
        <h2 className="text-2xl font-semibold">Session snapshot</h2>
        <div className="mt-5 space-y-3 text-sm">
          <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
            <p className="text-xs uppercase tracking-[0.22em] text-muted">
              Display name
            </p>
            <p className="mt-2 text-lg font-semibold text-foreground">
              {session?.user?.name ?? "Unknown"}
            </p>
          </div>
          <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
            <p className="text-xs uppercase tracking-[0.22em] text-muted">
              Username
            </p>
            <p className="mt-2 text-lg font-semibold text-foreground">
              {session?.user?.username ?? "Unknown"}
            </p>
          </div>
          <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
            <p className="text-xs uppercase tracking-[0.22em] text-muted">
              Role
            </p>
            <p className="mt-2 text-lg font-semibold text-foreground">
              {session?.user?.role ?? "Unknown"}
            </p>
          </div>
          <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
            <p className="text-xs uppercase tracking-[0.22em] text-muted">
              Force password reset
            </p>
            <p className="mt-2 text-lg font-semibold text-foreground">
              {session?.user?.forcePasswordReset ? "Yes" : "No"}
            </p>
          </div>
        </div>
      </section>
    </main>
  );
}
