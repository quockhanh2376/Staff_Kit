import type { Route } from "next";
import { redirect } from "next/navigation";

import type { AccountRole } from "@/generated/prisma/client";
import { auth } from "@/auth";

const ROLE_RANK: Record<AccountRole, number> = {
  USER: 0,
  ADMIN: 1,
  SUPER_ADMIN: 2,
};

export async function requireUser() {
  const session = await auth();
  const loginHref: Route = "/login";

  if (!session?.user) {
    redirect(loginHref);
  }

  return session;
}

export async function requireRole(requiredRole: AccountRole) {
  const session = await requireUser();
  const currentRole = session.user.role;
  const dashboardHref: Route = "/dashboard";

  if (ROLE_RANK[currentRole] < ROLE_RANK[requiredRole]) {
    redirect(dashboardHref);
  }

  return session;
}
