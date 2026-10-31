import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { render, screen, waitFor } from "@testing-library/react"
import "@testing-library/jest-dom/vitest"
import App from "./App"
import { staffApi } from "./services/staff-api"
import { __resetAppVersionCacheForTesting } from "./lib/appVersion"
import * as authHook from "./features/auth/useAuthState"

vi.mock("./services/staff-api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./services/staff-api")>()
  return {
    ...actual,
    staffApi: {
      ...actual.staffApi,
      ping: vi.fn(),
      initDatabase: vi.fn().mockResolvedValue({ status: "ok" }),
      getDatabaseStatus: vi.fn().mockResolvedValue({ status: "ok" }),
      getAppVersion: vi.fn(),
      listTeams: vi.fn().mockResolvedValue([]),
      listEmployeeGroupCounts: vi.fn().mockResolvedValue({ total: 0, byGroup: {} }),
      searchEmployees: vi.fn().mockResolvedValue({ items: [], total: 0 }),
      listEmployeeColumns: vi.fn().mockResolvedValue([]),
      listAccounts: vi.fn().mockResolvedValue([]),
    },
  }
})

const storageMock: Record<string, string> = {}
const localStorageMock = {
  getItem: (key: string) => storageMock[key] ?? null,
  setItem: (key: string, value: string) => {
    storageMock[key] = value
  },
  removeItem: (key: string) => {
    delete storageMock[key]
  },
  clear: () => {
    Object.keys(storageMock).forEach((k) => delete storageMock[k])
  },
}
Object.defineProperty(window, "localStorage", { value: localStorageMock, writable: true })

