"use server";

import { AuthError } from "next-auth";

import { signIn } from "@/auth";

export type LoginFormState = {
  error: string | null;
};

export async function loginAction(
  _previousState: LoginFormState,
  formData: FormData,
): Promise<LoginFormState> {
  const username = formData.get("username")?.toString().trim() ?? "";
  const password = formData.get("password")?.toString() ?? "";

  if (!username || !password) {
    return {
      error: "Username and password are required.",
    };
  }

  try {
    await signIn("credentials", {
      username,
      password,
      redirectTo: "/dashboard",
    });
  } catch (error) {
    if (error instanceof AuthError) {
      return {
        error: "Invalid username or password.",
      };
    }

    throw error;
  }

  return {
    error: null,
  };
}
