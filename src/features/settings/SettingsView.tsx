import {
    AlertTriangle,
    CheckCircle2,
    ChevronDown,
    ChevronUp,
    Database,
    DownloadCloud,
    History,
    KeyRound,
    LoaderCircle,
    MoveRight,
    PencilLine,
    RotateCcw,
    Save,
    Trash2,
    Upload,
    Users,
} from "lucide-react"
import type { AuthState } from "../auth/useAuthState"
import type { SettingsState } from "./useSettingsState"
import type { ImportState } from "../import/useImportState"
import type { AssetImportState } from "../assets/useAssetDirectImportState"
import type { AssetDashboardState } from "../assets/useAssetDashboardState"
import type { StaffGroupKey } from "../../types/app"
import { STAFF_GROUP_BUTTONS, DEFAULT_NEW_ACCOUNT_PASSWORD } from "../../lib/constants"
import { getGroupCount } from "../../lib/utils"
import { AssetDashboard } from "../assets/AssetDashboard"
import { useIdleCollapse } from "./useIdleCollapse"

type SettingsViewProps = {
    auth: AuthState
    activeUserScope: string
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

const settingsSubCardClass = "rounded-[12px] border border-slate-800 bg-[#1c2128] p-3"
const settingsLabelClass =
    "text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-400"
const settingsInputClass =
    "w-full rounded-[10px] border border-slate-800 bg-[#0d1117] px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 outline-none transition focus:border-emerald-500/50"
const settingsSecondaryButtonClass =
    "inline-flex items-center justify-center gap-2 rounded-[10px] border border-slate-700 bg-slate-800 px-3 py-2 text-sm font-semibold text-slate-100 transition hover:bg-slate-700 disabled:opacity-50"
const settingsPrimaryButtonClass =
    "inline-flex items-center justify-center gap-2 rounded-[10px] border border-emerald-500/70 bg-emerald-500 px-3 py-2 text-sm font-semibold text-[#03130d] transition hover:bg-emerald-400 disabled:opacity-50"
const settingsMutedTextClass = "text-slate-400"
const settingsPanelHeadingClass =
    "text-sm font-semibold uppercase tracking-[0.08em] text-slate-100"
const adminPortalImportOptions: Array<{ key: StaffGroupKey; label: string }> = [
    { key: "employee_list", label: "Employee List" },
    { key: "onboarding", label: "Onboarding" },
    { key: "offboarding", label: "Offboarding" },
    { key: "internal_movement", label: "Movement" },
]
const settingsCardHeaderClass =
    "min-h-[68px] bg-[#1c2128] px-4 py-4"
const settingsCardHeaderTitleClass =
    "flex min-h-[36px] min-w-0 items-center gap-2 text-xl font-semibold text-slate-100"

export function SettingsView({
    auth,
    activeUserScope,
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
    const adminPortalCard = useIdleCollapse(60000)
    const databaseBackupCard = useIdleCollapse(60000)

    return (
        <section className="bg-[#0f141b] px-4 py-7 text-slate-300 md:px-8">
            <h2 className="text-[30px] font-bold text-slate-100">Settings</h2>
            <div className="mt-4 grid gap-4 lg:grid-cols-2">
                {/* Admin Portal — 2-column: left = user mgmt, right = import */}
                <div
                    {...adminPortalCard.bindActivityHandlers}
                    className="self-start overflow-hidden rounded-[16px] border border-slate-800 bg-[#161b22] text-sm text-slate-300 shadow-[0_18px_42px_rgba(0,0,0,0.22)]"
                >
                    <div
                        className={`cursor-pointer transition-colors hover:bg-[#222a35] ${settingsCardHeaderClass} ${adminPortalCard.isExpanded ? "border-b border-slate-800" : ""}`}
                        onClick={() => adminPortalCard.setExpanded((current) => !current)}
                    >
                        <div className="flex items-center justify-between gap-3">
                            <div className={settingsCardHeaderTitleClass}>
                                <Users size={20} className="shrink-0 text-emerald-400" />
                                <div>Admin Portal</div>
                                <span className="ml-1 inline-flex items-center justify-center rounded-[8px] border border-slate-700 bg-slate-800 p-1 text-slate-400">
                                    {adminPortalCard.isExpanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                                </span>
                            </div>
                        </div>
                    </div>

                    <div
                        className={`origin-top overflow-hidden transition-[max-height,opacity] duration-300 ease-out ${adminPortalCard.isExpanded ? "max-h-[1400px] opacity-100" : "max-h-0 opacity-0"}`}
                    >
                        <div className="space-y-4 p-4">
                            <div className="border-b border-slate-800 pb-4">
                                <div className="inline-flex max-w-full flex-wrap overflow-hidden rounded-[10px] border border-emerald-500/45 bg-[#0d1117] shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
                                    <button
                                        className="inline-flex items-center gap-2 border-r border-emerald-500/35 bg-emerald-500 px-4 py-2.5 text-sm font-semibold text-[#03130d] transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-60"
                                        onClick={() => void imp.handlePickImportFiles()}
                                        type="button"
                                        disabled={!auth.canImportData || imp.isImporting}
                                    >
                                        {imp.isImporting ? (
                                            <LoaderCircle className="animate-spin" size={15} />
                                        ) : (
                                            <Upload size={15} />
                                        )}
                                        {imp.isImporting ? "Preparing import..." : "Import"}
                                    </button>
                                    <div className="relative flex min-w-0 items-center gap-2 px-3 py-2.5">
                                        <span className="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">
                                            TO:
                                        </span>
                                        <select
                                            className="min-w-[180px] appearance-none bg-transparent pr-6 text-sm font-semibold text-slate-100 outline-none disabled:cursor-not-allowed disabled:opacity-60"
                                            value={imp.importTargetGroup}
                                            onChange={(event) =>
                                                imp.setImportTargetGroup(event.target.value as StaffGroupKey)
                                            }
                                            disabled={!auth.canImportData || imp.isImporting}
                                        >
                                            {adminPortalImportOptions.map((item) => (
                                                <option key={item.key} value={item.key} className="bg-[#1c2128] text-slate-100">
                                                    {item.label}
                                                </option>
                                            ))}
                                        </select>
                                        <ChevronDown
                                            size={14}
                                            className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-slate-500"
                                        />
                                    </div>
                                </div>
                                {!auth.canImportData && (
                                    <div className="mt-2 text-xs text-slate-500">
                                        Admin access required for import.
                                    </div>
                                )}
                            </div>

                            <div className="grid gap-4 xl:grid-cols-[minmax(0,0.92fr)_minmax(320px,1.08fr)]">
                        {/* LEFT — user management */}
                                <div className="space-y-4">
                            {/* Create account */}
                            <div className={settingsSubCardClass}>
                                <div className={`mb-2 ${settingsLabelClass}`}>
                                    Create Local User
                                </div>
                                <div className="grid gap-2 sm:grid-cols-2">
                                    <input
                                        className={settingsInputClass}
                                        placeholder="Display name..."
                                        value={auth.newAccountName}
                                        onChange={(event) => auth.setNewAccountName(event.target.value)}
                                        disabled={auth.isMutatingAccounts}
                                    />
                                    <input
                                        className={settingsInputClass}
                                        placeholder="Username..."
                                        value={auth.newAccountUsername}
                                        onChange={(event) => auth.setNewAccountUsername(event.target.value)}
                                        disabled={auth.isMutatingAccounts}
                                    />
                                    <input
                                        className={`${settingsInputClass} sm:col-span-2`}
                                        placeholder="Recovery code (optional)..."
                                        value={auth.newAccountRecoveryCode}
                                        onChange={(event) => auth.setNewAccountRecoveryCode(event.target.value)}
                                        disabled={auth.isMutatingAccounts}
                                    />
                                </div>
                                <div className={`mt-2 text-xs ${settingsMutedTextClass}`}>
                                    Default password for every new user:{" "}
                                    <span className="font-semibold text-slate-100">{DEFAULT_NEW_ACCOUNT_PASSWORD}</span>
                                </div>
                                <div className="mt-2 flex flex-wrap items-center gap-2">
                                    <select
                                        className={`${settingsInputClass} w-auto min-w-[160px]`}
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
                                        className={settingsPrimaryButtonClass}
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

                                </div>

                                <div className="overflow-hidden rounded-[12px] border border-slate-800 bg-[#1c2128]">
                                    <div className="border-b border-slate-800 px-3 py-3">
                                        <div className={settingsPanelHeadingClass}>
                                            Accounts ({auth.accounts.length})
                                        </div>
                                    </div>
                                    <div
                                        className="max-h-[360px] space-y-2 overflow-y-auto px-3 pb-3 pt-3"
                                        onScroll={adminPortalCard.notifyActivity}
                                    >
                                {auth.isLoadingAccounts && (
                                    <div className={`text-xs ${settingsMutedTextClass}`}>Loading accounts...</div>
                                )}
                                {auth.accounts.map((account) => {
                                    const isActive = auth.activeAccountId === account.id
                                    return (
                                        <div
                                            key={account.id}
                                            className={`rounded-[12px] border px-3 py-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.02)] ${isActive
                                                ? "border-emerald-500/55 bg-emerald-500/10"
                                                : "border-slate-800 bg-[#1c2128]"
                                                }`}
                                        >
                                            <div className="flex flex-wrap items-center gap-2">
                                                <div className="text-sm font-semibold text-slate-100">{account.displayName}</div>
                                                <span className="rounded-[999px] border border-slate-700 bg-slate-900/60 px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-slate-400">
                                                    {account.role}
                                                </span>
                                                {isActive && (
                                                    <span className="rounded-[999px] border border-emerald-500/45 bg-emerald-500/12 px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-emerald-300">
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
                                            <div className={`mt-1 text-[11px] ${settingsMutedTextClass}`}>
                                                Username: {account.username}
                                            </div>
                                            <div className={`mt-0.5 text-[11px] ${settingsMutedTextClass}`}>
                                                Profile ID: {account.accountKey}
                                            </div>

                                            {/* Inline edit panel */}
                                            {auth.editingAccountId === account.id && (
                                                <div className="mt-3 rounded-[12px] border border-emerald-500/35 bg-emerald-500/8 p-3">
                                                    <div className={`mb-2 ${settingsLabelClass}`}>Edit Account</div>
                                                    <div className="grid gap-2 sm:grid-cols-2">
                                                        <div>
                                                            <label className={`mb-1 block ${settingsMutedTextClass}`}>Display Name</label>
                                                            <input
                                                                className={`${settingsInputClass} text-xs`}
                                                                value={auth.editDraftName}
                                                                onChange={(e) => auth.setEditDraftName(e.target.value)}
                                                                disabled={auth.isMutatingAccounts}
                                                                autoFocus
                                                            />
                                                        </div>
                                                        <div>
                                                            <label className={`mb-1 block ${settingsMutedTextClass}`}>Username</label>
                                                            <input
                                                                className={`${settingsInputClass} text-xs`}
                                                                value={auth.editDraftUsername}
                                                                onChange={(e) => auth.setEditDraftUsername(e.target.value)}
                                                                disabled={auth.isMutatingAccounts}
                                                            />
                                                        </div>
                                                    </div>
                                                    <div className="mt-2">
                                                        <label className={`mb-1 block ${settingsMutedTextClass}`}>Role</label>
                                                        {account.isSuperAdmin ? (
                                                            <div className="rounded-[10px] border border-slate-700 bg-[#0d1117] px-2 py-1.5 text-xs text-slate-400">
                                                                🔑 Super Admin (locked — only adman)
                                                            </div>
                                                        ) : (
                                                            <select
                                                                className={`${settingsInputClass} text-xs`}
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
                                                            className={`${settingsPrimaryButtonClass} px-3 py-1.5 text-xs`}
                                                            onClick={() => void auth.handleEditSave(setGlobalError, triggerReload)}
                                                            type="button"
                                                            disabled={auth.isMutatingAccounts || !auth.editDraftName.trim() || !auth.editDraftUsername.trim()}
                                                        >
                                                            {auth.isMutatingAccounts ? "Saving..." : "Save Changes"}
                                                        </button>
                                                        <button
                                                            className={`${settingsSecondaryButtonClass} px-3 py-1.5 text-xs`}
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
                        {settings.backupSettings?.retentionFiles === -999 ? (
                        <div>
                            <div className={settingsSubCardClass}>
                                {auth.canImportData ? (
                                    <>
                                        <div className={`text-xs ${settingsMutedTextClass}`}>
                                            Import target:{" "}
                                            <span className="font-semibold text-slate-100">{imp.importTargetGroupLabel}</span>
                                        </div>
                                        <button
                                            className={`mt-3 ${settingsPrimaryButtonClass}`}
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
                                    <p className={`mt-2 text-xs ${settingsMutedTextClass} opacity-60`}>
                                        🔒 Admin access required.
                                    </p>
                                )}
                            </div>

                            {/* Import target group selector — visible to all, read-only for standard users */}
                            <div className={`grid grid-cols-2 gap-2 text-sm ${settingsMutedTextClass}`}>
                                {STAFF_GROUP_BUTTONS.map((item) => {
                                    const isSelected = imp.importTargetGroup === item.key
                                    return (
                                        <button
                                            key={item.key}
                                            className={`rounded-[12px] border px-3 py-2 text-left transition ${isSelected
                                                ? "border-emerald-500/55 bg-emerald-500/10"
                                                : "border-slate-800 bg-[#1c2128] hover:border-emerald-500/35"
                                                } ${!auth.canImportData ? "pointer-events-none opacity-60" : ""}`}
                                            onClick={() => auth.canImportData && imp.setImportTargetGroup(item.key as StaffGroupKey)}
                                            type="button"
                                            disabled={!auth.canImportData}
                                        >
                                            <div className={`text-xs uppercase tracking-[0.06em] ${settingsMutedTextClass}`}>{item.label}</div>
                                            <div className="mt-1 text-[18px] font-semibold text-slate-100">
                                                {getGroupCount(employeeGroupCounts as Parameters<typeof getGroupCount>[0], item.key)}
                                            </div>
                                            {isSelected && (
                                                <div className="mt-1 text-[11px] font-semibold uppercase tracking-[0.06em] text-emerald-300">
                                                    Import Target
                                                </div>
                                            )}
                                        </button>
                                    )
                                })}
                            </div>
                        </div>
                        ) : null}
                            </div>
                        </div>
                    </div>
                </div>




                <div
                    {...databaseBackupCard.bindActivityHandlers}
                    className="self-start overflow-hidden rounded-[16px] border border-slate-800 bg-[#161b22] text-sm text-slate-300 shadow-[0_18px_42px_rgba(0,0,0,0.22)]"
                >
                    <div
                        className={`cursor-pointer transition-colors hover:bg-[#222a35] ${settingsCardHeaderClass} ${databaseBackupCard.isExpanded ? "border-b border-slate-800" : ""}`}
                        onClick={() => databaseBackupCard.setExpanded((current) => !current)}
                    >
                        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                            <div className={settingsCardHeaderTitleClass}>
                                <Database size={20} className="shrink-0 text-emerald-400" />
                                <div>Data-Backup</div>
                                <span className="ml-1 inline-flex items-center justify-center rounded-[8px] border border-slate-700 bg-slate-800 p-1 text-slate-400">
                                    {databaseBackupCard.isExpanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                                </span>
                            </div>
                            <div className="flex w-full gap-2 sm:w-auto">
                                <button
                                    className={`sm:flex-none ${settingsSecondaryButtonClass} flex-1`}
                                    onClick={(event) => {
                                        event.stopPropagation()
                                        void settings.handleRestoreFromFile()
                                    }}
                                    type="button"
                                >
                                    <RotateCcw size={15} />
                                    Restore
                                </button>
                                <button
                                    className={`sm:flex-none ${settingsPrimaryButtonClass} flex-1`}
                                    onClick={(event) => {
                                        event.stopPropagation()
                                        void settings.handleBackupNow()
                                    }}
                                    type="button"
                                    disabled={settings.isBackingUpData || settings.isSavingBackupSettings}
                                >
                                    {settings.isBackingUpData ? (
                                        <LoaderCircle className="animate-spin" size={15} />
                                    ) : (
                                        <DownloadCloud size={15} />
                                    )}
                                    {settings.isBackingUpData ? "Backing up..." : "Backup"}
                                </button>
                            </div>
                        </div>
                    </div>

                    <div
                        className={`origin-top overflow-hidden transition-[max-height,opacity] duration-300 ease-out ${databaseBackupCard.isExpanded ? "max-h-[1200px] opacity-100" : "max-h-0 opacity-0"}`}
                    >
                        <div className="grid gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_minmax(320px,0.95fr)]">
                            <div className="space-y-5">
                                <div>
                                    <div className={`mb-1.5 ${settingsLabelClass}`}>
                                        Database Path
                                    </div>
                                    <div className="cursor-default truncate rounded-[10px] border border-slate-800 bg-[#0d1117] px-3 py-2 text-sm font-medium text-slate-100">
                                        {dbStatus?.dbPath || "-"}
                                    </div>
                                </div>

                                <div className="space-y-3 rounded-[12px] border border-slate-800 bg-[#1c2128] p-3">
                                    <div className="flex flex-wrap items-center justify-between gap-2">
                                        <div className={settingsPanelHeadingClass}>
                                            DB Location
                                        </div>
                                        <span className="inline-flex items-center gap-1 rounded-[999px] border border-slate-700 bg-slate-900/70 px-2 py-0.5 text-[10px] font-semibold text-slate-300">
                                            <AlertTriangle size={12} />
                                            Only one person should edit
                                        </span>
                                    </div>
                                    <div className="flex flex-col gap-2 sm:flex-row">
                                        <input
                                            className={`${settingsInputClass} min-w-0 font-mono text-sm`}
                                            value={settings.dbCustomPathInput}
                                            onChange={(event) => settings.setDbCustomPathInput(event.target.value)}
                                            placeholder="D:\\OneDrive - Company\\StaffKit"
                                            disabled={settings.isMovingDb}
                                        />
                                        {settings.dbMovePending ? (
                                            <div className="flex gap-2">
                                                <button
                                                    className="inline-flex items-center gap-1 rounded-[10px] border border-emerald-500/45 bg-emerald-500/12 px-3 py-2 text-sm font-semibold text-emerald-300 transition hover:bg-emerald-500/20 disabled:opacity-50"
                                                    onClick={() => void settings.handleMoveDatabase()}
                                                    type="button"
                                                    disabled={settings.isMovingDb}
                                                >
                                                    <MoveRight size={15} />
                                                    {settings.isMovingDb ? "Moving..." : "Confirm Move"}
                                                </button>
                                                <button
                                                    className={settingsSecondaryButtonClass}
                                                    onClick={settings.handleMoveDatabaseCancel}
                                                    type="button"
                                                >
                                                    Cancel
                                                </button>
                                            </div>
                                        ) : (
                                            <button
                                                className={settingsSecondaryButtonClass}
                                                onClick={() => void settings.handleMoveDatabase()}
                                                type="button"
                                                disabled={settings.isMovingDb || !settings.dbCustomPathInput.trim()}
                                            >
                                                <MoveRight size={15} />
                                                Move
                                            </button>
                                        )}
                                    </div>
                                    {settings.dbPathMessage && (
                                        <div className="rounded-[10px] border border-emerald-500/28 bg-emerald-500/8 px-3 py-2 text-xs text-emerald-100">
                                            {settings.dbPathMessage}
                                        </div>
                                    )}
                                </div>

                                <div className="space-y-3">
                                    <div className={settingsLabelClass}>
                                        Backup Settings
                                    </div>
                                    <input
                                        className={`${settingsInputClass} font-mono text-sm`}
                                        value={settings.backupDirectoryInput}
                                        onChange={(event) => settings.setBackupDirectoryInput(event.target.value)}
                                        placeholder="D:\\MGdrive\\Backupdata"
                                        disabled={settings.isSavingBackupSettings || settings.isBackingUpData}
                                    />
                                    <label className="group flex items-start gap-2 text-sm text-slate-400">
                                        <span className="relative mt-0.5 flex shrink-0 items-center justify-center">
                                            <input
                                                type="checkbox"
                                                className="peer sr-only"
                                                checked={settings.backupAutoEnabled}
                                                onChange={(event) => settings.setBackupAutoEnabled(event.target.checked)}
                                                disabled={settings.isSavingBackupSettings || settings.isBackingUpData}
                                            />
                                            <span className="flex h-4 w-4 items-center justify-center rounded-[5px] border border-slate-700 bg-[#0d1117] text-transparent transition peer-checked:border-emerald-500 peer-checked:bg-emerald-500 peer-checked:text-[#03130d]">
                                                <CheckCircle2 size={12} />
                                            </span>
                                        </span>
                                        <span className="leading-snug transition-colors group-hover:text-slate-200">
                                            7 versions and delete after 400 days
                                        </span>
                                    </label>
                                    <button
                                        className={settingsSecondaryButtonClass}
                                        onClick={() => void settings.handleSaveBackupSettings()}
                                        type="button"
                                        disabled={settings.isSavingBackupSettings || settings.isBackingUpData}
                                    >
                                        <Save size={15} />
                                        {settings.isSavingBackupSettings ? "Saving..." : "Save Settings"}
                                    </button>
                                    {settings.backupStatusMessage && (
                                        <div className="rounded-[10px] border border-emerald-500/28 bg-emerald-500/8 px-3 py-2 text-xs text-emerald-100">
                                            {settings.backupStatusMessage}
                                        </div>
                                    )}
                                </div>
                            </div>

                            <div className="overflow-hidden rounded-[12px] border border-slate-800 bg-[#1c2128]">
                                <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 px-3 py-3">
                                    <div className="flex items-center gap-2">
                                        <History size={15} className="text-slate-400" />
                                        <div className={settingsPanelHeadingClass}>
                                            Snapshot History ({settings.snapshots.length || 7})
                                        </div>
                                    </div>
                                    <button
                                        className="inline-flex items-center gap-1 rounded-[10px] border border-slate-700 bg-[#0d1117] px-2.5 py-1.5 text-xs font-semibold text-slate-100 transition hover:bg-slate-800"
                                        onClick={() => void settings.handleCreateSnapshot()}
                                        type="button"
                                    >
                                        <Save size={13} />
                                        Save Snapshot
                                    </button>
                                </div>
                                <div className="space-y-2 px-3 pb-3 pt-2">
                                    <div className="text-[11px] leading-relaxed text-slate-400">
                                        Auto Save and Restore Back
                                    </div>
                                    {settings.snapshotMessage && (
                                        <div className="rounded-[10px] border border-emerald-500/28 bg-emerald-500/8 px-3 py-2 text-xs text-emerald-100">
                                            {settings.snapshotMessage}
                                        </div>
                                    )}
                                    {settings.isLoadingSnapshots ? (
                                        <div className="py-10 text-center text-xs text-slate-400">Loading...</div>
                                    ) : settings.snapshots.length === 0 ? (
                                        <div className="py-10 text-center text-xs text-slate-400">
                                            No snapshots yet. Import or close the app to create one.
                                        </div>
                                    ) : (
                                        <div
                                            className="max-h-[300px] space-y-2 overflow-y-auto pr-1"
                                            onScroll={databaseBackupCard.notifyActivity}
                                        >
                                            {settings.snapshots.map((snap) => (
                                                <div
                                                    key={snap.filename}
                                                    className="flex items-center justify-between gap-3 rounded-[10px] border border-slate-800 bg-[#0d1117] px-3 py-2"
                                                >
                                                    <div className="min-w-0 flex-1">
                                                        <div className="truncate text-sm font-semibold text-slate-100">
                                                            {snap.timestamp}
                                                        </div>
                                                        <div className="text-[11px] text-slate-400">
                                                            {snap.label} · {snap.sizeMb.toFixed(1)} MB
                                                        </div>
                                                    </div>
                                                    <button
                                                        className="inline-flex shrink-0 items-center gap-1 rounded-[10px] border border-emerald-500/45 bg-emerald-500/10 px-3 py-1.5 text-sm font-semibold text-emerald-300 transition hover:bg-emerald-500/18 disabled:opacity-40"
                                                        onClick={() => void settings.handleRestoreSnapshot(snap.filename)}
                                                        type="button"
                                                        disabled={!!settings.isRestoringSnapshot}
                                                    >
                                                        <RotateCcw size={13} />
                                                        {settings.isRestoringSnapshot === snap.filename ? "Restoring..." : "Restore"}
                                                    </button>
                                                </div>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                {settings.backupSettings?.retentionFiles === -1 ? (
                <div className="rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4 text-sm text-[var(--text-secondary)]">
                    {/* Header */}
                    <div className="mb-3 flex items-center justify-between gap-2">
                        <div className="text-sm font-semibold text-[var(--text-primary)]">Database & Backup</div>
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
                                <span className="font-semibold text-[var(--text-primary)]">Database path:</span>
                                <div className="mt-1 break-all text-xs">{dbStatus?.dbPath || "-"}</div>
                            </div>

                            {/* Shared DB Location */}
                            <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
                                <div className="mb-1 text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                                    Shared DB Location
                                </div>
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
                                7 versions and delete after 400 days
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
                ) : null}
            </div>{/* end lg:grid-cols-2 */}

            <AssetDashboard
                activeUserScope={activeUserScope}
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

