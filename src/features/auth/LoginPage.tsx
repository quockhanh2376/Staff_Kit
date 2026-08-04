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
    const isDark = theme === "dark"
    const pageClass = isDark ? "bg-[#0d1117] text-slate-100" : "bg-[#edf2ee] text-slate-900"
    const headerClass = isDark
        ? "border-slate-900 bg-[#0d1117]/95"
        : "border-slate-200 bg-[#edf2ee]/95"
    const headerBrandClass = isDark
        ? "bg-emerald-500/12 text-emerald-400"
        : "bg-emerald-500/10 text-emerald-700"
    const themeToggleShellClass = isDark
        ? "border-slate-800 bg-[#161b22]"
        : "border-slate-300 bg-white/90"
    const activeThemeButtonClass = isDark
        ? "bg-emerald-500/16 text-emerald-400"
        : "bg-emerald-500/12 text-emerald-700"
    const inactiveThemeButtonClass = isDark
        ? "text-slate-400 hover:text-slate-200"
        : "text-slate-500 hover:text-slate-900"
    const cardClass = isDark
        ? "border-slate-800 bg-[#161b22] text-slate-100 shadow-2xl"
        : "border-slate-200 bg-white text-slate-900 shadow-[0_18px_40px_-24px_rgba(15,23,42,0.35)]"
    const titleClass = isDark ? "text-slate-100" : "text-slate-900"
    const subtitleClass = isDark ? "text-slate-400" : "text-slate-500"
    const fieldLabelClass = isDark
        ? "text-[11px] font-semibold uppercase tracking-wider text-slate-400"
        : "text-[11px] font-semibold uppercase tracking-wider text-slate-500"
    const inputClass = isDark
        ? "h-10 w-full rounded-md border border-slate-700 bg-[#0d1117] px-3 text-sm text-slate-200 outline-none transition-all placeholder:text-slate-500 focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 disabled:cursor-not-allowed disabled:opacity-60"
        : "h-10 w-full rounded-md border border-slate-300 bg-slate-50 px-3 text-sm text-slate-900 outline-none transition-all placeholder:text-slate-400 focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 disabled:cursor-not-allowed disabled:opacity-60"
    const passwordToggleClass = isDark
        ? "absolute right-0 top-0 flex h-full items-center justify-center px-3 text-slate-500 transition-colors hover:text-slate-300"
        : "absolute right-0 top-0 flex h-full items-center justify-center px-3 text-slate-400 transition-colors hover:text-slate-700"
    const primaryButtonClass =
        "inline-flex h-10 w-full items-center justify-center gap-2 rounded-md bg-emerald-600 text-sm font-medium text-white transition-colors hover:bg-emerald-500 disabled:cursor-not-allowed disabled:opacity-50"
    const secondaryButtonClass = isDark
        ? "flex h-10 w-full items-center justify-center rounded-md border border-slate-700 bg-transparent text-sm font-medium text-slate-300 transition-colors hover:bg-slate-800/50"
        : "flex h-10 w-full items-center justify-center rounded-md border border-slate-300 bg-transparent text-sm font-medium text-slate-700 transition-colors hover:bg-slate-100"
    const inlineBannerClass = isDark
        ? "mt-4 rounded-md border border-slate-800 bg-[#0d1117] px-3 py-2 text-sm text-slate-400"
        : "mt-4 rounded-md border border-slate-200 bg-slate-50 px-3 py-2 text-sm text-slate-600"

    return (
        <div className={`min-h-screen ${pageClass}`}>
            <header className={`sticky top-0 z-40 h-16 border-b px-5 backdrop-blur ${headerClass}`}>
                <div className="flex h-full w-full items-center gap-4">
                    <div className="flex min-w-[220px] items-center gap-3">
                        <div className={`flex h-9 w-9 items-center justify-center rounded-[8px] ${headerBrandClass}`}>
                            <Users size={18} />
                        </div>
                        <span className="text-[30px] font-bold leading-none">Staff Kit</span>
                    </div>

                    <div className="ml-auto flex items-center gap-4">
                        <div className={`flex items-center rounded-[8px] border p-1 ${themeToggleShellClass}`}>
                            <button
                                className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${theme === "dark" ? activeThemeButtonClass : inactiveThemeButtonClass}`}
                                onClick={() => setTheme("dark")}
                                type="button"
                            >
                                <span className="inline-flex items-center gap-1">
                                    <Moon size={14} />
                                    Dark
                                </span>
                            </button>
                            <button
                                className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${theme === "light" ? activeThemeButtonClass : inactiveThemeButtonClass}`}
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
                <section className={`w-full max-w-[420px] rounded-xl border p-7 sm:p-8 ${cardClass}`}>
                    <div className="mb-8">
                        <h1 className={`text-2xl font-bold ${titleClass}`}>{auth.isForgotPasswordMode ? "Forgot Password" : "Login"}</h1>
                        <p className={`mt-2 text-sm ${subtitleClass}`}>
                        {auth.isForgotPasswordMode
                            ? "Enter username + recovery code to set a new password."
                            : "Welcome to Staff Kit ! Please Sign In"}
                        </p>
                    </div>

                    {auth.loginError && (
                        <div className="mt-4 rounded-[10px] border border-red-500/50 bg-red-500/10 px-3 py-2 text-sm text-red-300">
                            {auth.loginError}
                        </div>
                    )}

                    {globalError && !auth.loginError && (
                        <div className="mt-4 rounded-[10px] border border-red-500/50 bg-red-500/10 px-3 py-2 text-sm text-red-300">
                            {globalError}
                        </div>
                    )}

                    {(isBootstrapping || auth.isLoadingAccounts) && (
                        <div className={`mt-4 flex items-center gap-2 ${inlineBannerClass}`}>
                            <LoaderCircle className="animate-spin" size={15} />
                            Loading local accounts...
                        </div>
                    )}

                    {/* Backend account-discovery failure: show the real error
                        instead of a misleading "No local account found" empty
                        state. Do not render the empty-state block while a load
                        is pending or has errored. */}
                    {!isBootstrapping && !auth.isLoadingAccounts && auth.accountsLoadError && (
                        <div className="mt-4 rounded-[10px] border border-red-500/50 bg-red-500/10 px-4 py-2 text-sm text-red-300">
                            Could not load accounts. {auth.accountsLoadError}
                        </div>
                    )}

                    {!isBootstrapping &&
                        !auth.isLoadingAccounts &&
                        !auth.accountsLoadError &&
                        auth.accounts.length === 0 && (
                            <div className={inlineBannerClass}>
                                No local account found. Please restart app after migration, or use
                                Temporary Reset in Settings if data is inconsistent.
                            </div>
                        )}

                    {auth.accounts.length > 0 && (
                        <>
                            {!auth.isForgotPasswordMode ? (
                                <form
                                    className="space-y-5"
                                    onSubmit={(event: FormEvent<HTMLFormElement>) =>
                                        void auth.handleLoginSubmit(event)
                                    }
                                >
                                    <div className="space-y-1.5">
                                        <label className={fieldLabelClass}>Username</label>
                                        <input
                                            className={inputClass}
                                            value={auth.loginUsername}
                                            onChange={(event) => auth.setLoginUsername(event.target.value)}
                                            disabled={auth.isSigningIn}
                                            autoComplete="username"
                                        />
                                    </div>
                                    <div className="space-y-1.5">
                                        <label className={fieldLabelClass}>Password</label>
                                        <div className="relative">
                                            <input
                                                className={`${inputClass} pr-10`}
                                                type={auth.showLoginPassword ? "text" : "password"}
                                                value={auth.loginPassword}
                                                onChange={(event) => auth.setLoginPassword(event.target.value)}
                                                disabled={auth.isSigningIn}
                                                autoComplete="current-password"
                                                placeholder="••••••••"
                                            />
                                            <button
                                                className={passwordToggleClass}
                                                type="button"
                                                onClick={() => auth.setShowLoginPassword((v) => !v)}
                                                aria-label={auth.showLoginPassword ? "Hide password" : "Show password"}
                                            >
                                                {auth.showLoginPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                                            </button>
                                        </div>
                                    </div>

                                    <div className="space-y-3 pt-2">
                                        <button
                                            className={primaryButtonClass}
                                            type="submit"
                                            disabled={!auth.loginUsername.trim() || !auth.loginPassword.trim() || auth.isSigningIn}
                                        >
                                            {auth.isSigningIn ? <LoaderCircle className="animate-spin" size={16} /> : <LogIn className="h-4 w-4" />}
                                            {auth.isSigningIn ? "Signing in..." : "Login"}
                                        </button>

                                        <button
                                            className={secondaryButtonClass}
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
                                    </div>
                                </form>
                            ) : (
                                <form
                                    className="space-y-5"
                                    onSubmit={(event: FormEvent<HTMLFormElement>) =>
                                        void auth.handleForgotPasswordSubmit(event, setGlobalError)
                                    }
                                >
                                    <div className="space-y-1.5">
                                        <label className={fieldLabelClass}>Username</label>
                                        <input
                                            className={inputClass}
                                            value={auth.forgotUsername}
                                            onChange={(event) => auth.setForgotUsername(event.target.value)}
                                            disabled={auth.isSubmittingForgotPassword}
                                        />
                                    </div>
                                    <div className="space-y-1.5">
                                        <label className={fieldLabelClass}>Recovery Code</label>
                                        <input
                                            className={inputClass}
                                            value={auth.forgotRecoveryCode}
                                            onChange={(event) => auth.setForgotRecoveryCode(event.target.value)}
                                            disabled={auth.isSubmittingForgotPassword}
                                        />
                                    </div>
                                    <div className="space-y-1.5">
                                        <label className={fieldLabelClass}>New Password</label>
                                        <div className="relative">
                                            <input
                                                className={`${inputClass} pr-10`}
                                                type={auth.showForgotNewPassword ? "text" : "password"}
                                                value={auth.forgotNewPassword}
                                                onChange={(event) => auth.setForgotNewPassword(event.target.value)}
                                                disabled={auth.isSubmittingForgotPassword}
                                                placeholder="••••••••"
                                            />
                                            <button
                                                className={passwordToggleClass}
                                                type="button"
                                                onClick={() => auth.setShowForgotNewPassword((v) => !v)}
                                                aria-label={auth.showForgotNewPassword ? "Hide password" : "Show password"}
                                            >
                                                {auth.showForgotNewPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                                            </button>
                                        </div>
                                    </div>
                                    <div className="space-y-3 pt-2">
                                        <button
                                            className={primaryButtonClass}
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
                                            className={secondaryButtonClass}
                                            type="button"
                                            onClick={() => {
                                                auth.setForgotPasswordMode(false)
                                                auth.setShowForgotNewPassword(false)
                                                setGlobalError(null)
                                            }}
                                        >
                                            Back to login
                                        </button>
                                    </div>
                                </form>
                            )}
                        </>
                    )}
                </section>
            </main>
        </div>
    )
}
