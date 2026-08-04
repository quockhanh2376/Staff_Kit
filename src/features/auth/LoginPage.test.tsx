import { describe, expect, it, vi } from "vitest"
import { render, screen } from "@testing-library/react"
import "@testing-library/jest-dom/vitest"
import type { AuthState } from "./useAuthState"
import { LoginPage } from "./LoginPage"

vi.mock("../../services/staff-api", () => ({
    staffApi: { ping: vi.fn(), initDatabase: vi.fn(), getDatabaseStatus: vi.fn() },
}))

// Minimal AuthState stub — only the fields LoginPage reads.
const baseAuth = {
    isAuthenticated: false,
    setAuthenticated: () => {},
    accounts: [] as AuthState["accounts"],
    activeAccountId: null,
    activeAccount: null,
    activeAccountName: "",
    isAdminAccount: false,
    canAccessSettings: false,
    canImportData: false,
    canResetData: false,
    canEditEmployeeTable: false,
    canEditEmployeeComputerName: false,
    isLoadingAccounts: false,
    accountsLoadError: null as string | null,
    isMutatingAccounts: false,
    loginUsername: "",
    setLoginUsername: () => {},
    loginPassword: "",
    setLoginPassword: () => {},
    showLoginPassword: false,
    setShowLoginPassword: () => {},
    isSigningIn: false,
    isForgotPasswordMode: false,
    setForgotPasswordMode: () => {},
    forgotUsername: "",
    setForgotUsername: () => {},
    forgotRecoveryCode: "",
    setForgotRecoveryCode: () => {},
    forgotNewPassword: "",
    setForgotNewPassword: () => {},
    showForgotNewPassword: false,
    setShowForgotNewPassword: () => {},
    isSubmittingForgotPassword: false,
    newAccountName: "",
    setNewAccountName: () => {},
    newAccountUsername: "",
    setNewAccountUsername: () => {},
    newAccountRecoveryCode: "",
    setNewAccountRecoveryCode: () => {},
    newAccountRole: "user" as const,
    setNewAccountRole: () => {},
    editingAccountId: null,
    editDraftName: "",
    setEditDraftName: () => {},
    editDraftUsername: "",
    setEditDraftUsername: () => {},
    editDraftRole: "user" as const,
    setEditDraftRole: () => {},
    handleLoginSubmit: vi.fn(),
    handleForgotPasswordSubmit: vi.fn(),
    handleChangePassword: vi.fn(),
    handleLogout: vi.fn(),
    handleActivateAccount: vi.fn(),
    handleStartEdit: vi.fn(),
    handleEditCancel: vi.fn(),
    handleEditSave: vi.fn(),
    handleAdminResetPassword: vi.fn(),
    handleDeleteAccount: vi.fn(),
    handleCreateAccount: vi.fn(),
    isSuperAdminAccount: false,
} as unknown as AuthState

const noopSetTheme = () => {}
const setGlobalError = () => {}

function renderLogin(overrides: Partial<AuthState> & { isBootstrapping?: boolean } = {}) {
    const { isBootstrapping = false, ...authOverrides } = overrides
    return render(
        <LoginPage
            theme="dark"
            setTheme={noopSetTheme}
            auth={{ ...baseAuth, ...authOverrides }}
            isBootstrapping={isBootstrapping}
            globalError={null}
            setGlobalError={setGlobalError}
        />,
    )
}

describe("LoginPage account discovery (Phase B regression)", () => {
    it("shows 'Loading local accounts...' and NOT 'No local account found' while loading", () => {
        renderLogin({ isLoadingAccounts: true })
        expect(screen.getByText(/Loading local accounts/i)).toBeInTheDocument()
        expect(screen.queryByText(/No local account found/i)).not.toBeInTheDocument()
    })

    it("shows 'Loading local accounts...' while bootstrapping even if not yet loading accounts", () => {
        renderLogin({ isBootstrapping: true, isLoadingAccounts: false })
        expect(screen.getByText(/Loading local accounts/i)).toBeInTheDocument()
        expect(screen.queryByText(/No local account found/i)).not.toBeInTheDocument()
    })

    it("surfaces a backend load error instead of the empty-state when account load fails", () => {
        renderLogin({ isLoadingAccounts: false, accountsLoadError: "database is locked" })
        expect(screen.getByText(/Could not load accounts/i)).toBeInTheDocument()
        expect(screen.getByText(/database is locked/i)).toBeInTheDocument()
        // The misleading empty state must NOT appear alongside the error.
        expect(screen.queryByText(/No local account found/i)).not.toBeInTheDocument()
    })

    it("renders the username/password form when accounts are present", () => {
        renderLogin({
            accounts: [
                {
                    id: 1,
                    accountKey: "adman",
                    displayName: "Admin",
                    username: "adman",
                    role: "super_admin",
                    isSuperAdmin: true,
                    isActive: true,
                    forcePasswordReset: false,
                    createdAt: "",
                    updatedAt: "",
                },
            ],
        })
        // The labels render (the username/password form is shown) and the empty
        // state does NOT appear. Query by text since the labels lack htmlFor.
        expect(screen.getByText("Username")).toBeInTheDocument()
        expect(screen.getByText("Password")).toBeInTheDocument()
        expect(screen.queryByText(/No local account found/i)).not.toBeInTheDocument()
    })

    it("shows the genuine empty-state only when loaded, no error, and zero accounts", () => {
        renderLogin({ isLoadingAccounts: false, accountsLoadError: null, accounts: [] })
        expect(screen.getByText(/No local account found/i)).toBeInTheDocument()
    })
})
