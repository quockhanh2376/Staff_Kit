import { FileSpreadsheet, KeyRound, LoaderCircle, PencilLine, PlusCircle, RefreshCw, Trash2, Upload, UserRoundCheck } from "lucide-react"
import type { AuthState } from "../auth/useAuthState"
import type { SettingsState } from "./useSettingsState"
import type { ImportState } from "../import/useImportState"
import type { AssetImportState } from "../assets/useAssetImportState"
import type { AssetDashboardState } from "../assets/useAssetDashboardState"
import type { StaffGroupKey } from "../../types/app"
import { STAFF_GROUP_BUTTONS, DEFAULT_NEW_ACCOUNT_PASSWORD } from "../../lib/constants"
import { getGroupCount } from "../../lib/utils"
import {
    getAssetImportActiveBatchTitle,
    buildAssetImportBatchSummaryCountLabel,
    getAssetImportBatchSummaryEmptyStateLabel,
    getAssetImportSettingsEntryActionLabel,
    getAssetImportSettingsEntryDescription,
    getAssetImportSettingsManualActionLabel,
} from "../assets/assetImportCopy"
import { getAssetImportModeLabel } from "../assets/assetImportModeConfig"
import { getAssetImportSummaryLabel } from "../assets/assetImportStatusMeta"
import { AssetDashboard } from "../assets/AssetDashboard"

type SettingsViewProps = {
    auth: AuthState
    settings: SettingsState
    importState: ImportState
    assetImport: AssetImportState
    assetDashboard: AssetDashboardState
    employeeGroupCounts: {
        employeeList: number
        onboarding: number
        offboarding: number
        internalMovement: number
    }
    dbStatus: { initialized: boolean; dbPath: string; sqliteVersion: string } | null
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
}

