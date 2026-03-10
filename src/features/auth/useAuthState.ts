import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react"
import { staffApi } from "../../services/staff-api"
import type { LocalAccountRecord } from "../../types/staff"
import type { LocalAccountRole } from "../../types/app"
import { getErrorMessage } from "../../lib/utils"
import { DEFAULT_ACCOUNT_NAME, DEFAULT_NEW_ACCOUNT_PASSWORD } from "../../lib/constants"

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
    const [accounts, setAccounts] = useState<LocalAccountRecord[]>([])
    const [activeAccountId, setActiveAccountId] = useState<number | null>(null)
    const [isLoadingAccounts, setLoadingAccounts] = useState(false)
    const [isMutatingAccounts, setMutatingAccounts] = useState(false)

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

    // Load accounts when db is ready
    useEffect(() => {
        if (!dbReady) return

        let disposed = false

        void (async () => {
            try {
                setLoadingAccounts(true)
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
                if (!isAuthenticated) {
                    setLoginUsername((prev) => {
                        if (prev.trim().length > 0) {
                            return prev
                        }
                        return active?.username ?? ""
                    })
                }

                if (data.length === 0) {
                    setAuthenticated(false)
                }
            } catch (error) {
                // propagate to caller
                console.error(getErrorMessage(error))
            } finally {
                if (!disposed) {
                    setLoadingAccounts(false)
                }
            }
        })()

        return () => {
            disposed = true
        }
    }, [dbReady, reloadToken, isAuthenticated])

    // Auto-logout if accounts cleared
    useEffect(() => {
        if (accounts.length > 0) {
            return
        }

        setActiveAccountId(null)
        setLoginUsername("")
        setLoginPassword("")
        setAuthenticated(false)
    }, [accounts])

    const activeAccount = useMemo(
        () =>
            accounts.find((item) => item.id === activeAccountId) ??
            accounts.find((item) => item.isActive) ??
            accounts[0] ??
            null,
        [accounts, activeAccountId],
    )

    const activeAccountName = activeAccount?.displayName ?? DEFAULT_ACCOUNT_NAME
    const isAdminAccount = activeAccount?.role === "admin" || activeAccount?.role === "super_admin"
    const isSuperAdminAccount = activeAccount?.isSuperAdmin === true
    const canAccessSettings = isAdminAccount
    const canImportData = isAdminAccount       // Only admins can import Excel
    const canResetData = isSuperAdminAccount   // Only super_admin (adman) can reset all data
    const canEditEmployeeTable = activeAccount !== null

    const handleLoginSubmit = async (event: FormEvent<HTMLFormElement>, setGlobalError: (msg: string | null) => void) => {
        event.preventDefault()

        const username = loginUsername.trim()
        const password = loginPassword.trim()
        if (!username || !password) {
            return
        }

        try {
            setSigningIn(true)
            setGlobalError(null)
            const account = await staffApi.loginLocalAccount({ username, password })
            setActiveAccountId(account.id)
            setAuthenticated(true)
            setLoginPassword("")
            setShowLoginPassword(false)
            onLoginSuccess()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
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
            setGlobalError(getErrorMessage(error))
        } finally {
            setSubmittingForgotPassword(false)
        }
    }

    const handleLogout = useCallback(() => {
        setAuthenticated(false)
        setShowLoginPassword(false)
        setShowForgotNewPassword(false)
        onLogout()
    }, [onLogout])

    const handleActivateAccount = async (id: number, setGlobalError: (msg: string | null) => void, triggerReload: () => void) => {
        if (id === activeAccountId) return
        try {
            setMutatingAccounts(true)
            setGlobalError(null)
            await staffApi.setActiveLocalAccount(id)
            setActiveAccountId(id)
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setMutatingAccounts(false)
        }
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
            setGlobalError(getErrorMessage(error))
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
            setGlobalError(getErrorMessage(error))
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
            setGlobalError(getErrorMessage(error))
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
            setGlobalError(getErrorMessage(error))
        } finally {
            setMutatingAccounts(false)
        }
    }

    return {
        // state
        isAuthenticated,
        setAuthenticated,
        accounts,
        activeAccountId,
        activeAccount,
        activeAccountName,
        isAdminAccount,
        canAccessSettings,
        canImportData,
        canResetData,
        canEditEmployeeTable,
        isLoadingAccounts,
        isMutatingAccounts,
        // login form
        loginUsername,
        setLoginUsername,
        loginPassword,
        setLoginPassword,
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
