import type { Route } from "next";
import Link from "next/link";

import { signOut } from "@/auth";
import { requireUser } from "@/lib/auth/guards";

export default async function ProtectedLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const session = await requireUser();
  const dashboardHref: Route = "/dashboard";

  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top_left,_rgba(34,197,94,0.24),_transparent_30%),linear-gradient(180deg,_rgba(255,255,255,0.22),_transparent)] px-6 py-6 text-foreground">
      <div className="mx-auto flex min-h-[calc(100vh-3rem)] max-w-7xl flex-col gap-6">
        <header className="flex flex-col gap-4 rounded-[28px] border border-border bg-surface px-6 py-5 shadow-[0_20px_60px_rgba(8,15,15,0.12)] backdrop-blur sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-1">
            <Link className="text-2xl font-semibold tracking-tight" href={dashboardHref}>
              AssetDesk-Pro
            </Link>
            <p className="text-sm text-muted">
              Signed in as {session.user.name} ({session.user.role})
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <div className="rounded-full border border-border bg-surface-strong px-4 py-2 text-sm text-muted">
              {session.user.username}
            </div>
            <form
              action={async () => {
                "use server";
                await signOut({
                  redirectTo: "/login",
                });
              }}
            >
              <button
                className="rounded-full border border-border bg-surface-strong px-4 py-2 text-sm font-medium text-foreground transition hover:border-accent hover:text-accent"
                type="submit"
              >
                Sign out
              </button>
            </form>
          </div>
        </header>

        <div className="flex-1">{children}</div>
      </div>
    </div>
  );
}
