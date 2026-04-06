import type { DefaultSession } from "next-auth";

import type { AccountRole } from "@/generated/prisma/client";

declare module "next-auth" {
  interface Session {
    user: DefaultSession["user"] & {
      id: string;
      username: string;
      role: AccountRole;
      forcePasswordReset: boolean;
    };
  }

  interface User {
    username: string;
    role: AccountRole;
    forcePasswordReset: boolean;
  }
}

declare module "next-auth/jwt" {
  interface JWT {
    username?: string;
    role?: AccountRole;
    forcePasswordReset?: boolean;
  }
}
