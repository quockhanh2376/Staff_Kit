"use client";

import { useActionState } from "react";

import type { LoginFormState } from "./actions";
import { loginAction } from "./actions";

const INITIAL_STATE: LoginFormState = {
  error: null,
};

export function LoginForm() {
  const [state, formAction, isPending] = useActionState(
    loginAction,
    INITIAL_STATE,
  );

  return (
    <form action={formAction} className="space-y-5">
      <div className="space-y-2">
        <label
          className="block text-sm font-medium text-foreground"
          htmlFor="username"
        >
          Username
        </label>
        <input
          required
          autoComplete="username"
          className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-base outline-none transition focus:border-accent"
          id="username"
          name="username"
          placeholder="Enter your AssetDesk-Pro username"
          type="text"
        />
      </div>

      <div className="space-y-2">
        <label
          className="block text-sm font-medium text-foreground"
          htmlFor="password"
        >
          Password
        </label>
        <input
          required
          autoComplete="current-password"
          className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-base outline-none transition focus:border-accent"
          id="password"
          name="password"
          placeholder="Enter your password"
          type="password"
        />
      </div>

      {state.error ? (
        <div className="rounded-2xl border border-red-500/25 bg-red-500/10 px-4 py-3 text-sm text-red-200 dark:text-red-300">
          {state.error}
        </div>
      ) : null}

      <button
        className="w-full rounded-2xl bg-accent px-4 py-3 text-base font-semibold text-white transition hover:bg-accent-strong disabled:cursor-not-allowed disabled:opacity-70"
        disabled={isPending}
        type="submit"
      >
        {isPending ? "Signing in..." : "Sign in"}
      </button>
    </form>
  );
}