export function SettingsView({
    auth,
    settings,
    importState,
    assetImport,
    assetDashboard,
    employeeGroupCounts,
    dbStatus,
    setGlobalError,
    triggerReload,
}: SettingsViewProps) {
    const imp = importState

    return (
        <section className="px-4 py-7 md:px-8">
            <h2 className="text-[30px] font-bold">Settings</h2>
            <div className="mt-4 grid gap-4 lg:grid-cols-2">
                {/* Admin Portal — 2-column: left = user mgmt, right = import */}
                <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
                    <div className="text-sm font-semibold text-[var(--text-primary)]">Admin Portal (Local Accounts)</div>
                    <p className="mt-1 text-xs text-[var(--text-secondary)]">
                        Manage local accounts in this app. Column layout is saved per account profile.
                    </p>

                    <div className="mt-4 grid gap-6 md:grid-cols-2">
                        {/* LEFT — user management */}
                        <div className="flex flex-col gap-3">
                            {/* Create account */}
                            <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                                <div className="mb-2 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                    Create Local User
                                </div>
                                <div className="grid gap-2 sm:grid-cols-2">
                                    <input
                                        className="form-input"
                                        placeholder="Display name..."
                                        value={auth.newAccountName}
                                        onChange={(event) => auth.setNewAccountName(event.target.value)}
                                        disabled={auth.isMutatingAccounts}
                                    />
                                    <input
                                        className="form-input"
                                        placeholder="Username..."
                                        value={auth.newAccountUsername}
                                        onChange={(event) => auth.setNewAccountUsername(event.target.value)}
                                        disabled={auth.isMutatingAccounts}
                                    />
                                    <input
                                        className="form-input"
                                        placeholder="Recovery code (optional)..."
                                        value={auth.newAccountRecoveryCode}
                                        onChange={(event) => auth.setNewAccountRecoveryCode(event.target.value)}
                                        disabled={auth.isMutatingAccounts}
                                    />
                                </div>
                                <div className="mt-2 text-xs text-[var(--text-secondary)]">
                                    Default password for every new user:{" "}
                                    <span className="font-semibold text-[var(--text-primary)]">{DEFAULT_NEW_ACCOUNT_PASSWORD}</span>
                                </div>
                                <div className="mt-2 flex flex-wrap items-center gap-2">
                                    <select
                                        className="rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-2 py-2 text-sm"
                                        value={auth.newAccountRole}
                                        onChange={(event) =>
                                            auth.setNewAccountRole(event.target.value === "admin" ? "admin" : "user")
                                        }
                                        disabled={auth.isMutatingAccounts}
                                    >
                                        <option value="user">Standard User</option>
                                        <option value="admin">Admin</option>
                                    </select>
                                    <button
                                        className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                                        onClick={() => void auth.handleCreateAccount(setGlobalError, triggerReload)}
                                        type="button"
                                        disabled={
                                            auth.isMutatingAccounts ||
                                            !auth.newAccountName.trim() ||
                                            !auth.newAccountUsername.trim()
                                        }
                                    >
                                        {auth.isMutatingAccounts ? "Saving..." : "Add Account"}
                                    </button>
                                </div>
                            </div>

                            {/* Account list */}
                            <div className="space-y-2">
                                {auth.isLoadingAccounts && (
                                    <div className="text-xs text-[var(--text-secondary)]">Loading accounts...</div>
                                )}
                                {auth.accounts.map((account) => {
                                    const isActive = auth.activeAccountId === account.id
                                    return (
                                        <div
                                            key={account.id}
                                            className={`rounded-[8px] border px-3 py-2 ${isActive
                                                ? "border-[var(--primary)]/60 bg-[var(--primary)]/10"
                                                : "border-[var(--border)] bg-[var(--surface-hover)]/25"
                                                }`}
                                        >
                                            <div className="flex flex-wrap items-center gap-2">
                                                <div className="text-sm font-semibold text-[var(--text-primary)]">{account.displayName}</div>
                                                <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                                    {account.role}
                                                </span>
                                                {isActive && (
                                                    <span className="rounded-[999px] border border-[var(--primary)]/45 px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--primary)]">
                                                        active
                                                    </span>
                                                )}
                                                {account.forcePasswordReset && (
                                                    <span className="rounded-[999px] border border-amber-400/50 px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-amber-300">
                                                        password reset required
                                                    </span>
                                                )}
                                                <div className="ml-auto flex items-center gap-2">
                                                    <button
                                                        className="action-icon-button"
                                                        onClick={() =>
                                                            void auth.handleActivateAccount(account.id, setGlobalError, triggerReload)
                                                        }
                                                        type="button"
                                                        disabled={auth.isMutatingAccounts || auth.isLoadingAccounts}
                                                        aria-label={`Use ${account.displayName}`}
                                                        title={`Use ${account.displayName}`}
                                                    >
                                                        <UserRoundCheck size={15} />
                                                    </button>
                                                    <button
                                                        className="action-icon-button"
                                                        onClick={() => auth.handleStartEdit(account)}
                                                        type="button"
                                                        disabled={auth.isMutatingAccounts || auth.isLoadingAccounts}
                                                        aria-label={`Edit ${account.displayName}`}
                                                        title={`Edit ${account.displayName}`}
                                                    >
                                                        <PencilLine size={15} />
                                                    </button>
                                                    <button
                                                        className="action-icon-button"
                                                        onClick={() =>
                                                            void auth.handleAdminResetPassword(
                                                                account,
                                                                auth.isAdminAccount,
                                                                auth.activeAccountId,
                                                                setGlobalError,
                                                                triggerReload,
                                                            )
                                                        }
                                                        type="button"
                                                        disabled={auth.isMutatingAccounts || auth.isLoadingAccounts}
                                                        aria-label={`Reset password for ${account.displayName}`}
                                                        title={`Reset password for ${account.displayName}`}
                                                    >
                                                        <KeyRound size={15} />
                                                    </button>
                                                    <button
                                                        className="action-icon-button action-icon-button-danger"
                                                        onClick={() =>
                                                            void auth.handleDeleteAccount(account, setGlobalError, triggerReload)
                                                        }
                                                        type="button"
                                                        disabled={auth.isMutatingAccounts || auth.isLoadingAccounts}
                                                        aria-label={`Delete ${account.displayName}`}
                                                        title={`Delete ${account.displayName}`}
                                                    >
                                                        <Trash2 size={15} />
                                                    </button>
                                                </div>
                                            </div>
                                            <div className="mt-1 text-[11px] text-[var(--text-secondary)]">
                                                Username: {account.username}
                                            </div>
                                            <div className="mt-0.5 text-[11px] text-[var(--text-secondary)]">
                                                Profile ID: {account.accountKey}
                                            </div>

                                            {/* Inline edit panel */}
                                            {auth.editingAccountId === account.id && (
                                                <div className="mt-3 rounded-[8px] border border-[var(--primary)]/40 bg-[var(--primary)]/5 p-3">
                                                    <div className="mb-2 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">Edit Account</div>
                                                    <div className="grid gap-2 sm:grid-cols-2">
                                                        <div>
                                                            <label className="mb-1 block text-[11px] text-[var(--text-secondary)]">Display Name</label>
                                                            <input
                                                                className="form-input text-xs"
                                                                value={auth.editDraftName}
                                                                onChange={(e) => auth.setEditDraftName(e.target.value)}
                                                                disabled={auth.isMutatingAccounts}
                                                                autoFocus
                                                            />
                                                        </div>
                                                        <div>
                                                            <label className="mb-1 block text-[11px] text-[var(--text-secondary)]">Username</label>
                                                            <input
                                                                className="form-input text-xs"
                                                                value={auth.editDraftUsername}
                                                                onChange={(e) => auth.setEditDraftUsername(e.target.value)}
                                                                disabled={auth.isMutatingAccounts}
                                                            />
                                                        </div>
                                                    </div>
                                                    <div className="mt-2">
                                                        <label className="mb-1 block text-[11px] text-[var(--text-secondary)]">Role</label>
                                                        {account.isSuperAdmin ? (
                                                            <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)] px-2 py-1.5 text-xs text-[var(--text-secondary)]">
                                                                🔑 Super Admin (locked — only adman)
                                                            </div>
                                                        ) : (
                                                            <select
                                                                className="rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1.5 text-xs"
                                                                value={auth.editDraftRole}
                                                                onChange={(e) =>
                                                                    auth.setEditDraftRole(e.target.value === "admin" ? "admin" : "user")
                                                                }
                                                                disabled={auth.isMutatingAccounts}
                                                            >
                                                                <option value="user">Standard User</option>
                                                                <option value="admin">Admin</option>
                                                            </select>
                                                        )}
                                                    </div>
                                                    <div className="mt-3 flex gap-2">
                                                        <button
                                                            className="rounded-[8px] bg-[var(--primary)] px-3 py-1.5 text-xs font-semibold text-[#00131c] disabled:opacity-50"
                                                            onClick={() => void auth.handleEditSave(setGlobalError, triggerReload)}
                                                            type="button"
                                                            disabled={auth.isMutatingAccounts || !auth.editDraftName.trim() || !auth.editDraftUsername.trim()}
                                                        >
                                                            {auth.isMutatingAccounts ? "Saving..." : "Save Changes"}
                                                        </button>
                                                        <button
                                                            className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"
                                                            onClick={auth.handleEditCancel}
                                                            type="button"
                                                            disabled={auth.isMutatingAccounts}
                                                        >
                                                            Cancel
                                                        </button>
                                                    </div>
                                                </div>
                                            )}
                                        </div>
                                    )
                                })}
                            </div>
                        </div>

                        {/* RIGHT — Import Excel (admin only) */}
                        <div className="flex flex-col gap-4">
                            <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                                <div className="mb-1 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                    Import Excel
                                </div>
                                {auth.canImportData ? (
                                    <>
                                        <p className="mt-1 text-xs text-[var(--text-secondary)]">
                                            Select one or multiple Excel files, then choose the columns before importing into app data.
                                        </p>
                                        <div className="mt-2 text-xs text-[var(--text-secondary)]">
                                            Import target:{" "}
                                            <span className="font-semibold text-[var(--text-primary)]">{imp.importTargetGroupLabel}</span>
                                        </div>
                                        <button
                                            className="mt-3 inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                                            onClick={() => void imp.handlePickImportFiles()}
                                            type="button"
                                            disabled={imp.isImporting}
                                        >
                                            {imp.isImporting ? (
                                                <LoaderCircle className="animate-spin" size={14} />
                                            ) : (
                                                <Upload size={14} />
                                            )}
                                            {imp.isImporting ? "Preparing import..." : "Import Excel"}
                                        </button>
                                    </>
                                ) : (
                                    <p className="mt-2 text-xs text-[var(--text-secondary)] opacity-60">
                                        🔒 Admin access required.
                                    </p>
                                )}
                            </div>

                            {/* Import target group selector — visible to all, read-only for standard users */}
                            <div className="grid grid-cols-2 gap-2 text-sm text-[var(--text-secondary)]">
                                {STAFF_GROUP_BUTTONS.map((item) => {
                                    const isSelected = imp.importTargetGroup === item.key
                                    return (
                                        <button
                                            key={item.key}
                                            className={`rounded-[10px] border px-3 py-2 text-left transition ${isSelected
                                                ? "border-[var(--primary)]/55 bg-[var(--primary)]/10"
                                                : "border-[var(--border)] bg-[var(--surface)] hover:border-[var(--primary)]/35"
                                                } ${!auth.canImportData ? "pointer-events-none opacity-60" : ""}`}
                                            onClick={() => auth.canImportData && imp.setImportTargetGroup(item.key as StaffGroupKey)}
                                            type="button"
                                            disabled={!auth.canImportData}
                                        >
                                            <div className="text-xs uppercase tracking-[0.06em]">{item.label}</div>
                                            <div className="mt-1 text-[18px] font-semibold text-[var(--text-primary)]">
                                                {getGroupCount(employeeGroupCounts as Parameters<typeof getGroupCount>[0], item.key)}
                                            </div>
                                            {isSelected && (
                                                <div className="mt-1 text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--primary)]">
                                                    Import Target
                                                </div>
                                            )}
                                        </button>
                                    )
                                })}
                            </div>
                        </div>
                    </div>
                </div>


                <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4 text-sm text-[var(--text-secondary)]">
                    <div className="text-sm font-semibold text-[var(--text-primary)]">Borrow LAN &amp; Asset Import</div>
                    <p className="mt-1 text-xs text-[var(--text-secondary)]">
                        Auto-detect the current LAN IP on this machine for the employee QR flow. The live Borrow URL updates immediately, and Save keeps it for the next app launch.
                    </p>

                    <div className="mt-4 grid gap-4 md:grid-cols-2">
                        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                            <div className="mb-2 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                Borrow LAN URL
                            </div>
                            <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr),120px]">
                                <input
                                    className="form-input text-xs"
                                    value={settings.borrowLanHostInput}
                                    onChange={(event) => settings.handleBorrowLanHostInputChange(event.target.value)}
                                    placeholder="192.168.1.25 or OFFICE-PC"
                                    disabled={!auth.isAdminAccount || settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
                                />
                                <input
                                    className="form-input text-xs"
                                    value={settings.borrowLanPortInput}
                                    onChange={(event) => settings.setBorrowLanPortInput(event.target.value)}
                                    placeholder="8787"
                                    disabled={!auth.isAdminAccount || settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
                                />
                            </div>
                            <div className="mt-2 rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-xs text-[var(--text-primary)]">
                                {settings.borrowLanUrlPreview}
                            </div>
                            {settings.borrowLanDetectionNote && (
                                <div className="mt-2 text-[11px] text-[var(--primary)]">
                                    {settings.borrowLanDetectionNote}
                                </div>
                            )}
                            <div className="mt-3 flex flex-wrap gap-2">
                                <button
                                    className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                                    onClick={() => settings.handleRefreshBorrowLanHost()}
                                    type="button"
                                    disabled={!auth.isAdminAccount || settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
                                >
                                    <RefreshCw className={settings.isDetectingBorrowLanHost ? "animate-spin" : undefined} size={14} />
                                    {settings.isDetectingBorrowLanHost ? "Detecting..." : "Refresh LAN IP"}
                                </button>
                                <button
                                    className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                                    onClick={() => void settings.handleSaveBorrowLanSettings()}
                                    type="button"
                                    disabled={!auth.isAdminAccount || settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
                                >
                                    {settings.isSavingBorrowLanSettings ? "Saving..." : "Save Borrow LAN Settings"}
                                </button>
                            </div>
                            {settings.borrowLanMessage && (
                                <div className="mt-2 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                                    {settings.borrowLanMessage}
                                </div>
                            )}
                        </div>

                        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                            <div className="mb-2 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                Asset Import Wizard
                            </div>
                            <p className="text-[11px] text-[var(--text-secondary)]">
                                {getAssetImportSettingsEntryDescription()}
                            </p>
                            <div className="mt-3 flex flex-wrap gap-2">
                                <button
                                    className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-xs font-semibold text-[#00131c] disabled:opacity-50"
                                    onClick={assetImport.openImportWizard}
                                    type="button"
                                    disabled={!auth.isAdminAccount}
                                >
                                    <FileSpreadsheet size={14} />
                                    {getAssetImportSettingsEntryActionLabel()}
                                </button>
                                <button
                                    className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                                    onClick={assetImport.openManualAssetPanel}
                                    type="button"
                                    disabled={!auth.isAdminAccount}
                                >
                                    <PlusCircle size={14} />
                                    {getAssetImportSettingsManualActionLabel()}
                                </button>
                            </div>
                            <div className="mt-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-xs text-[var(--text-primary)]">
                                {assetImport.batchSummaries.length === 0
                                    ? getAssetImportBatchSummaryEmptyStateLabel()
                                    : buildAssetImportBatchSummaryCountLabel(assetImport.batchSummaries.length)}
                            </div>
                            {assetImport.activeBatchSummary && (
                                <div className="mt-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2">
                                    <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                        {getAssetImportActiveBatchTitle()}
                                    </div>
                                    <div className="mt-2 text-xs text-[var(--text-primary)]">
                                        {assetImport.activeBatchSummary.batchKey} Â· {assetImport.activeBatchSummary.sourceFileName}
                                    </div>
                                    <div className="mt-1 text-[11px] text-[var(--text-secondary)]">
                                        {getAssetImportModeLabel(assetImport.activeBatchSummary.importType)} Â· {getAssetImportSummaryLabel("valid")} {assetImport.activeBatchSummary.validRows} Â· {getAssetImportSummaryLabel("errors")} {assetImport.activeBatchSummary.errorRows} Â· {getAssetImportSummaryLabel("imported")} {assetImport.activeBatchSummary.importedRows}
                                    </div>
                                    <button
                                        className="mt-3 rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)]"
                                        onClick={() => {
                                            const batch = assetImport.activeBatchSummary
                                            if (!batch) return
                                            void assetImport.openBatchDetail(batch.id)
                                        }}
                                        type="button"
                                    >
                                        Resume Review
                                    </button>
                                </div>
                            )}
                        </div>

                        <div className="hidden rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                            <div className="mb-2 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                Asset Seed Utility
                            </div>
                            <p className="text-[11px] text-[var(--text-secondary)]">
                                One asset per line. Format: <span className="font-semibold text-[var(--text-primary)]">assetCode|assetType|displayName|model|serialNumber|notes</span>
                            </p>
                            <textarea
                                className="form-input mt-2 min-h-[180px] resize-y text-xs"
                                value={settings.assetSeedText}
                                onChange={(event) => settings.setAssetSeedText(event.target.value)}
                                placeholder={"ASSET-001|Laptop|Dell Latitude 7440|7440|SN-001|Initial stock\nASSET-002|Mouse|Logitech M650|||"}
                                disabled={!auth.isAdminAccount || settings.isSeedingAssets}
                            />
                            <button
                                className="mt-3 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-xs font-semibold text-[#00131c] disabled:opacity-50"
                                onClick={() => void settings.handleSeedAssets()}
                                type="button"
                                disabled={!auth.isAdminAccount || settings.isSeedingAssets}
                            >
                                {settings.isSeedingAssets ? "Seeding..." : "Seed Assets"}
                            </button>
                            {settings.assetSeedMessage && (
                                <div className="mt-2 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                                    {settings.assetSeedMessage}
                                </div>
                            )}
                            {settings.seededAssets.length > 0 && (
                                <div className="mt-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2">
                                    <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                        Last Seeded Assets
                                    </div>
                                    <div className="mt-2 space-y-1 text-xs text-[var(--text-primary)]">
                                        {settings.seededAssets.slice(0, 5).map((asset) => (
                                            <div key={asset.id}>
                                                {asset.assetCode} · {asset.assetType} · {asset.displayName} · {asset.status}
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>


                {/* Database & Backup — 2-column card */}
                <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4 text-sm text-[var(--text-secondary)]">
                    {/* Header */}
                    <div className="mb-3 flex items-center justify-between gap-2">
                        <div className="text-sm font-semibold text-[var(--text-primary)]">Database &amp; Backup</div>
                        <div className="flex gap-2">
                            <button
                                className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:brightness-110 disabled:opacity-50"
                                onClick={() => void settings.handleRestoreFromFile()}
                                type="button"
                            >
                                Restore Data
                            </button>
                            <button
                                className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-xs font-semibold text-[#00131c] transition hover:brightness-110 disabled:opacity-50"
                                onClick={() => void settings.handleBackupNow()}
                                type="button"
                                disabled={settings.isBackingUpData || settings.isSavingBackupSettings}
                            >
                                {settings.isBackingUpData ? <LoaderCircle className="animate-spin" size={13} /> : null}
                                {settings.isBackingUpData ? "Backing up..." : "Backup Data"}
                            </button>
                        </div>
                    </div>

                    {/* 2-column body */}
                    <div className="grid gap-4 md:grid-cols-2">
                        {/* ── Left: DB info + Shared path + Backup settings ── */}
                        <div className="space-y-3">
                            <div>
                                <span className="font-semibold text-[var(--text-primary)]">SQLite status:</span>{" "}
                                {dbStatus?.initialized ? "initialized" : "not initialized"}
                            </div>
                            <div>
                                <span className="font-semibold text-[var(--text-primary)]">SQLite version:</span>{" "}
                                {dbStatus?.sqliteVersion || "-"}
                            </div>
                            <div>
                                <span className="font-semibold text-[var(--text-primary)]">Database path:</span>
                                <div className="mt-1 break-all text-xs">{dbStatus?.dbPath || "-"}</div>
                            </div>

                            {/* Shared DB Location */}
                            <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                                <div className="mb-1 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                    Shared DB Location
                                </div>
                                <p className="mb-2 text-[11px] text-[var(--text-secondary)]">
                                    Point to a SharePoint / OneDrive synced folder to share data across the team.
                                </p>
                                <div className="mb-2 rounded-[6px] border border-amber-400/40 bg-amber-400/10 px-2 py-1.5 text-[11px] text-amber-300">
                                    ⚠️ Only one person should edit at a time.
                                </div>
                                <input
                                    className="form-input text-xs"
                                    value={settings.dbCustomPathInput}
                                    onChange={(e) => settings.setDbCustomPathInput(e.target.value)}
                                    placeholder="D:\OneDrive - Company\StaffKit"
                                    disabled={settings.isMovingDb}
                                />
                                {settings.dbMovePending ? (
                                    <div className="mt-2 flex gap-2">
                                        <button
                                            className="rounded-[8px] border border-green-500/60 bg-green-500/15 px-3 py-1.5 text-xs font-semibold text-green-400 transition hover:bg-green-500/25 disabled:opacity-50"
                                            onClick={() => void settings.handleMoveDatabase()}
                                            type="button"
                                            disabled={settings.isMovingDb}
                                        >
                                            {settings.isMovingDb ? "Moving..." : "✅ Confirm Move"}
                                        </button>
                                        <button
                                            className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)]"
                                            onClick={settings.handleMoveDatabaseCancel}
                                            type="button"
                                        >
                                            Cancel
                                        </button>
                                    </div>
                                ) : (
                                    <button
                                        className="mt-2 rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                                        onClick={() => void settings.handleMoveDatabase()}
                                        type="button"
                                        disabled={settings.isMovingDb || !settings.dbCustomPathInput.trim()}
                                    >
                                        Move Database Here
                                    </button>
                                )}
                                {settings.dbPathMessage && (
                                    <div className="mt-2 rounded-[6px] border border-[var(--primary)]/30 bg-[var(--primary)]/8 px-2 py-1.5 text-[11px] text-[var(--text-primary)]">
                                        {settings.dbPathMessage}
                                    </div>
                                )}
                            </div>

                            {/* Backup settings */}
                            <div>
                                <span className="font-semibold text-[var(--text-primary)]">Backup path:</span>
                                <input
                                    className="form-input mt-1"
                                    value={settings.backupDirectoryInput}
                                    onChange={(e) => settings.setBackupDirectoryInput(e.target.value)}
                                    placeholder="D:\\MGdrive\\Backupdata"
                                    disabled={settings.isSavingBackupSettings || settings.isBackingUpData}
                                />
                            </div>
                            <label className="flex items-center gap-2 text-xs text-[var(--text-secondary)]">
                                <input
                                    type="checkbox"
                                    checked={settings.backupAutoEnabled}
                                    onChange={(e) => settings.setBackupAutoEnabled(e.target.checked)}
                                    disabled={settings.isSavingBackupSettings || settings.isBackingUpData}
                                />
                                Auto backup every {settings.backupSettings?.autoBackupIntervalDays ?? 7} days, keep{" "}
                                {settings.backupSettings?.retentionFiles ?? 7} files, delete after{" "}
                                {settings.backupSettings?.autoBackupRetentionDays ?? 400} days
                            </label>
                            <button
                                className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
                                onClick={() => void settings.handleSaveBackupSettings()}
                                type="button"
                                disabled={settings.isSavingBackupSettings || settings.isBackingUpData}
                            >
                                {settings.isSavingBackupSettings ? "Saving..." : "Save Backup Settings"}
                            </button>
                            {settings.backupStatusMessage && (
                                <div className="rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                                    {settings.backupStatusMessage}
                                </div>
                            )}
                        </div>

                        {/* ── Right: Snapshot History ── */}
                        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                            <div className="mb-2 flex items-center justify-between gap-2">
                                <div className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                    Snapshot History (last 7)
                                </div>
                                <button
                                    className="rounded-[6px] border border-[var(--border)] px-2 py-1 text-[11px] font-semibold text-[var(--text-primary)] hover:bg-[var(--surface-hover)]"
                                    onClick={() => void settings.handleCreateSnapshot()}
                                    type="button"
                                >
                                    + Save Snapshot
                                </button>
                            </div>
                            <p className="mb-2 text-[11px] text-[var(--text-secondary)]">
                                Auto-created before each Import and when closing the app. Click Restore to roll back.
                            </p>
                            {settings.snapshotMessage && (
                                <div className="mb-2 rounded-[6px] border border-[var(--primary)]/30 bg-[var(--primary)]/8 px-2 py-1.5 text-[11px] text-[var(--text-primary)]">
                                    {settings.snapshotMessage}
                                </div>
                            )}
                            {settings.isLoadingSnapshots ? (
                                <div className="py-4 text-center text-[11px] text-[var(--text-secondary)]">Loading...</div>
                            ) : settings.snapshots.length === 0 ? (
                                <div className="py-4 text-center text-[11px] text-[var(--text-secondary)]">
                                    No snapshots yet. Import or close the app to create one.
                                </div>
                            ) : (
                                <div className="space-y-1.5">
                                    {settings.snapshots.map((snap) => (
                                        <div
                                            key={snap.filename}
                                            className="flex items-center justify-between gap-2 rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2.5 py-1.5"
                                        >
                                            <div className="min-w-0 flex-1">
                                                <div className="text-[11px] font-semibold text-[var(--text-primary)]">{snap.timestamp}</div>
                                                <div className="text-[10px] text-[var(--text-secondary)]">
                                                    {snap.label} · {snap.sizeMb.toFixed(1)} MB
                                                </div>
                                            </div>
                                            <button
                                                className="shrink-0 rounded-[6px] border border-[var(--primary)]/50 bg-[var(--primary)]/10 px-2 py-1 text-[11px] font-semibold text-[var(--primary)] hover:bg-[var(--primary)]/20 disabled:opacity-40"
                                                onClick={() => void settings.handleRestoreSnapshot(snap.filename)}
                                                type="button"
                                                disabled={!!settings.isRestoringSnapshot}
                                            >
                                                {settings.isRestoringSnapshot === snap.filename ? "Restoring..." : "Restore"}
                                            </button>
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </div>{/* end lg:grid-cols-2 */}

            <AssetDashboard
                auth={auth}
                assetDashboard={assetDashboard}
                assetImport={assetImport}
            />



            {/* Data Reset — admin only */}
            {auth.canResetData && (
                <div className="mt-4 max-w-3xl rounded-[12px] border border-red-500/45 bg-red-500/10 p-4">
                    <div className="text-sm font-semibold text-red-300">Temporary Reset (Data Wipe)</div>
                    <p className="mt-1 text-xs text-red-200/90">
                        Delete all employees, teams, imported dynamic columns, and imported values. Use this only while preparing data.
                    </p>
                    <button
                        className="mt-3 inline-flex items-center gap-2 rounded-[8px] border border-red-400/60 bg-red-500/20 px-3 py-2 text-sm font-semibold text-red-100 disabled:opacity-50"
                        onClick={() => void settings.handleResetAllData()}
                        type="button"
                        disabled={settings.isResettingData}
                    >
                        {settings.isResettingData ? <LoaderCircle className="animate-spin" size={14} /> : null}
                        {settings.isResettingData ? "Resetting..." : "Reset All Data (Temporary)"}
                    </button>
                </div>
            )}
        </section>
    )
}

