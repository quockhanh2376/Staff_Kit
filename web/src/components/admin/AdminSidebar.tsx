"use client";

import type { Route } from "next";
import Link from "next/link";
import { usePathname } from "next/navigation";

const navItems = [
  { href: "/dashboard", label: "Dashboard" },
  { href: "/employees", label: "Employees" },
  { href: "/assets", label: "Assets" },
  { href: "/reviews", label: "Pending Reviews" },
  { href: "/receive", label: "Scan Receive" },
  { href: "/return", label: "Scan Return" },
  { href: "/audit", label: "Audit Logs" },
] as const;

export function AdminSidebar() {
  const pathname = usePathname();

  return (
    <aside className="rounded-[32px] border border-border bg-surface px-4 py-5 shadow-[0_28px_80px_rgba(8,15,15,0.12)] backdrop-blur">
      <div className="border-b border-border px-3 pb-4">
        <p className="text-xs font-semibold uppercase tracking-[0.22em] text-accent">
          AssetDesk-Pro
        </p>
        <h2 className="mt-3 text-2xl font-semibold tracking-tight">
          Admin Workspace
        </h2>
        <p className="mt-2 text-sm leading-6 text-muted">
          Seeded preview shell for assets, QR flows, reviews, and audit.
        </p>
      </div>

      <nav className="mt-4 space-y-2">
        {navItems.map((item) => {
          const isActive =
            pathname === item.href ||
            (item.href !== "/dashboard" && pathname.startsWith(item.href));

          return (
            <Link
              key={item.href}
              className={`block rounded-2xl px-4 py-3 text-sm font-medium transition ${
                isActive
                  ? "border border-accent/30 bg-accent/12 text-accent shadow-[inset_0_0_0_1px_rgba(15,159,79,0.12)]"
                  : "border border-transparent text-foreground hover:border-border hover:bg-surface-strong"
              }`}
              href={item.href as Route}
            >
              {item.label}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