describe("App shell runtime version indicator", () => {
  beforeEach(() => {
    __resetAppVersionCacheForTesting()
    vi.clearAllMocks()
    localStorage.clear()
  })

  afterEach(() => {
    __resetAppVersionCacheForTesting()
  })

  it("renders version in pre-login LoginPage header", async () => {
    vi.mocked(staffApi.getAppVersion).mockResolvedValue("2.2.1")
    vi.spyOn(authHook, "useAuthState").mockReturnValue({
      isAuthenticated: false,
      setAuthenticated: vi.fn(),
      accounts: [],
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
      accountsLoadError: null,
      isMutatingAccounts: false,
      loginUsername: "",
      setLoginUsername: vi.fn(),
      loginPassword: "",
      setLoginPassword: vi.fn(),
      showLoginPassword: false,
      setShowLoginPassword: vi.fn(),
      isSigningIn: false,
      isForgotPasswordMode: false,
      setForgotPasswordMode: vi.fn(),
      forgotUsername: "",
      setForgotUsername: vi.fn(),
      forgotRecoveryCode: "",
      setForgotRecoveryCode: vi.fn(),
      forgotNewPassword: "",
      setForgotNewPassword: vi.fn(),
      showForgotNewPassword: false,
      setShowForgotNewPassword: vi.fn(),
      isSubmittingForgotPassword: false,
      newAccountName: "",
      setNewAccountName: vi.fn(),
      newAccountUsername: "",
      setNewAccountUsername: vi.fn(),
      newAccountRecoveryCode: "",
      setNewAccountRecoveryCode: vi.fn(),
      newAccountPassword: "",
      setNewAccountPassword: vi.fn(),
      newAccountRole: "user",
      setNewAccountRole: vi.fn(),
      editingAccountId: null,
      editDraftName: "",
      setEditDraftName: vi.fn(),
      editDraftUsername: "",
      setEditDraftUsername: vi.fn(),
      editDraftRole: "user",
      setEditDraftRole: vi.fn(),
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
    } as unknown as authHook.AuthState)

    render(<App />)

    await waitFor(() => {
      expect(screen.getByTestId("app-version")).toHaveTextContent("Version 2.2.1")
    })
  })

  it("renders exact same version in in-app shell after login", async () => {
    vi.mocked(staffApi.getAppVersion).mockResolvedValue("2.2.1-internal-rc1")
    vi.spyOn(authHook, "useAuthState").mockReturnValue({
      isAuthenticated: true,
      setAuthenticated: vi.fn(),
      accounts: [],
      activeAccountId: 1,
      activeAccount: { id: 1, username: "admin", displayName: "Admin", role: "admin" },
      activeAccountName: "Admin",
      isAdminAccount: true,
      canAccessSettings: true,
      canImportData: true,
      canResetData: false,
      canEditEmployeeTable: true,
      canEditEmployeeComputerName: true,
      isLoadingAccounts: false,
      accountsLoadError: null,
      isMutatingAccounts: false,
      loginUsername: "",
      setLoginUsername: vi.fn(),
      loginPassword: "",
      setLoginPassword: vi.fn(),
      showLoginPassword: false,
      setShowLoginPassword: vi.fn(),
      isSigningIn: false,
      isForgotPasswordMode: false,
      setForgotPasswordMode: vi.fn(),
      forgotUsername: "",
      setForgotUsername: vi.fn(),
      forgotRecoveryCode: "",
      setForgotRecoveryCode: vi.fn(),
      forgotNewPassword: "",
      setForgotNewPassword: vi.fn(),
      showForgotNewPassword: false,
      setShowForgotNewPassword: vi.fn(),
      isSubmittingForgotPassword: false,
      newAccountName: "",
      setNewAccountName: vi.fn(),
      newAccountUsername: "",
      setNewAccountUsername: vi.fn(),
      newAccountRecoveryCode: "",
      setNewAccountRecoveryCode: vi.fn(),
      newAccountPassword: "",
      setNewAccountPassword: vi.fn(),
      newAccountRole: "admin",
      setNewAccountRole: vi.fn(),
      editingAccountId: null,
      editDraftName: "",
      setEditDraftName: vi.fn(),
      editDraftUsername: "",
      setEditDraftUsername: vi.fn(),
      editDraftRole: "admin",
      setEditDraftRole: vi.fn(),
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
    } as unknown as authHook.AuthState)

    render(<App />)

    await waitFor(() => {
      expect(screen.getByTestId("app-version")).toHaveTextContent("Version 2.2.1-internal-rc1")
    })
  })

  it("renders fallback 'Version unavailable' in in-app shell when reading fails", async () => {
    vi.mocked(staffApi.getAppVersion).mockRejectedValue(new Error("bridge failed"))
    vi.spyOn(authHook, "useAuthState").mockReturnValue({
      isAuthenticated: true,
      setAuthenticated: vi.fn(),
      accounts: [],
      activeAccountId: 1,
      activeAccount: { id: 1, username: "admin", displayName: "Admin", role: "admin" },
      activeAccountName: "Admin",
      isAdminAccount: true,
      canAccessSettings: true,
      canImportData: true,
      canResetData: false,
      canEditEmployeeTable: true,
      canEditEmployeeComputerName: true,
      isLoadingAccounts: false,
      accountsLoadError: null,
      isMutatingAccounts: false,
      loginUsername: "",
      setLoginUsername: vi.fn(),
      loginPassword: "",
      setLoginPassword: vi.fn(),
      showLoginPassword: false,
      setShowLoginPassword: vi.fn(),
      isSigningIn: false,
      isForgotPasswordMode: false,
      setForgotPasswordMode: vi.fn(),
      forgotUsername: "",
      setForgotUsername: vi.fn(),
      forgotRecoveryCode: "",
      setForgotRecoveryCode: vi.fn(),
      forgotNewPassword: "",
      setForgotNewPassword: vi.fn(),
      showForgotNewPassword: false,
      setShowForgotNewPassword: vi.fn(),
      isSubmittingForgotPassword: false,
      newAccountName: "",
      setNewAccountName: vi.fn(),
      newAccountUsername: "",
      setNewAccountUsername: vi.fn(),
      newAccountRecoveryCode: "",
      setNewAccountRecoveryCode: vi.fn(),
      newAccountPassword: "",
      setNewAccountPassword: vi.fn(),
      newAccountRole: "admin",
      setNewAccountRole: vi.fn(),
      editingAccountId: null,
      editDraftName: "",
      setEditDraftName: vi.fn(),
      editDraftUsername: "",
      setEditDraftUsername: vi.fn(),
      editDraftRole: "admin",
      setEditDraftRole: vi.fn(),
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
    } as unknown as authHook.AuthState)

    render(<App />)

    await waitFor(() => {
      expect(screen.getByTestId("app-version")).toHaveTextContent("Version unavailable")
    })
  })
})
