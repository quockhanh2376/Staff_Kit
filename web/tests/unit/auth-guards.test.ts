import { beforeEach, describe, expect, it, vi } from "vitest";

const redirectMock = vi.fn((href: string) => {
  throw new Error(`redirect:${href}`);
});

const authMock = vi.fn();

vi.mock("next/navigation", () => ({
  redirect: redirectMock,
}));

vi.mock("@/auth", () => ({
  auth: authMock,
}));

describe("auth guards", () => {
  beforeEach(() => {
    redirectMock.mockClear();
    authMock.mockReset();
  });

  it("redirects to login when the user is missing", async () => {
    authMock.mockResolvedValue(null);
    const { requireUser } = await import("@/lib/auth/guards");

    await expect(requireUser()).rejects.toThrow("redirect:/login");
  });

  it("returns the session when a user exists", async () => {
    authMock.mockResolvedValue({
      user: {
        id: "7",
        username: "adman",
        role: "SUPER_ADMIN",
      },
    });
    const { requireUser } = await import("@/lib/auth/guards");

    await expect(requireUser()).resolves.toEqual({
      user: {
        id: "7",
        username: "adman",
        role: "SUPER_ADMIN",
      },
    });
  });

  it("redirects to dashboard when the role is insufficient", async () => {
    authMock.mockResolvedValue({
      user: {
        id: "8",
        username: "reader",
        role: "USER",
      },
    });
    const { requireRole } = await import("@/lib/auth/guards");

    await expect(requireRole("ADMIN")).rejects.toThrow("redirect:/dashboard");
  });
});
