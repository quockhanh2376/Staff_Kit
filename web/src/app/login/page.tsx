import type { Route } from "next";
import { redirect } from "next/navigation";

import { auth } from "@/auth";

import { LoginForm } from "./LoginForm";

export default async function LoginPage() {
  const session = await auth();
  const dashboardHref: Route = "/dashboard";

  if (session?.user) {
    redirect(dashboardHref);
  }

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top_left,_rgba(34,197,94,0.22),_transparent_28%),linear-gradient(180deg,_rgba(12,24,16,0.16),_transparent)] px-6 py-10">
      <div className="mx-auto grid min-h-[calc(100vh-5rem)] max-w-6xl gap-8 lg:grid-cols-[1.1fr_0.9fr]">
        <section className="flex flex-col justify-between rounded-[32px] border border-border bg-surface px-8 py-8 shadow-[0_28px_80px_rgba(8,15,15,0.12)] backdrop-blur">
          <div className="space-y-5">
            <div className="inline-flex rounded-full border border-border bg-surface-strong px-4 py-1 text-sm font-medium text-accent">
              AssetDesk-Pro
            </div>
            <div className="space-y-3">
              <h1 className="max-w-2xl text-4xl font-semibold tracking-tight sm:text-5xl">
                Sign in to the new AssetDesk-Pro workspace.
              </h1>
              <p className="max-w-2xl text-base leading-7 text-muted sm:text-lg">
                This is the Phase 1 auth shell: Prisma-backed credentials
                authentication, role-aware sessions, and a protected dashboard
                baseline for the upcoming AssetDesk-Pro workflows.
              </p>
            </div>
          </div>

          <div className="grid gap-4 pt-8 text-sm text-muted sm:grid-cols-3">
            <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
              <p className="font-semibold text-foreground">Default locale</p>
              <p className="mt-1">English-first text foundation</p>
            </div>
            <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
              <p className="font-semibold text-foreground">Auth strategy</p>
              <p className="mt-1">Credentials + JWT session</p>
            </div>
            <div className="rounded-3xl border border-border bg-surface-strong px-5 py-4">
              <p className="font-semibold text-foreground">Bootstrap user</p>
              <p className="mt-1">Seeded from environment values</p>
            </div>
          </div>
        </section>

        <section className="flex items-center">
          <div className="w-full rounded-[32px] border border-border bg-surface px-6 py-7 shadow-[0_28px_80px_rgba(8,15,15,0.12)] backdrop-blur sm:px-8">
            <div className="mb-6 space-y-2">
              <h2 className="text-2xl font-semibold">Account login</h2>
              <p className="text-sm leading-6 text-muted">
                Use the seeded bootstrap admin account or any future Prisma
                account created in the AssetDesk-Pro local database.
              </p>
            </div>

            <LoginForm />
          </div>
        </section>
      </div>
    </main>
  );
}
