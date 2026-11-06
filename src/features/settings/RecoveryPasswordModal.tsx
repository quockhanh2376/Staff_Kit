import { useState, useEffect } from "react"
import { AlertTriangle, Eye, EyeOff, KeyRound, LoaderCircle } from "lucide-react"

export interface RecoveryPasswordModalProps {
    isOpen: boolean
    isProcessing: boolean
    error?: string
    onConfirm: (password: string) => Promise<void> | void
    onClose: () => void
}

export function RecoveryPasswordModal({
    isOpen,
    isProcessing,
    error,
    onConfirm,
    onClose,
}: RecoveryPasswordModalProps) {
    const [password, setPassword] = useState("")
    const [showPassword, setShowPassword] = useState(false)

    useEffect(() => {
        if (!isOpen) return
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === "Escape" && !isProcessing) {
                e.preventDefault()
                setPassword("")
                setShowPassword(false)
                onClose()
            }
        }
        window.addEventListener("keydown", handleKeyDown)
        return () => window.removeEventListener("keydown", handleKeyDown)
    }, [isOpen, isProcessing, onClose])

    if (!isOpen) return null

    const handleSubmit = (e?: React.FormEvent) => {
        if (e) e.preventDefault()
        if (isProcessing || !password.trim()) return
        const submitted = password.trim()
        setPassword("")
        setShowPassword(false)
        void onConfirm(submitted)
    }

    const handleCancel = () => {
        if (isProcessing) return
        setPassword("")
        setShowPassword(false)
        onClose()
    }

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
            <div
                className="w-full max-w-md rounded-[16px] border border-[var(--border)] bg-[var(--surface)] p-6 shadow-2xl space-y-4"
                role="dialog"
                aria-modal="true"
                aria-labelledby="recovery-reauth-title"
            >
                <div className="flex items-center gap-2 border-b border-[var(--border)] pb-3">
                    <KeyRound className="text-amber-400" size={20} />
                    <h3 id="recovery-reauth-title" className="text-base font-semibold text-[var(--text-primary)]">
                        Confirm Password / Xác nhận mật khẩu
                    </h3>
                </div>

                <div className="rounded-[10px] border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-200 space-y-1">
                    <p className="font-semibold">
                        Current Super Admin password is required to generate 8 new Recovery Codes.
                    </p>
                    <p className="text-amber-300/80">
                        Vui lòng nhập mật khẩu Super Admin hiện tại để tạo 8 mã khôi phục mới.
                    </p>
                </div>

                <form onSubmit={handleSubmit} className="space-y-4">
                    <div className="space-y-1.5">
                        <label
                            htmlFor="recovery-reauth-password"
                            className="block text-xs font-medium text-[var(--text-secondary)]"
                        >
                            Super Admin Password / Mật khẩu Super Admin
                        </label>
                        <div className="relative">
                            <input
                                id="recovery-reauth-password"
                                data-testid="recovery-reauth-password-input"
                                type={showPassword ? "text" : "password"}
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                autoFocus
                                disabled={isProcessing}
                                placeholder="••••••••••••"
                                autoComplete="current-password"
                                className="w-full rounded-[8px] border border-[var(--border)] bg-[var(--bg)] pl-3 pr-10 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-secondary)] focus:outline-none focus:ring-1 focus:ring-amber-500 disabled:opacity-50"
                            />
                            <button
                                type="button"
                                data-testid="recovery-reauth-toggle-visibility"
                                aria-label={showPassword ? "Hide password" : "Show password"}
                                onClick={() => setShowPassword((prev) => !prev)}
                                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition"
                            >
                                {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                            </button>
                        </div>
                    </div>

                    {error && (
                        <div
                            data-testid="recovery-reauth-error"
                            className="flex items-start gap-2 rounded-[10px] border border-rose-500/40 bg-rose-500/10 p-3 text-xs text-rose-200"
                        >
                            <AlertTriangle size={15} className="mt-0.5 shrink-0 text-rose-400" />
                            <div className="flex-1 leading-snug">{error}</div>
                        </div>
                    )}

                    <div className="flex justify-end gap-2 pt-2 border-t border-[var(--border)]">
                        <button
                            type="button"
                            data-testid="recovery-reauth-cancel-button"
                            className="px-4 py-2 text-xs font-semibold rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)] hover:brightness-95 text-[var(--text-primary)] transition disabled:opacity-50"
                            onClick={handleCancel}
                            disabled={isProcessing}
                        >
                            Cancel / Hủy
                        </button>

                        <button
                            type="submit"
                            data-testid="recovery-reauth-confirm-button"
                            className="inline-flex items-center justify-center gap-1.5 px-4 py-2 text-xs font-semibold rounded-[8px] bg-amber-600 hover:bg-amber-500 text-white transition disabled:opacity-50 disabled:cursor-not-allowed"
                            disabled={!password.trim() || isProcessing}
                        >
                            {isProcessing ? (
                                <>
                                    <LoaderCircle className="animate-spin" size={13} />
                                    <span>Verifying...</span>
                                </>
                            ) : (
                                <span>Confirm / Xác nhận</span>
                            )}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    )
}
