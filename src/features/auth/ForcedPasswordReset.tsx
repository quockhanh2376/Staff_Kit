import { useState, type FormEvent } from "react"
import { KeyRound, LoaderCircle, LogOut } from "lucide-react"

type ForcedPasswordResetProps = {
    displayName: string
    onSubmit: (currentPassword: string, newPassword: string, setGlobalError: (msg: string | null) => void) => Promise<void>
    onLogout: () => void
    setGlobalError: (msg: string | null) => void
    globalError: string | null
}

export function ForcedPasswordReset({
    displayName,
    onSubmit,
    onLogout,
    setGlobalError,
    globalError,
}: ForcedPasswordResetProps) {
    const [currentPassword, setCurrentPassword] = useState("")
    const [newPassword, setNewPassword] = useState("")
    const [confirmPassword, setConfirmPassword] = useState("")
    const [showCurrent, setShowCurrent] = useState(false)
    const [showNew, setShowNew] = useState(false)
    const [isSubmitting, setSubmitting] = useState(false)
    const [validationError, setValidationError] = useState<string | null>(null)

    const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault()

        // Client-side validation (the backend also enforces policy).
        if (!currentPassword.trim()) {
            setValidationError("Current password is required.")
            return
        }
        if (!newPassword.trim()) {
            setValidationError("New password is required.")
            return
        }
        if (newPassword.trim().length < 6) {
            setValidationError("New password must be at least 6 characters.")
            return
        }
        if (newPassword !== confirmPassword) {
            setValidationError("New password and confirmation do not match.")
            return
        }

        setValidationError(null)
        setGlobalError(null)
        setSubmitting(true)
        await onSubmit(currentPassword.trim(), newPassword.trim(), setGlobalError)
        setSubmitting(false)
        // If the submission succeeded, the parent clears the session and returns
        // to login. If it failed, we stay on this screen.
    }

    return (
        <div className="flex min-h-screen items-center justify-center bg-[#0d0d0d] px-4">
            <div className="w-full max-w-md rounded-[16px] border border-[var(--border)] bg-[var(--surface)] p-8 shadow-2xl">
                <div className="mb-6 flex items-center gap-3">
                    <div className="flex h-10 w-10 items-center justify-center rounded-[10px] bg-amber-500/15 text-amber-400">
                        <KeyRound size={20} />
                    </div>
                    <div>
                        <h1 className="text-xl font-bold text-[var(--text-primary)]">
                            Change Your Password
                        </h1>
                        <p className="text-xs text-[var(--text-secondary)]">
                            Signed in as <span className="font-semibold">{displayName}</span>
                        </p>
                    </div>
                </div>

                <p className="mb-6 rounded-[10px] border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-300">
                    You must set a new password before continuing. The default password is no longer
                    valid after this change.
                </p>

                {(validationError || globalError) && (
                    <div className="mb-4 rounded-[10px] border border-red-500/50 bg-red-500/10 px-4 py-2 text-sm text-red-300">
                        {validationError ?? globalError}
                    </div>
                )}

                <form className="space-y-4" onSubmit={handleSubmit}>
                    <div className="space-y-1.5">
                        <label className="text-xs font-semibold uppercase tracking-wide text-[var(--text-secondary)]">
                            Current Password
                        </label>
                        <div className="relative">
                            <input
                                className="h-10 w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 pr-10 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--primary)]"
                                type={showCurrent ? "text" : "password"}
                                value={currentPassword}
                                onChange={(e) => setCurrentPassword(e.target.value)}
                                disabled={isSubmitting}
                                autoComplete="current-password"
                                autoFocus
                            />
                            <button
                                type="button"
                                className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                                onClick={() => setShowCurrent((v) => !v)}
                            >
                                {showCurrent ? "Hide" : "Show"}
                            </button>
                        </div>
                    </div>

                    <div className="space-y-1.5">
                        <label className="text-xs font-semibold uppercase tracking-wide text-[var(--text-secondary)]">
                            New Password
                        </label>
                        <div className="relative">
                            <input
                                className="h-10 w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 pr-10 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--primary)]"
                                type={showNew ? "text" : "password"}
                                value={newPassword}
                                onChange={(e) => setNewPassword(e.target.value)}
                                disabled={isSubmitting}
                                autoComplete="new-password"
                            />
                            <button
                                type="button"
                                className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                                onClick={() => setShowNew((v) => !v)}
                            >
                                {showNew ? "Hide" : "Show"}
                            </button>
                        </div>
                    </div>

                    <div className="space-y-1.5">
                        <label className="text-xs font-semibold uppercase tracking-wide text-[var(--text-secondary)]">
                            Confirm New Password
                        </label>
                        <input
                            className="h-10 w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--primary)]"
                            type={showNew ? "text" : "password"}
                            value={confirmPassword}
                            onChange={(e) => setConfirmPassword(e.target.value)}
                            disabled={isSubmitting}
                            autoComplete="new-password"
                        />
                    </div>

                    <button
                        className="flex h-10 w-full items-center justify-center gap-2 rounded-[8px] bg-[var(--primary)] text-sm font-semibold text-[#00131c] transition hover:brightness-110 disabled:opacity-50"
                        type="submit"
                        disabled={isSubmitting}
                    >
                        {isSubmitting ? (
                            <>
                                <LoaderCircle className="animate-spin" size={16} />
                                Changing...
                            </>
                        ) : (
                            "Change Password"
                        )}
                    </button>
                </form>

                <button
                    className="mt-4 flex w-full items-center justify-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                    type="button"
                    onClick={onLogout}
                >
                    <LogOut size={14} />
                    Sign out
                </button>
            </div>
        </div>
    )
}
