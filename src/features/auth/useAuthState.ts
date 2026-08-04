import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react"
import { staffApi } from "../../services/staff-api"
import { clearSession } from "../../services/session"
import type { LocalAccountRecord } from "../../types/staff"
import type { LocalAccountRole } from "../../types/app"
import { getUserErrorMessage } from "../../lib/errorHandling"
import { DEFAULT_ACCOUNT_NAME, DEFAULT_NEW_ACCOUNT_PASSWORD } from "../../lib/constants"
import { authErrorCodeMessage, detectAuthErrorCode } from "./authErrors"
import { deriveAuthCapabilities } from "./authCapabilities"

type UseAuthStateOptions = {
    dbReady: boolean
    reloadToken: number
    onLoginSuccess: () => void
    onLogout: () => void
}

export type AuthState = ReturnType<typeof useAuthState>

export function useAuthState({
    dbReady,
    reloadToken,
    onLoginSuccess,
    onLogout,
}: UseAuthStateOptions) {
    const [isAuthenticated, setAuthenticated] = useState(false)
    // The authoritative authenticated identity — set ONLY from the verified
    // backend login result. Never derived from login hints, synthetic stubs,
    // active_local_account_id, or a previous session.
    const [authenticatedAccount, setAuthenticatedAccount] =
        useState<LocalAccountRecord | null>(null)
    const [accounts, setAccounts] = useState<LocalAccountRecord[]>([])
    const [activeAccountId, setActiveAccountId] = useState<number | null>(null)
    const [isLoadingAccounts, setLoadingAccounts] = useState(false)
    const [accountsLoadError, setAccountsLoadError] = useState<string | null>(null)
    const [isMutatingAccounts, setMutatingAccounts] = useState(false)
    // Login-form error: stays inside the Login card, NOT the global banner.
    // Cleared when the user edits username/password or retries.
    const [loginError, setLoginErrorState] = useState<string | null>(null)
    const clearLoginError = useCallback(() => setLoginErrorState(null), [])
    const setLoginUsernameClearingError = useCallback((v: string) => {
        setLoginErrorState(null)
        setLoginUsername(v)
    }, [])
    const setLoginPasswordClearingError = useCallback((v: string) => {
        setLoginErrorState(null)
        setLoginPassword(v)
    }, [])
    // Forced password reset: when true, the app renders a blocking reset screen
    // instead of normal content. Derived from the login result's forcePasswordReset.
    const [forcePasswordReset, setForcePasswordReset] = useState(false)

    // Login form state
    const [loginUsername, setLoginUsername] = useState("")
    const [loginPassword, setLoginPassword] = useState("")
    const [showLoginPassword, setShowLoginPassword] = useState(false)
    const [isSigningIn, setSigningIn] = useState(false)

    // Forgot password state
    const [isForgotPasswordMode, setForgotPasswordMode] = useState(false)
    const [forgotUsername, setForgotUsername] = useState("")
    const [forgotRecoveryCode, setForgotRecoveryCode] = useState("")
    const [forgotNewPassword, setForgotNewPassword] = useState("")
    const [showForgotNewPassword, setShowForgotNewPassword] = useState(false)
    const [isSubmittingForgotPassword, setSubmittingForgotPassword] = useState(false)

    // New account form
    const [newAccountName, setNewAccountName] = useState("")
    const [newAccountUsername, setNewAccountUsername] = useState("")
    const [newAccountRecoveryCode, setNewAccountRecoveryCode] = useState("")
    const [newAccountRole, setNewAccountRole] = useState<LocalAccountRole>("user")

    // Inline edit form state
    const [editingAccountId, setEditingAccountId] = useState<number | null>(null)
    const [editDraftName, setEditDraftName] = useState("")
    const [editDraftUsername, setEditDraftUsername] = useState("")
    const [editDraftRole, setEditDraftRole] = useState<LocalAccountRole>("user")

    // Load accounts when db is ready.
    // Pre-login: use the public listLoginAccountHints (username + displayName
    // only). Post-login: use listLocalAccounts ONLY for super_admin (the Settings
    // admin panel needs full records). Standard users never trigger this admin
    // command, avoiding AUTH_FORBIDDEN during startup.
    useEffect(() => {
        if (!dbReady) return

        // After login, the authenticated identity comes from authenticatedAccount
        // (set by handleLoginSubmit from the backend login result). The accounts
        // list is only for the Settings admin panel. Skip admin-only loading for
        // non-super_admin users entirely.
        const isSuperAdmin = authenticatedAccount?.role === "super_admin"

        let disposed = false

        void (async () => {
            try {
                setLoadingAccounts(true)
                setAccountsLoadError(null)

                if (!isAuthenticated) {
                    // Pre-login: public hints only (no roles, no ids exposed).
                    const hints = await staffApi.listLoginAccountHints()
                    if (disposed) return
                    const stubs: LocalAccountRecord[] = hints.map((hint, index) => ({
                        id: -(index + 1), // negative synthetic id so it never collides with real DB ids
                        accountKey: hint.username,
                        displayName: hint.displayName,
                        username: hint.username,
                        role: "user",
                        isSuperAdmin: false,
                        isActive: index === 0,
                        forcePasswordReset: false,
                        createdAt: "",
                        updatedAt: "",
                    }))
                    setAccounts(stubs)
                    const active = stubs[0] ?? null
                    setActiveAccountId(active?.id ?? null)
                    setLoginUsername((prev) => {
                        if (prev.trim().length > 0) return prev
                        return active?.username ?? ""
                    })
                    if (stubs.length === 0) setAuthenticated(false)
                } else if (isSuperAdmin) {
                    // Post-login super_admin: load full records for the Settings
                    // admin panel. This is the ONLY role that can call this.
                    const data = await staffApi.listLocalAccounts()
                    if (disposed) return
                    setAccounts(data)
                    const active = data.find((item) => item.isActive) ?? data[0] ?? null
                    setActiveAccountId((current) => {
                        if (current !== null && data.some((item) => item.id === current)) {
                            return current
                        }
                        return active?.id ?? null
                    })
                }
                // Non-super_admin authenticated users: do NOT call listLocalAccounts.
                // Their identity comes from authenticatedAccount (set at login).
            } catch (error) {
                // Surface a real backend failure as a user-facing error instead of
                // silently degrading to an empty-account ("No local account found")
                // state. We do NOT clear `accounts` here.
                const message = getUserErrorMessage(error)
                if (!disposed) {
                    setAccountsLoadError(message)
                    console.error("[Staff Kit] failed to load accounts:", message)
                }
            } finally {
                if (!disposed) {
                    setLoadingAccounts(false)
                }
            }
        })()

        return () => {
            disposed = true
        }
    }, [dbReady, reloadToken, isAuthenticated, authenticatedAccount?.role])

    // Auto-logout if accounts cleared (pre-login only).
    // When authenticated, the identity comes from authenticatedAccount (set by
    // login), NOT from the accounts list. The accounts list is only for the
    // super_admin Settings panel and may be empty for non-super_admin users.
    // This effect must NOT wipe authenticated state when accounts is empty
    // post-login.
    useEffect(() => {
        if (isAuthenticated || accounts.length > 0) {
            return
        }

        setActiveAccountId(null)
        setLoginUsername("")
        setLoginPassword("")
        setAuthenticated(false)
    }, [accounts, isAuthenticated])

    // The active account is the authenticated identity when logged in.
    // It comes exclusively from the backend login result — never from login
    // hints, synthetic stubs, or the accounts list (which is admin-panel-only).
    const activeAccount = useMemo(
        () =>
            isAuthenticated
                ? authenticatedAccount
                : accounts.find((item) => item.id === activeAccountId) ??
                  accounts.find((item) => item.isActive) ??
                  accounts[0] ??
                  null,
        [isAuthenticated, authenticatedAccount, accounts, activeAccountId],
    )
    const authCapabilities = useMemo(
        () => deriveAuthCapabilities({ activeAccount, isAuthenticated }),
        [activeAccount, isAuthenticated],
    )

    const activeAccountName = activeAccount?.displayName ?? DEFAULT_ACCOUNT_NAME
    const {
        isAdminAccount,
        isSuperAdminAccount,
        canAccessSettings,
        canImportData,
        canResetData,
        canEditEmployeeTable,
        canEditEmployeeComputerName,
    } = authCapabilities

    const handleLoginSubmit = async (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault()

        const username = loginUsername.trim()
        const password = loginPassword.trim()
        if (!username || !password) {
            return
        }

        try {
            setSigningIn(true)
            setLoginErrorState(null)
            const result = await staffApi.loginLocalAccount({ username, password })
            // Atomically replace ALL previous authenticated identity with the
            // verified backend result. Never derive from hints/stubs/previous state.
            setAuthenticatedAccount(result.account)
            setActiveAccountId(result.account.id)
            setAuthenticated(true)
            setLoginPassword("")
            setShowLoginPassword(false)
            // If the account requires a forced password reset, block normal app
            // access and show the reset screen.
            setForcePasswordReset(result.account.forcePasswordReset)
            if (!result.account.forcePasswordReset) {
                onLoginSuccess()
            }
        } catch (error) {
            // Map known backend invalid-credential errors to a safe, specific
            // message that stays inside the Login card. Do NOT use the global
            // error banner for expected login failures. Do not reveal whether
            // the username exists — both wrong-password and unknown-user get
            // the same message.
            const rawMessage = error instanceof Error ? error.message : String(error)
            const lowerMessage = rawMessage.toLowerCase()
            if (
                lowerMessage.includes("incorrect username or password") ||
                lowerMessage.includes("invalid username") ||
                lowerMessage.includes("password")
            ) {
                setLoginErrorState("Username or password is incorrect.")
            } else {
                setLoginErrorState(getUserErrorMessage(error))
            }
            // Preserve any existing authenticated session during a failed
            // account-switch login — do NOT clear authenticatedAccount.
        } finally {
            setSigningIn(false)
        }
    }

    const handleForgotPasswordSubmit = async (event: FormEvent<HTMLFormElement>, setGlobalError: (msg: string | null) => void) => {
        event.preventDefault()

        const username = forgotUsername.trim()
        const recoveryCode = forgotRecoveryCode.trim()
        const newPassword = forgotNewPassword.trim()
        if (!username || !recoveryCode || !newPassword) {
            return
        }

        try {
            setSubmittingForgotPassword(true)
            setGlobalError(null)
            await staffApi.forgotLocalAccountPassword({ username, recoveryCode, newPassword })
            setForgotPasswordMode(false)
            setForgotUsername(username)
            setLoginUsername(username)
            setLoginPassword("")
            setForgotRecoveryCode("")
            setForgotNewPassword("")
            setShowForgotNewPassword(false)
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setSubmittingForgotPassword(false)
        }
    }

    // ── Own password change (SEC-001 Phase D3) ─────────────────────────────────
    // The backend invalidates the current session after a successful own password
    // change, so we must proactively clear the in-memory session and return to the
    // login screen instead of letting the NEXT IPC call discover the dead session.
    // The user must NOT wait for a follow-up call to be told their session ended.
    //
    // On failure the session is preserved (the backend rejects before any mutation),
    // and the stable auth error code (if any) is surfaced with a friendly message.
    const handleChangePassword = useCallback(
        async (
            currentPassword: string,
            newPassword: string,
            setGlobalError: (msg: string | null) => void,
        ) => {
            const accountId = authenticatedAccount?.id ?? null
            if (accountId === null || accountId < 0) {
                // No real authenticated account (synthetic pre-login stubs use
                // negative ids). Guard before any IPC.
                setGlobalError("Sign in before changing your password.")
                return
            }
            try {
                setGlobalError(null)
                await staffApi.changeLocalAccountPassword({
                    accountId,
                    currentPassword,
                    newPassword,
                })
                // Success: the backend just invalidated this session. Clear the
                // in-memory token, surface the reason, and reset app state.
                clearSession()
                setGlobalError("Your password was changed. Please sign in again.")
                setAuthenticated(false)
                setForcePasswordReset(false)
                setAuthenticatedAccount(null)
                onLogout()
            } catch (error) {
                const code = detectAuthErrorCode(error)
                if (code) {
                    setGlobalError(authErrorCodeMessage(code))
                    return
                }
                // Surface known backend password-change errors with specific
                // actionable messages instead of the generic banner.
                const rawMessage = error instanceof Error ? error.message : String(error)
                if (rawMessage.includes("password must be")) {
                    setGlobalError("Password must be at least 6 characters.")
                } else if (rawMessage.includes("current password is incorrect")) {
                    setGlobalError("Current password is incorrect.")
                } else if (rawMessage.includes("was not found")) {
                    setGlobalError("Account not found. Please sign out and try again.")
                } else {
                    setGlobalError(getUserErrorMessage(error))
                }
            }
        },
        [authenticatedAccount, onLogout],
    )

    const handleLogout = useCallback(async () => {
        // Invalidate the backend session (idempotent), then clear ALL local state.
        try {
            await staffApi.logoutLocalAccount()
        } catch {
            // Logout must always succeed locally even if the IPC call fails.
        }
        clearSession()
        setAuthenticatedAccount(null)
        setAuthenticated(false)
        setForcePasswordReset(false)
        setShowLoginPassword(false)
        setShowForgotNewPassword(false)
        onLogout()
    }, [onLogout])

    const handleActivateAccount = async (
        account: { id: number; username: string },
        setGlobalError: (msg: string | null) => void,
    ) => {
        // SEC-001 Phase B: account switching is now an explicit re-login. We do
        // NOT call set_active_local_account; instead we surface the selected
        // account's username on the login form so the operator can authenticate.
        if (account.id === activeAccountId) return
        setGlobalError(null)
        // Clear ALL previous authenticated identity — no stale identity retention.
        clearSession()
        setAuthenticatedAccount(null)
        setAuthenticated(false)
        setForcePasswordReset(false)
        setLoginUsername(account.username)
        setLoginPassword("")
        setForgotPasswordMode(false)
        onLogout()
    }

    const handleStartEdit = (account: LocalAccountRecord) => {
        setEditingAccountId(account.id)
        setEditDraftName(account.displayName)
        setEditDraftUsername(account.username)
        setEditDraftRole(account.role === "admin" ? "admin" : "user")
    }

    const handleEditCancel = () => {
        setEditingAccountId(null)
    }

    const handleEditSave = async (setGlobalError: (msg: string | null) => void, triggerReload: () => void) => {
        if (editingAccountId === null) return
        const name = editDraftName.trim()
        const username = editDraftUsername.trim()
        if (!name || !username) return

        try {
            setMutatingAccounts(true)
            setGlobalError(null)
            await staffApi.updateLocalAccount({
                id: editingAccountId,
                displayName: name,
                username,
                role: editDraftRole,
            })
            setEditingAccountId(null)
            triggerReload()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setMutatingAccounts(false)
        }
    }

    const handleAdminResetPassword = async (
        account: LocalAccountRecord,
        isAdminAcc: boolean,
        activeAccId: number | null,
        setGlobalError: (msg: string | null) => void,
        triggerReload: () => void,
    ) => {
        if (!activeAccId || !isAdminAcc) {
            setGlobalError("Only admin account can reset user passwords.")
            return
        }

        const newPassword = window.prompt(`Reset password for '${account.displayName}'`, "")
        if (!newPassword) return
        if (newPassword.trim().length < 6) {
            setGlobalError("New password must be at least 6 characters.")
            return
        }
        const newRecoveryCode = window.prompt("Optional new recovery code", "") ?? ""

        try {
            setMutatingAccounts(true)
            setGlobalError(null)
            await staffApi.adminResetLocalAccountPassword({
                adminAccountId: activeAccId,
                targetAccountId: account.id,
                newPassword: newPassword.trim(),
                newRecoveryCode: newRecoveryCode.trim() || null,
            })
            triggerReload()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setMutatingAccounts(false)
        }
    }

    const handleDeleteAccount = async (account: LocalAccountRecord, setGlobalError: (msg: string | null) => void, triggerReload: () => void) => {
        const accepted = window.confirm(`Delete account '${account.displayName}'?`)
        if (!accepted) return

        try {
            setMutatingAccounts(true)
            setGlobalError(null)
            await staffApi.deleteLocalAccount(account.id)
            triggerReload()
        } catch (error) {
            // Surface the self-delete rule with a clear message; for other auth
            // codes (FORBIDDEN) use the friendly mapping; fall back to the
            // generic classifier for everything else. The session is preserved
            // (the backend rejects self-delete before any mutation).
            const code = detectAuthErrorCode(error)
            if (code) {
                setGlobalError(authErrorCodeMessage(code))
            } else {
                setGlobalError(getUserErrorMessage(error))
            }
        } finally {
            setMutatingAccounts(false)
        }
    }

    const handleCreateAccount = async (setGlobalError: (msg: string | null) => void, triggerReload: () => void) => {
        const name = newAccountName.trim()
        const username = newAccountUsername.trim()
        if (!name || !username) return

        try {
            setMutatingAccounts(true)
            setGlobalError(null)
            await staffApi.createLocalAccount({
                displayName: name,
                username,
                password: DEFAULT_NEW_ACCOUNT_PASSWORD,
                recoveryCode: newAccountRecoveryCode.trim() || null,
                role: newAccountRole,
            })
            setNewAccountName("")
            setNewAccountUsername("")
            setNewAccountRecoveryCode("")
            setNewAccountRole("user")
            triggerReload()
        } catch (error) {
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setMutatingAccounts(false)
        }
    }

    return {
        // state
        isAuthenticated,
        setAuthenticated,
        authenticatedAccount,
        accounts,
        activeAccountId,
        activeAccount,
        activeAccountName,
        forcePasswordReset,
        isAdminAccount,
        canAccessSettings,
        canImportData,
        canResetData,
        canEditEmployeeTable,
        canEditEmployeeComputerName,
        isLoadingAccounts,
        accountsLoadError,
        isMutatingAccounts,
        // login form
        loginUsername,
        setLoginUsername: setLoginUsernameClearingError,
        loginPassword,
        setLoginPassword: setLoginPasswordClearingError,
        loginError,
        clearLoginError,
        showLoginPassword,
        setShowLoginPassword,
        isSigningIn,
        // forgot password
        isForgotPasswordMode,
        setForgotPasswordMode,
        forgotUsername,
        setForgotUsername,
        forgotRecoveryCode,
        setForgotRecoveryCode,
        forgotNewPassword,
        setForgotNewPassword,
        showForgotNewPassword,
        setShowForgotNewPassword,
        isSubmittingForgotPassword,
        // new account form
        newAccountName,
        setNewAccountName,
        newAccountUsername,
        setNewAccountUsername,
        newAccountRecoveryCode,
        setNewAccountRecoveryCode,
        newAccountRole,
        setNewAccountRole,
        // inline edit form
        editingAccountId,
        editDraftName,
        setEditDraftName,
        editDraftUsername,
        setEditDraftUsername,
        editDraftRole,
        setEditDraftRole,
        // handlers
        handleLoginSubmit,
        handleForgotPasswordSubmit,
        handleChangePassword,
        handleLogout,
        handleActivateAccount,
        handleStartEdit,
        handleEditCancel,
        handleEditSave,
        handleAdminResetPassword,
        handleDeleteAccount,
        handleCreateAccount,
        // derived
        isSuperAdminAccount,
    }
}
