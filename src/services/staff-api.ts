import { invoke } from "@tauri-apps/api/core"
import type {
  DatabaseStatus,
  BackupRunResult,
  BackupSettings,
  BackupSettingsUpdateInput,
  SnapshotInfo,
  EmployeeColumnDefinition,
  EmployeeGroupCounts,
  EmployeeColumnUpsertInput,
  EmployeeListResponse,
  EmployeePayload,
  EmployeeQueryInput,
  EmployeeRecord,
  MoveEmployeesGroupInput,
  LocalAccountLoginInput,
  LocalAccountCreateInput,
  LocalAccountRecord,
  LocalForgotPasswordInput,
  LocalPasswordChangeInput,
  LocalPasswordResetInput,
  LocalAccountUpdateInput,
  ImportColumnsPreview,
  ImportExcelInput,
  ImportReport,
  ImportPreviewResult,
  TeamRecord,
  TeamUpsertInput,
} from "../types/staff"

declare global {
  interface Window {
    __TAURI__?: unknown
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauriRuntime = () =>
  typeof window !== "undefined" && (window.__TAURI__ !== undefined || window.__TAURI_INTERNALS__ !== undefined)

const ensureTauriRuntime = () => {
  if (!isTauriRuntime()) {
    throw new Error("Staff Kit data API requires Tauri runtime. Use `npm run tauri:dev`.")
  }
}

const call = async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
  ensureTauriRuntime()
  return invoke<T>(command, args)
}

export const staffApi = {
  ping: () => call<string>("ping"),

  initDatabase: () => call<DatabaseStatus>("init_database"),

  getDatabaseStatus: () => call<DatabaseStatus>("get_database_status"),

  getBackupSettings: () => call<BackupSettings>("get_backup_settings"),

  updateBackupSettings: (payload: BackupSettingsUpdateInput) =>
    call<BackupSettings>("update_backup_settings", {
      payload: {
        backupDirectoryPath: payload.backupDirectoryPath,
        autoBackupEnabled: payload.autoBackupEnabled,
      },
    }),

  backupDatabaseNow: () => call<BackupRunResult>("backup_database_now"),

  runAutoBackupIfDue: () => call<BackupRunResult>("run_auto_backup_if_due"),

  listHistorySnapshots: () => call<SnapshotInfo[]>("list_history_snapshots"),

  createHistorySnapshot: (label: string) =>
    call<SnapshotInfo>("create_history_snapshot_cmd", { label }),

  restoreHistorySnapshot: (filename: string) =>
    call<void>("restore_history_snapshot", { filename }),

  moveDatabaseTo: (targetFolder: string) =>
    call<string>("move_database_to", { targetFolder }),

  getDbCustomPath: () => call<string | null>("get_db_custom_path"),

  restoreDatabaseFromFile: (sourcePath: string) =>
    call<void>("restore_database_from_file", { sourcePath }),

  listEmployees: (filters: EmployeeQueryInput) =>
    call<EmployeeListResponse>("list_employees", {
      filters,
    }),

  searchEmployees: (filters: EmployeeQueryInput) =>
    call<EmployeeListResponse>("search_employees", {
      filters,
    }),

  listEmployeeGroupCounts: () => call<EmployeeGroupCounts>("list_employee_group_counts"),

  listLocalAccounts: () => call<LocalAccountRecord[]>("list_local_accounts"),

  createLocalAccount: (payload: LocalAccountCreateInput) =>
    call<LocalAccountRecord>("create_local_account", {
      payload: {
        displayName: payload.displayName,
        username: payload.username,
        password: payload.password ?? null,
        recoveryCode: payload.recoveryCode ?? null,
        role: payload.role,
      },
    }),

  updateLocalAccount: (payload: LocalAccountUpdateInput) =>
    call<LocalAccountRecord>("update_local_account", {
      payload: {
        id: payload.id,
        displayName: payload.displayName,
        username: payload.username,
        role: payload.role,
      },
    }),

  loginLocalAccount: (payload: LocalAccountLoginInput) =>
    call<LocalAccountRecord>("login_local_account", {
      payload: {
        username: payload.username,
        password: payload.password,
      },
    }),

  changeLocalAccountPassword: (payload: LocalPasswordChangeInput) =>
    call<boolean>("change_local_account_password", {
      payload: {
        accountId: payload.accountId,
        currentPassword: payload.currentPassword,
        newPassword: payload.newPassword,
        newRecoveryCode: payload.newRecoveryCode ?? null,
      },
    }),

  adminResetLocalAccountPassword: (payload: LocalPasswordResetInput) =>
    call<boolean>("admin_reset_local_account_password", {
      payload: {
        adminAccountId: payload.adminAccountId,
        targetAccountId: payload.targetAccountId,
        newPassword: payload.newPassword,
        newRecoveryCode: payload.newRecoveryCode ?? null,
      },
    }),

  forgotLocalAccountPassword: (payload: LocalForgotPasswordInput) =>
    call<boolean>("forgot_local_account_password", {
      payload: {
        username: payload.username,
        recoveryCode: payload.recoveryCode,
        newPassword: payload.newPassword,
      },
    }),

  deleteLocalAccount: (id: number) =>
    call<boolean>("delete_local_account", {
      id,
    }),

  setActiveLocalAccount: (id: number) =>
    call<boolean>("set_active_local_account", {
      id,
    }),

  listEmployeeColumns: () => call<EmployeeColumnDefinition[]>("list_employee_columns"),

  upsertEmployeeColumn: (payload: EmployeeColumnUpsertInput) =>
    call<EmployeeColumnDefinition>("upsert_employee_column", {
      payload: {
        key: payload.key ?? null,
        label: payload.label,
      },
    }),

  deleteEmployeeColumn: (key: string) =>
    call<boolean>("delete_employee_column", {
      key,
    }),

  createEmployee: (payload: EmployeePayload) =>
    call<EmployeeRecord>("create_employee", {
      payload,
    }),

  updateEmployee: (id: number, payload: EmployeePayload) =>
    call<EmployeeRecord>("update_employee", {
      id,
      payload,
    }),

  moveEmployeesGroup: (payload: MoveEmployeesGroupInput) =>
    call<number>("move_employees_group", {
      payload: {
        employeeIds: payload.employeeIds,
        targetStaffGroup: payload.targetStaffGroup,
      },
    }),

  deleteEmployee: (id: number) =>
    call<boolean>("delete_employee", {
      id,
    }),

  listTeams: () => call<TeamRecord[]>("list_teams"),

  upsertTeam: (payload: TeamUpsertInput) =>
    call<TeamRecord>("upsert_team", {
      payload,
    }),

  deleteTeam: (id: number) =>
    call<boolean>("delete_team", {
      id,
    }),

  resetAllData: () => call<boolean>("reset_all_data"),

  importExcel: (payload: ImportExcelInput = {}) =>
    call<ImportReport>("import_excel", {
      payload: {
        filePath: payload.filePath ?? null,
        filePaths: payload.filePaths ?? null,
        selectedColumnKeys: payload.selectedColumnKeys ?? null,
        targetStaffGroup: payload.targetStaffGroup ?? null,
      },
    }),

  previewImportExcel: (payload: ImportExcelInput = {}) =>
    call<ImportPreviewResult>("preview_import_excel", {
      payload: {
        filePath: payload.filePath ?? null,
        filePaths: payload.filePaths ?? null,
        selectedColumnKeys: payload.selectedColumnKeys ?? null,
        targetStaffGroup: payload.targetStaffGroup ?? null,
      },
    }),

  inspectImportColumns: (payload: ImportExcelInput = {}) =>
    call<ImportColumnsPreview>("inspect_import_columns", {
      payload: {
        filePath: payload.filePath ?? null,
        filePaths: payload.filePaths ?? null,
      },
    }),
}
