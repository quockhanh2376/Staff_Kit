import { fireEvent, render, screen } from "@testing-library/react"
import "@testing-library/jest-dom/vitest"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { SettingsView } from "./SettingsView"
import type { SettingsState } from "./useSettingsState"
import type { AuthState } from "../auth/useAuthState"
import type { SecurityPolicyRecord } from "../../types/staff"
import type { ImportState } from "../import/useImportState"
import type { DataImportState } from "../import/useDataImportState"
import type { AssetImportState } from "../assets/useAssetDirectImportState"
import type { AssetDashboardState } from "../assets/useAssetDashboardState"

vi.mock("../assets/AssetDashboard", () => ({ AssetDashboard: () => <div>AssetDashboard</div> }))
vi.mock("./RemoteEeSyncSettings", () => ({ RemoteEeSyncSettings: () => <div>RemoteEeSyncSettings</div> }))
vi.mock("./AuditLogViewer", () => ({ AuditLogViewer: () => <div>AuditLogViewer</div> }))

describe("SettingsView — Recovery Codes UI (RC-Phase C1)", () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    const createMockSettings = (overrides?: Partial<SettingsState>): SettingsState => ({
        securityPolicy: { mode: "standard" } as unknown as SecurityPolicyRecord,
        isSavingSecurityPolicy: false,
        securityPolicyStatusMessage: "",
        handleSaveSecurityPolicy: vi.fn(),

        recoveryCodeStatus: {
            total_codes: 8,
            remaining_codes: 8,
            has_active_batch: true,
            has_legacy_code: false,
            is_default_legacy_code: false,
        },
        isLoadingRecoveryStatus: false,
        isGeneratingRecoveryCodes: false,
        isActivatingRecoveryCodes: false,
        isRevokingRecoveryCodes: false,
        recoveryPasswordModalOpen: false,
        setRecoveryPasswordModalOpen: vi.fn(),
        recoveryPasswordModalError: "",
        setRecoveryPasswordModalError: vi.fn(),
        handleOpenGenerateRecoveryCodesModal: vi.fn(),
        handleCloseGenerateRecoveryCodesModal: vi.fn(),
        handleConfirmGenerateRecoveryCodes: vi.fn(),
        recoveryCodesModalOpen: false,
        setRecoveryCodesModalOpen: vi.fn(),
        pendingRecoveryBatch: null,
        recoveryStatusMessage: "",
        clearRecoveryStatusMessage: vi.fn(),
        recoveryModalError: "",
        setRecoveryModalError: vi.fn(),
        loadRecoveryCodeStatus: vi.fn(),
        handleStartGenerateRecoveryCodes: vi.fn(),
        handleConfirmActivateRecoveryCodes: vi.fn(),
        handleCancelRecoveryCodes: vi.fn(),
        handleRevokeRecoveryCodes: vi.fn(),

        backupSettings: null,
        backupDirectoryInput: "",
        setBackupDirectoryInput: vi.fn(),
        backupAutoEnabled: false,
        setBackupAutoEnabled: vi.fn(),
        isSavingBackupSettings: false,
        isBackingUpData: false,
        backupStatusMessage: "",
        isResettingData: false,
        handleSaveBackupSettings: vi.fn(),
        handleBackupNow: vi.fn(),
        handleResetAllData: vi.fn(),
        usbBackupDirectoryInput: "",
        setUsbBackupDirectoryInput: vi.fn(),
        usbBackupEnabled: false,
        setUsbBackupEnabled: vi.fn(),
        isSavingUsbBackupSettings: false,
        usbBackupStatusMessage: "",
        handleBrowseUsbDirectory: vi.fn(),
        handleSaveUsbBackupSettings: vi.fn(),
        snapshots: [],
        isLoadingSnapshots: false,
        isRestoringSnapshot: false,
        dbCustomPathInput: "",
        setDbCustomPathInput: vi.fn(),
        isMovingDb: false,
        dbMovePending: null,
        dbPathMessage: "",
        handleMoveDatabase: vi.fn(),
        handleMoveDatabaseCancel: vi.fn(),
        handleCreateSnapshot: vi.fn(),
        handleRestoreSnapshot: vi.fn(),
        handleRestoreFromFile: vi.fn(),
        snapshotMessage: "",
        ...overrides,
    } as unknown as SettingsState)

    const createMockAuth = (isSuperAdmin: boolean): AuthState => ({
        isSuperAdminAccount: isSuperAdmin,
        isAdminAccount: true,
        isAuthenticated: true,
        activeAccount: { username: isSuperAdmin ? "adman" : "admduy", role: isSuperAdmin ? "super_admin" : "admin" },
        activeAccountId: isSuperAdmin ? 2 : 8,
        accounts: [],
        isLoadingAccounts: false,
        isMutatingAccounts: false,
        newAccountName: "",
        newAccountUsername: "",
        newAccountRole: "user",
        newAccountRecoveryCode: "",
        editingAccountId: null,
        editDraftName: "",
        editDraftUsername: "",
        editDraftRole: "user",
        canImportData: true,
        canResetData: isSuperAdmin,
        canAccessSettings: true,
        setNewAccountName: vi.fn(),
        setNewAccountUsername: vi.fn(),
        setNewAccountRole: vi.fn(),
        setNewAccountRecoveryCode: vi.fn(),
        setEditDraftName: vi.fn(),
        setEditDraftUsername: vi.fn(),
        setEditDraftRole: vi.fn(),
        handleCreateAccount: vi.fn(),
        handleStartEdit: vi.fn(),
        handleEditSave: vi.fn(),
        handleEditCancel: vi.fn(),
        handleDeleteAccount: vi.fn(),
        handleAdminResetPassword: vi.fn(),
    } as unknown as AuthState)

    const baseProps = {
        activeUserScope: "global",
        importState: { canImportData: true, importTargetGroupLabel: "All" } as unknown as ImportState,
        dataImport: {} as unknown as DataImportState,
        assetImport: {} as unknown as AssetImportState,
        assetDashboard: {} as unknown as AssetDashboardState,
        employeeGroupCounts: { employeeList: 0, onboarding: 0, offboarding: 0, internalMovement: 0 },
        dbStatus: null,
        setGlobalError: vi.fn(),
        triggerReload: vi.fn(),
    }

    it("renders 'Generate Recovery Codes' button when user is Super Admin and no active batch", () => {
        const settings = createMockSettings({
            recoveryCodeStatus: {
                total_codes: 0,
                remaining_codes: 0,
                has_active_batch: false,
                has_legacy_code: false,
                is_default_legacy_code: false,
            },
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const btn = screen.getByTestId("generate-recovery-codes-button")
        expect(btn).toBeInTheDocument()
        expect(btn).toHaveTextContent("Generate Recovery Codes")
        expect(screen.queryByText(/Only Super Admin can generate recovery codes/i)).not.toBeInTheDocument()
    })

    it("renders 'Generate New Codes' button when user is Super Admin and active batch exists", () => {
        const settings = createMockSettings({
            recoveryCodeStatus: {
                total_codes: 8,
                remaining_codes: 7,
                has_active_batch: true,
                has_legacy_code: false,
                is_default_legacy_code: false,
            },
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const btn = screen.getByTestId("generate-recovery-codes-button")
        expect(btn).toBeInTheDocument()
        expect(btn).toHaveTextContent("Generate New Codes")
    })

    it("hides 'Generate Recovery Codes' button and shows notice when user is NOT Super Admin", () => {
        const settings = createMockSettings()
        const auth = createMockAuth(false)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        expect(screen.queryByTestId("generate-recovery-codes-button")).not.toBeInTheDocument()
        expect(screen.getByText(/Only Super Admin can generate recovery codes/i)).toBeInTheDocument()
    })

    it("displays dynamic '7/8 remaining' badge and removes verbose paragraph when active batch is present", () => {
        const settings = createMockSettings({
            recoveryCodeStatus: {
                total_codes: 8,
                remaining_codes: 7,
                has_active_batch: true,
                has_legacy_code: false,
                is_default_legacy_code: false,
            },
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const badge = screen.getByTestId("recovery-batch-status-badge")
        expect(badge).toHaveTextContent("7/8 remaining")
        expect(screen.queryByText(/cryptographic recovery codes are active/i)).not.toBeInTheDocument()
    })

    it("displays 'Not configured' when default unconfigured code is present", () => {
        const settings = createMockSettings({
            recoveryCodeStatus: {
                total_codes: 0,
                remaining_codes: 0,
                has_active_batch: false,
                has_legacy_code: true,
                is_default_legacy_code: true,
            },
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const badge = screen.getByTestId("recovery-batch-status-badge")
        expect(badge).toHaveTextContent("Not configured")
    })

    it("displays 'Not configured' when recovery codes are uninitialized", () => {
        const settings = createMockSettings({
            recoveryCodeStatus: {
                total_codes: 0,
                remaining_codes: 0,
                has_active_batch: false,
                has_legacy_code: false,
                is_default_legacy_code: false,
            },
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const badge = screen.getByTestId("recovery-batch-status-badge")
        expect(badge).toHaveTextContent("Not configured")
    })

    it("wires button click to handleStartGenerateRecoveryCodes", () => {
        const handleStart = vi.fn()
        const settings = createMockSettings({
            handleStartGenerateRecoveryCodes: handleStart,
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const btn = screen.getByTestId("generate-recovery-codes-button")
        fireEvent.click(btn)

        expect(handleStart).toHaveBeenCalledTimes(1)
    })

    it("renders modal with 8 codes, copy utility, and requires confirmation before activation", async () => {
        const mockClipboard = { writeText: vi.fn().mockResolvedValue(undefined) }
        Object.assign(navigator, { clipboard: mockClipboard })

        const handleActivate = vi.fn()
        const handleCancel = vi.fn()
        const testCodes = [
            "A1111-B2222", "C3333-D4444", "E5555-F6666", "G7777-H8888",
            "J9999-K0000", "M1234-N5678", "P9012-Q3456", "R7890-S1234",
        ]
        const settings = createMockSettings({
            recoveryCodesModalOpen: true,
            pendingRecoveryBatch: {
                batch_id: "batch_test_123",
                codes: testCodes,
                expires_in_seconds: 900,
            },
            handleConfirmActivateRecoveryCodes: handleActivate,
            handleCancelRecoveryCodes: handleCancel,
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        // Modal title & warning
        expect(screen.getByText("One-Time Recovery Codes (Save Immediately)")).toBeInTheDocument()
        expect(screen.getByText(/Important: These codes will only be displayed once/i)).toBeInTheDocument()

        // 8 codes rendered
        for (const code of testCodes) {
            expect(screen.getByText(code)).toBeInTheDocument()
        }

        // Copy button
        const copyBtn = screen.getByText("Copy All Codes")
        fireEvent.click(copyBtn)
        expect(mockClipboard.writeText).toHaveBeenCalledWith(testCodes.join("\n"))

        // Confirm & Activate button initially disabled
        const activateBtn = screen.getByRole("button", { name: /Confirm & Activate Codes/i })
        expect(activateBtn).toBeDisabled()

        // Check confirmation box
        const checkbox = screen.getByLabelText(/safely copied, printed, or stored/i)
        expect(checkbox).not.toBeChecked()
        fireEvent.click(checkbox)
        expect(checkbox).toBeChecked()
        expect(activateBtn).toBeEnabled()

        // Click activation
        fireEvent.click(activateBtn)
        expect(handleActivate).toHaveBeenCalledTimes(1)

        // Cancel button
        const cancelBtn = screen.getByRole("button", { name: /Cancel & Discard/i })
        fireEvent.click(cancelBtn)
        expect(handleCancel).toHaveBeenCalledTimes(1)
    })

    it("displays recoveryModalError alert banner inside modal when activation fails", () => {
        const testCodes = [
            "RC01-AAAA", "RC02-BBBB", "RC03-CCCC", "RC04-DDDD",
            "RC05-EEEE", "RC06-FFFF", "RC07-GGGG", "RC08-HHHH",
        ]
        const settings = createMockSettings({
            recoveryCodesModalOpen: true,
            pendingRecoveryBatch: {
                batch_id: "batch-err-123",
                codes: testCodes,
                expires_in_seconds: 900,
            },
            recoveryModalError: "failed to revoke previous recovery codes: database locked",
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        expect(screen.getByText("failed to revoke previous recovery codes: database locked")).toBeInTheDocument()
        for (const code of testCodes) {
            expect(screen.getByText(code)).toBeInTheDocument()
        }
    })

    it("displays recoveryStatusMessage when present and allows dismissal", () => {
        const clearMsg = vi.fn()
        const settings = createMockSettings({
            recoveryStatusMessage: "8 new Recovery Codes successfully activated.",
            clearRecoveryStatusMessage: clearMsg,
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const msg = screen.getByTestId("recovery-status-message")
        expect(msg).toHaveTextContent("8 new Recovery Codes successfully activated.")

        const dismissBtn = screen.getByRole("button", { name: /Dismiss status message/i })
        fireEvent.click(dismissBtn)
        expect(clearMsg).toHaveBeenCalledTimes(1)
    })

    it("clears recoveryStatusMessage when SettingsView unmounts", () => {
        const clearMsg = vi.fn()
        const settings = createMockSettings({
            clearRecoveryStatusMessage: clearMsg,
        })
        const auth = createMockAuth(true)

        const { unmount } = render(<SettingsView {...baseProps} auth={auth} settings={settings} />)
        expect(clearMsg).not.toHaveBeenCalled()
        unmount()
        expect(clearMsg).toHaveBeenCalledTimes(1)
    })

    describe("Password Reauthentication Modal", () => {
        it("renders modal with masked input and proper bilingual titles when recoveryPasswordModalOpen is true", () => {
            const settings = createMockSettings({
                recoveryPasswordModalOpen: true,
            })
            const auth = createMockAuth(true)

            render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

            expect(screen.getByRole("heading", { name: /Confirm Password \/ Xác nhận mật khẩu/i })).toBeInTheDocument()
            expect(screen.getByText(/Current Super Admin password is required to generate 8 new Recovery Codes/i)).toBeInTheDocument()
            const input = screen.getByTestId("recovery-reauth-password-input")
            expect(input).toBeInTheDocument()
            expect(input).toHaveAttribute("type", "password")
        })

        it("toggles password visibility with eye icon", () => {
            const settings = createMockSettings({
                recoveryPasswordModalOpen: true,
            })
            const auth = createMockAuth(true)

            render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

            const input = screen.getByTestId("recovery-reauth-password-input")
            const toggleBtn = screen.getByTestId("recovery-reauth-toggle-visibility")

            expect(input).toHaveAttribute("type", "password")
            fireEvent.click(toggleBtn)
            expect(input).toHaveAttribute("type", "text")
            fireEvent.click(toggleBtn)
            expect(input).toHaveAttribute("type", "password")
        })

        it("submits password to handleConfirmGenerateRecoveryCodes on form submission", () => {
            const handleConfirm = vi.fn()
            const settings = createMockSettings({
                recoveryPasswordModalOpen: true,
                handleConfirmGenerateRecoveryCodes: handleConfirm,
            })
            const auth = createMockAuth(true)

            render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

            const input = screen.getByTestId("recovery-reauth-password-input")
            const confirmBtn = screen.getByTestId("recovery-reauth-confirm-button")

            // Initially disabled when empty
            expect(confirmBtn).toBeDisabled()

            // Fill password
            fireEvent.change(input, { target: { value: "SecretSuperPassword" } })
            expect(confirmBtn).toBeEnabled()

            fireEvent.click(confirmBtn)
            expect(handleConfirm).toHaveBeenCalledWith("SecretSuperPassword")
        })

        it("submits password on Enter key press", () => {
            const handleConfirm = vi.fn()
            const settings = createMockSettings({
                recoveryPasswordModalOpen: true,
                handleConfirmGenerateRecoveryCodes: handleConfirm,
            })
            const auth = createMockAuth(true)

            render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

            const input = screen.getByTestId("recovery-reauth-password-input")
            fireEvent.change(input, { target: { value: "SecretSuperPassword" } })
            fireEvent.submit(input.closest("form")!)

            expect(handleConfirm).toHaveBeenCalledWith("SecretSuperPassword")
        })

        it("calls handleCloseGenerateRecoveryCodesModal when cancel button is clicked", () => {
            const handleClose = vi.fn()
            const settings = createMockSettings({
                recoveryPasswordModalOpen: true,
                handleCloseGenerateRecoveryCodesModal: handleClose,
            })
            const auth = createMockAuth(true)

            render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

            const cancelBtn = screen.getByTestId("recovery-reauth-cancel-button")
            fireEvent.click(cancelBtn)

            expect(handleClose).toHaveBeenCalledTimes(1)
        })

        it("displays in-modal error alert banner when recoveryPasswordModalError is set", () => {
            const settings = createMockSettings({
                recoveryPasswordModalOpen: true,
                recoveryPasswordModalError: "Invalid password / Mật khẩu không chính xác",
            })
            const auth = createMockAuth(true)

            render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

            const errorBanner = screen.getByTestId("recovery-reauth-error")
            expect(errorBanner).toBeInTheDocument()
            expect(errorBanner).toHaveTextContent("Invalid password / Mật khẩu không chính xác")
        })

        it("disables inputs and buttons during generation processing", () => {
            const settings = createMockSettings({
                recoveryPasswordModalOpen: true,
                isGeneratingRecoveryCodes: true,
            })
            const auth = createMockAuth(true)

            render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

            const input = screen.getByTestId("recovery-reauth-password-input")
            const confirmBtn = screen.getByTestId("recovery-reauth-confirm-button")
            const cancelBtn = screen.getByTestId("recovery-reauth-cancel-button")

            expect(input).toBeDisabled()
            expect(confirmBtn).toBeDisabled()
            expect(cancelBtn).toBeDisabled()
            expect(screen.getByText("Verifying...")).toBeInTheDocument()
        })
    })
})
