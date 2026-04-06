import { z } from "zod";

import { AccountRole as AccountRoleValues } from "@/generated/prisma/client";
import type { AccountRole } from "@/generated/prisma/client";
import { logEvent } from "@/lib/logger";
import { verifyPassword } from "@/lib/password";

const credentialsSchema = z.object({
  username: z.string().trim().min(1),
  password: z.string().min(1),
});

type AccountRecord = {
  id: number;
  displayName: string;
  username: string;
  passwordHash: string;
  role: AccountRole;
  forcePasswordReset: boolean;
  isActive: boolean;
};

type AuthorizeDependencies = {
  findAccountByUsername: (username: string) => Promise<AccountRecord | null>;
  updateLastLoginAt: (id: number) => Promise<void>;
  verifyPassword?: typeof verifyPassword;
  logEvent?: typeof logEvent;
};

export type AuthenticatedUser = {
  id: string;
  name: string;
  username: string;
  role: AccountRole;
  forcePasswordReset: boolean;
};

export type SessionLike = {
  user?: {
    id?: string;
    name?: string | null;
    username?: string;
    role?: AccountRole;
    forcePasswordReset?: boolean;
  };
};

export type TokenLike = {
  sub?: string;
  name?: unknown;
  username?: unknown;
  role?: unknown;
  forcePasswordReset?: unknown;
};

export function isAccountRole(value: unknown): value is AccountRole {
  return (
    typeof value === "string" &&
    Object.values(AccountRoleValues).includes(value as AccountRole)
  );
}

export async function authorizeCredentials(
  rawCredentials: unknown,
  dependencies: AuthorizeDependencies,
): Promise<AuthenticatedUser | null> {
  const parsed = credentialsSchema.safeParse(rawCredentials);

  if (!parsed.success) {
    return null;
  }

  const { username, password } = parsed.data;
  const verifyPasswordFn = dependencies.verifyPassword ?? verifyPassword;
  const logEventFn = dependencies.logEvent ?? logEvent;
  const account = await dependencies.findAccountByUsername(username);

  if (!account || !account.isActive) {
    logEventFn({
      level: "warn",
      feature: "auth",
      action: "credentials.authorize",
      result: "account_not_found_or_inactive",
      context: {
        username,
      },
    });

    return null;
  }

  const isValid = await verifyPasswordFn(account.passwordHash, password);

  if (!isValid) {
    logEventFn({
      level: "warn",
      feature: "auth",
      action: "credentials.authorize",
      result: "invalid_password",
      context: {
        username,
      },
    });

    return null;
  }

  await dependencies.updateLastLoginAt(account.id);

  logEventFn({
    level: "info",
    feature: "auth",
    action: "credentials.authorize",
    result: "success",
    context: {
      username,
      role: account.role,
    },
  });

  return {
    id: String(account.id),
    name: account.displayName,
    username: account.username,
    role: account.role,
    forcePasswordReset: account.forcePasswordReset,
  };
}

export function applyUserToToken<TToken extends TokenLike>(
  token: TToken,
  user?: {
    name?: string | null;
    username: string;
    role: AccountRole;
    forcePasswordReset: boolean;
  } | null,
): TToken {
  if (user) {
    token.name = user.name ?? user.username;
    token.username = user.username;
    token.role = user.role;
    token.forcePasswordReset = user.forcePasswordReset;
  }

  return token;
}

export function applyTokenToSession<TSession extends SessionLike>(
  session: TSession,
  token: TokenLike,
) {
  if (session.user && token.sub) {
    const resolvedName =
      typeof session.user.name === "string"
        ? session.user.name
        : typeof token.name === "string"
          ? token.name
          : typeof token.username === "string"
            ? token.username
            : "User";
    const resolvedRole = isAccountRole(token.role) ? token.role : "USER";

    session.user.id = token.sub;
    session.user.name = resolvedName;
    session.user.username =
      typeof token.username === "string" ? token.username : "unknown";
    session.user.role = resolvedRole;
    session.user.forcePasswordReset = Boolean(token.forcePasswordReset);
  }

  return session;
}
