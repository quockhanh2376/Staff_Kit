import type { FormEvent } from "react"
import {
    Eye,
    EyeOff,
    LoaderCircle,
    LogIn,
    Moon,
    Sun,
    Users,
} from "lucide-react"
import type { Theme } from "../../types/app"
import type { AuthState } from "./useAuthState"

type LoginPageProps = {
    theme: Theme
    setTheme: (theme: Theme) => void
    auth: AuthState
    isBootstrapping: boolean
    globalError: string | null
    setGlobalError: (msg: string | null) => void
}

export function LoginPage({
    theme,
    setTheme,
    auth,
    isBootstrapping,
    globalError,
    setGlobalError,
}: LoginPageProps) {
    return (
        <div className="min-h-screen bg-[var(--bg)] text-[var(--text-primary)]">
            <header className="sticky top-0 z-40 h-16 border-b border-[var(--border)] bg-[var(--bg)]/95 px-5 backdrop-blur">
                <div className="flex h-full w-full items-center gap-4">
                    <div className="flex min-w-[220px] items-center gap-3">
                        <div className="flex h-9 w-9 items-center justify-center rounded-[8px] bg-[var(--primary)]/15 text-[var(--primary)]">
                            <Users size={18} />
                        </div>
                        <span className="text-[30px] font-bold leading-none">Staff Kit</span>
                    </div>

                    <div className="ml-auto flex items-center gap-4">
                        <div className="flex items-center rounded-[8px] border border-[var(--border)] p-1">
                            <button
                                className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${theme === "dark"
                                    ? "bg-[var(--primary)]/20 text-[var(--primary)]"
                                    : "text-[var(--text-secondary)]"
                                    }`}
                                onClick={() => setTheme("dark")}
                                type="button"
                            >
                                <span className="inline-flex items-center gap-1">
                                    <Moon size={14} />
                                    Dark
                                </span>
                            </button>
                            <button
                                className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${theme === "light"
                                    ? "bg-[var(--primary)]/20 text-[var(--primary)]"
                                    : "text-[var(--text-secondary)]"
                                    }`}
                                onClick={() => setTheme("light")}
                                type="button"
                            >
                                <span className="inline-flex items-center gap-1">
                                    <Sun size={14} />
                                    Light
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
            </header>

            <main className="flex h-[calc(100vh-4rem)] items-center justify-center px-4 py-6">
                <section className="w-full max-w-[440px] rounded-[14px] border border-[var(--border)] bg-[var(--surface)] p-5 shadow-[0_8px_20px_-12px_rgba(0,0,0,0.6)]">
                    <h1 className="text-2xl font-bold">{auth.isForgotPasswordMode ? "Forgot Password" : "Login"}</h1>
                    <p className="mt-1 text-sm text-[var(--text-secondary)]">
                        {auth.isForgotPasswordMode
                            ? "Enter username + recovery code to set a new password."
                            : "Welcome to Staff Kit ! Please Sign In"}
                    </p>

                    {globalError && (
                        <div className="mt-4 rounded-[10px] border border-red-500/50 bg-red-500/10 px-3 py-2 text-sm text-red-300">
                            {globalError}
                        </div>
                    )}

                    {(isBootstrapping || auth.isLoadingAccounts) && (
                        <div className="mt-4 flex items-center gap-2 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/40 px-3 py-2 text-sm text-[var(--text-secondary)]">
                            <LoaderCircle className="animate-spin" size={15} />
                            Loading local accounts...
                        </div>
                    )}

                    {!isBootstrapping && !auth.isLoadingAccounts && auth.accounts.length === 0 && (
                        <div className="mt-4 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 px-3 py-2 text-sm text-[var(--text-secondary)]">
                            No local account found. Please restart app after migration, or use Temporary Reset in Settings if data is inconsistent.
                        </div>
                    )}

                    {auth.accounts.length > 0 && (
                        <>
                            {!auth.isForgotPasswordMode ? (
                                <form
                                    className="mt-4 space-y-3"
                                    onSubmit={(event: FormEvent<HTMLFormElement>) =>
                                        void auth.handleLoginSubmit(event, setGlobalError)
                                    }
                                >
                                    <label className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                        Username
                                    </label>
                                    <input
                                        className="form-input"
                                        value={auth.loginUsername}
                                        onChange={(event) => auth.setLoginUsername(event.target.value)}
                                        disabled={auth.isSigningIn}
                                        autoComplete="username"
                                    />
                                    <label className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                        Password
                                    </label>
                                    <div className="relative">
                                        <input
                                            className="form-input pr-10"
                                            type={auth.showLoginPassword ? "text" : "password"}
                                            value={auth.loginPassword}
                                            onChange={(event) => auth.setLoginPassword(event.target.value)}
                                            disabled={auth.isSigningIn}
                                            autoComplete="current-password"
                                        />
                                        <button
                                            className="absolute right-2 top-1/2 inline-flex -translate-y-1/2 items-center justify-center rounded-[6px] p-1 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                                            type="button"
                                            onClick={() => auth.setShowLoginPassword((v) => !v)}
                                            aria-label={auth.showLoginPassword ? "Hide password" : "Show password"}
                                        >
                                            {auth.showLoginPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                                        </button>
                                    </div>

                                    <button
                                        className="inline-flex w-full items-center justify-center gap-2 rounded-[8px] bg-[var(--primary)] px-4 py-2.5 font-semibold text-[#00131c] transition hover:brightness-110 disabled:opacity-50"
                                        type="submit"
                                        disabled={!auth.loginUsername.trim() || !auth.loginPassword.trim() || auth.isSigningIn}
                                    >
                                        {auth.isSigningIn ? <LoaderCircle className="animate-spin" size={16} /> : <LogIn size={16} />}
                                        {auth.isSigningIn ? "Signing in..." : "Login"}
                                    </button>

                                    <button
                                        className="w-full rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                                        type="button"
                                        onClick={() => {
                                            auth.setForgotPasswordMode(true)
                                            auth.setForgotUsername(auth.loginUsername)
                                            auth.setShowForgotNewPassword(false)
                                            setGlobalError(null)
                                        }}
                                    >
                                        Forgot password
                                    </button>
                                </form>
                            ) : (
                                <form
                                    className="mt-4 space-y-3"
                                    onSubmit={(event: FormEvent<HTMLFormElement>) =>
                                        void auth.handleForgotPasswordSubmit(event, setGlobalError)
                                    }
                                >
                                    <label className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                        Username
                                    </label>
                                    <input
                                        className="form-input"
                                        value={auth.forgotUsername}
                                        onChange={(event) => auth.setForgotUsername(event.target.value)}
                                        disabled={auth.isSubmittingForgotPassword}
                                    />
                                    <label className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                        Recovery Code
                                    </label>
                                    <input
                                        className="form-input"
                                        value={auth.forgotRecoveryCode}
                                        onChange={(event) => auth.setForgotRecoveryCode(event.target.value)}
                                        disabled={auth.isSubmittingForgotPassword}
                                    />
                                    <label className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                        New Password
                                    </label>
                                    <div className="relative">
                                        <input
                                            className="form-input pr-10"
                                            type={auth.showForgotNewPassword ? "text" : "password"}
                                            value={auth.forgotNewPassword}
                                            onChange={(event) => auth.setForgotNewPassword(event.target.value)}
                                            disabled={auth.isSubmittingForgotPassword}
                                        />
                                        <button
                                            className="absolute right-2 top-1/2 inline-flex -translate-y-1/2 items-center justify-center rounded-[6px] p-1 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                                            type="button"
                                            onClick={() => auth.setShowForgotNewPassword((v) => !v)}
                                            aria-label={auth.showForgotNewPassword ? "Hide password" : "Show password"}
                                        >
                                            {auth.showForgotNewPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                                        </button>
                                    </div>
                                    <button
                                        className="inline-flex w-full items-center justify-center gap-2 rounded-[8px] bg-[var(--primary)] px-4 py-2.5 font-semibold text-[#00131c] transition hover:brightness-110 disabled:opacity-50"
                                        type="submit"
                                        disabled={
                                            !auth.forgotUsername.trim() ||
                                            !auth.forgotRecoveryCode.trim() ||
                                            !auth.forgotNewPassword.trim() ||
                                            auth.isSubmittingForgotPassword
                                        }
                                    >
                                        {auth.isSubmittingForgotPassword ? <LoaderCircle className="animate-spin" size={16} /> : null}
                                        {auth.isSubmittingForgotPassword ? "Resetting..." : "Reset Password"}
                                    </button>
                                    <button
                                        className="w-full rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                                        type="button"
                                        onClick={() => {
                                            auth.setForgotPasswordMode(false)
                                            auth.setShowForgotNewPassword(false)
                                            setGlobalError(null)
                                        }}
                                    >
                                        Back to login
                                    </button>
                                </form>
                            )}
                        </>
                    )}
                </section>
            </main>
        </div>
    )
}
