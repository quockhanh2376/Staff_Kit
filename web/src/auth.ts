import NextAuth from "next-auth";
import Credentials from "next-auth/providers/credentials";

import { prisma } from "@/lib/prisma";
import {
  applyTokenToSession,
  applyUserToToken,
  authorizeCredentials,
} from "@/lib/auth/core";

export const { handlers, auth, signIn, signOut } = NextAuth({
  trustHost: true,
  session: {
    strategy: "jwt",
  },
  pages: {
    signIn: "/login",
  },
  providers: [
    Credentials({
      name: "AssetDesk-Pro Credentials",
      credentials: {
        username: {
          label: "Username",
          type: "text",
        },
        password: {
          label: "Password",
          type: "password",
        },
      },
      async authorize(rawCredentials) {
        return authorizeCredentials(rawCredentials, {
          findAccountByUsername: (username) =>
            prisma.localAccount.findUnique({
              where: {
                username,
              },
            }),
          updateLastLoginAt: async (id) => {
            await prisma.localAccount.update({
              where: {
                id,
              },
              data: {
                lastLoginAt: new Date(),
              },
            });
          },
        });
      },
    }),
  ],
  callbacks: {
    async jwt({ token, user }) {
      return applyUserToToken(token, user);
    },
    async session({ session, token }) {
      return applyTokenToSession(session, token);
    },
  },
});
