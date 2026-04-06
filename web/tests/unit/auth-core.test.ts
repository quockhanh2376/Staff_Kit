import { describe, expect, it, vi } from "vitest";

import {
  applyTokenToSession,
  applyUserToToken,
  authorizeCredentials,
} from "@/lib/auth/core";

describe("authorizeCredentials", () => {
  it("returns null when credentials are invalid", async () => {
    const result = await authorizeCredentials(
      { username: "", password: "" },
      {
        findAccountByUsername: vi.fn(),
        updateLastLoginAt: vi.fn(),
      },
    );

    expect(result).toBeNull();
  });

  it("returns null for inactive accounts", async () => {
    const findAccountByUsername = vi.fn().mockResolvedValue({
      id: 7,
      displayName: "adman",
      username: "adman",
      passwordHash: "hash",
      role: "SUPER_ADMIN",
      forcePasswordReset: false,
      isActive: false,
    });
    const updateLastLoginAt = vi.fn();
    const verifyPassword = vi.fn();

    const result = await authorizeCredentials(
      { username: "adman", password: "20252026" },
      {
        findAccountByUsername,
        updateLastLoginAt,
        verifyPassword,
        logEvent: vi.fn(),
      },
    );

    expect(result).toBeNull();
    expect(verifyPassword).not.toHaveBeenCalled();
    expect(updateLastLoginAt).not.toHaveBeenCalled();
  });

  it("returns null for invalid password", async () => {
    const updateLastLoginAt = vi.fn();
    const verifyPassword = vi.fn().mockResolvedValue(false);

    const result = await authorizeCredentials(
      { username: "adman", password: "wrong" },
      {
        findAccountByUsername: vi.fn().mockResolvedValue({
          id: 7,
          displayName: "adman",
          username: "adman",
          passwordHash: "hash",
          role: "SUPER_ADMIN",
          forcePasswordReset: false,
          isActive: true,
        }),
        updateLastLoginAt,
        verifyPassword,
        logEvent: vi.fn(),
      },
    );

    expect(result).toBeNull();
    expect(updateLastLoginAt).not.toHaveBeenCalled();
  });

  it("returns a mapped user and updates last login for valid credentials", async () => {
    const updateLastLoginAt = vi.fn();
    const verifyPassword = vi.fn().mockResolvedValue(true);
    const logEvent = vi.fn();

    const result = await authorizeCredentials(
      { username: "adman", password: "20252026" },
      {
        findAccountByUsername: vi.fn().mockResolvedValue({
          id: 7,
          displayName: "Admin Man",
          username: "adman",
          passwordHash: "hash",
          role: "SUPER_ADMIN",
          forcePasswordReset: false,
          isActive: true,
        }),
        updateLastLoginAt,
        verifyPassword,
        logEvent,
      },
    );

    expect(result).toEqual({
      id: "7",
      name: "Admin Man",
      username: "adman",
      role: "SUPER_ADMIN",
      forcePasswordReset: false,
    });
    expect(updateLastLoginAt).toHaveBeenCalledWith(7);
    expect(logEvent).toHaveBeenCalledWith(
      expect.objectContaining({
        result: "success",
      }),
    );
  });
});

describe("auth session helpers", () => {
  it("applies authenticated user values to the token", () => {
    const token = applyUserToToken(
      {},
      {
        name: "Admin Man",
        username: "adman",
        role: "SUPER_ADMIN",
        forcePasswordReset: true,
      },
    );

    expect(token).toMatchObject({
      name: "Admin Man",
      username: "adman",
      role: "SUPER_ADMIN",
      forcePasswordReset: true,
    });
  });

  it("maps token values into the session user", () => {
    const session = applyTokenToSession(
      {
        user: {},
      },
      {
        sub: "99",
        username: "adman",
        role: "SUPER_ADMIN",
        forcePasswordReset: false,
      },
    );

    expect(session).toEqual({
      user: {
        id: "99",
        name: "adman",
        username: "adman",
        role: "SUPER_ADMIN",
        forcePasswordReset: false,
      },
    });
  });

  it("falls back to USER role and unknown username when token data is malformed", () => {
    const session = applyTokenToSession(
      {
        user: {},
      },
      {
        sub: "11",
        name: {},
        username: {},
        role: "NOT_A_ROLE",
        forcePasswordReset: 0,
      },
    );

    expect(session).toEqual({
      user: {
        id: "11",
        name: "User",
        username: "unknown",
        role: "USER",
        forcePasswordReset: false,
      },
    });
  });
});
