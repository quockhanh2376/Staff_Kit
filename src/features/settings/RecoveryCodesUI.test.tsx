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
        recoveryCodesModalOpen: false,
        setRecoveryCodesModalOpen: vi.fn(),
        pendingRecoveryBatch: null,
        recoveryStatusMessage: "",
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

    it("renders 'Generate Recovery Codes' button when user is Super Admin", () => {
        const settings = createMockSettings()
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const btn = screen.getByTestId("generate-recovery-codes-button")
        expect(btn).toBeInTheDocument()
        expect(btn).toHaveTextContent("Generate Recovery Codes")
        expect(screen.queryByText(/Only Super Admin can generate recovery codes/i)).not.toBeInTheDocument()
    })

    it("hides 'Generate Recovery Codes' button and shows notice when user is NOT Super Admin", () => {
        const settings = createMockSettings()
        const auth = createMockAuth(false)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        expect(screen.queryByTestId("generate-recovery-codes-button")).not.toBeInTheDocument()
        expect(screen.getByText(/Only Super Admin can generate recovery codes/i)).toBeInTheDocument()
    })

    it("displays 'Active Batch (8/8 codes)' badge when active batch is present", () => {
        const settings = createMockSettings({
            recoveryCodeStatus: {
                total_codes: 8,
                remaining_codes: 8,
                has_active_batch: true,
                has_legacy_code: false,
                is_default_legacy_code: false,
            },
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const badge = screen.getByTestId("recovery-batch-status-badge")
        expect(badge).toHaveTextContent("Active Batch (8/8 codes)")
        expect(screen.getByText(/8 one-time cryptographic recovery codes are active \(8 remaining\)/i)).toBeInTheDocument()
    })

    it("displays 'Not Configured (Default Legacy)' when default unconfigured code is present", () => {
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
        expect(badge).toHaveTextContent("Not Configured (Default Legacy)")
    })

    it("displays 'No Active Batch' when recovery codes are uninitialized", () => {
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
        expect(badge).toHaveTextContent("No Active Batch")
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

    it("displays recoveryStatusMessage when present", () => {
        const settings = createMockSettings({
            recoveryStatusMessage: "8 new Recovery Codes successfully activated.",
        })
        const auth = createMockAuth(true)

        render(<SettingsView {...baseProps} auth={auth} settings={settings} />)

        const msg = screen.getByTestId("recovery-status-message")
        expect(msg).toHaveTextContent("8 new Recovery Codes successfully activated.")
    })
})
