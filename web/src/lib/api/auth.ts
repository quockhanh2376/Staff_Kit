import type { AccountRole } from "@/generated/prisma/client";
import { auth } from "@/auth";

import { ApiError } from "./errors";

const ROLE_RANK: Record<AccountRole, number> = {
  USER: 0,
  ADMIN: 1,
  SUPER_ADMIN: 2,
};

export type ApiActor = {
  accountId: number;
  username: string;
  displayName: string;
  role: AccountRole;
};

export async function requireApiActor(requiredRole: AccountRole = "ADMIN") {
  const session = await auth();

  if (!session?.user) {
    throw new ApiError(401, "unauthorized", "Authentication is required.");
  }

  const accountId = Number.parseInt(session.user.id, 10);

  if (Number.isNaN(accountId)) {
    throw new ApiError(500, "invalid_session", "Session is missing a valid account id.");
  }

  if (ROLE_RANK[session.user.role] < ROLE_RANK[requiredRole]) {
    throw new ApiError(403, "forbidden", "You do not have permission to perform this action.");
  }

  return {
    accountId,
    username: session.user.username,
    displayName: session.user.name ?? session.user.username,
    role: session.user.role,
  } satisfies ApiActor;
}
