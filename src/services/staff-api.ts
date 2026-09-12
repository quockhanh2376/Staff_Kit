import { invoke } from "@tauri-apps/api/core"
import {
  AUTH_REQUIRED,
  AUTH_SESSION_EXPIRED,
  detectAuthErrorCode,
} from "../features/auth/authErrors"
import {
  clearSession,
  getSessionToken,
  notifyUnauthorized,
  setSession,
} from "./session"
import type {
  DatabaseStatus,
  BackupRunResult,
  BackupSettings,
  BackupSettingsUpdateInput,
  BorrowLanSettings,
  BorrowLanSettingsUpdateInput,
  BorrowLanTokenStatus,
  BorrowLanServerStatus,
  AssetCategoryDetailRecord,
  AssetCategoryUpsertInput,
  AssetCategoryRecord,
  AssetDashboardQuantityRecord,
  AssetDashboardSerializedRecord,
  AssetDashboardSummary,
  SnapshotInfo,
  AssetDirectImportInput,
  AssetDirectImportPreview,
  AssetDirectImportReport,
  AssetImportBatchCreateInput,
  AssetImportBatchDetail,
  AssetImportBatchSummary,
  AssetImportCommitResult,
  AssetImportFileInspection,
  AssetImportInspectInput,
  AssetImportRowRecord,
  AssetImportRowSkipInput,
  AssetImportRowUpdateInput,
  AssetRecord,
  AssetSeedItemInput,
  StockItemQuantityUpdateInput,
  EmployeeColumnDefinition,
  EmployeeGroupCounts,
  EmployeeAssetSeedInput,
  EmployeeAssetSeedPreview,
  EmployeeAssetSeedReport,
  EmployeeColumnUpsertInput,
  EmployeeListResponse,
  EmployeePayload,
  EmployeeQueryInput,
  EmployeeRecord,
  MoveEmployeesGroupInput,
  LocalAccountLoginInput,
  LocalAccountLoginResult,
  LoginAccountHint,
  LocalAccountCreateInput,
  LocalAccountRecord,
  LocalForgotPasswordInput,
  LocalPasswordChangeInput,
  LocalPasswordResetInput,
  LocalAccountUpdateInput,
  ImportColumnsPreview,
  ImportDetectionResult,
  ImportExcelInput,
  ImportReport,
  ImportPreviewResult,
  BorrowRequestRecord,
  BorrowPolicyRecord,
  BorrowRequestEvidenceRecord,
  BorrowRequestRejectInput,
  TeamRecord,
  TeamUpsertInput,
  MssqlConnectionDefaults,
  MssqlImportPreview,
  MssqlImportReport,
} from "../types/staff"

declare global {
  interface Window {
    __TAURI__?: unknown
    __TAURI_INTERNALS__?: unknown
  }
}

/** Checks if running inside a Tauri webview (desktop runtime). */
const isTauriRuntime = () =>
  typeof window !== "undefined" && (window.__TAURI__ !== undefined || window.__TAURI_INTERNALS__ !== undefined)

const ensureTauriRuntime = () => {
  if (!isTauriRuntime()) {
    throw new Error("Staff Kit data API requires Tauri runtime. Use `npm run tauri:dev`.")
  }
}

/**
 * Centralized handling of stable backend auth error codes (SEC-001 Phase B).
 * - AUTH_REQUIRED / AUTH_SESSION_EXPIRED: clear the in-memory session and notify
 *   subscribers so the app returns to login. Re-throw so callers can also react.
 * - AUTH_FORBIDDEN: the session is still valid; surface a user-facing error but
 *   do not log the user out.
 *
 * Raw backend error details are never logged; only the stable code is matched.
 */
const handleBackendError = (error: unknown): never => {
  const code = detectAuthErrorCode(error)
  if (code === AUTH_REQUIRED || code === AUTH_SESSION_EXPIRED) {
    notifyUnauthorized(code)
  }
  // Re-throw so feature callers / setGlobalError can react. The user-facing
  // message for AUTH_FORBIDDEN is handled by the existing error-handling layer.
  throw error
}

/**
 * Guarded Tauri invocation: injects the current session token for commands
 * that require authorization. Never sends an empty/absent token — if no session
 * exists, the call is rejected client-side as AUTH_REQUIRED before reaching IPC,
 * and the centralized handler notifies subscribers.
 */
const call = async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
  ensureTauriRuntime()
  const sessionToken = getSessionToken()
  // No session in memory: behave exactly as the backend would for a missing
  // token, without making an IPC round-trip. handleBackendError returns `never`.
  if (!sessionToken) return handleBackendError(new Error(AUTH_REQUIRED))
  const merged = { ...(args ?? {}), sessionToken }
  try {
    return await invoke<T>(command, merged)
  } catch (error) {
    return handleBackendError(error)
  }
}

/**
 * Public/session-exempt Tauri invocation for commands callable before login
 * (login, public reads, diagnostics). Never injects a session token.
 */
const callPublic = async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
  ensureTauriRuntime()
  try {
    return await invoke<T>(command, args)
  } catch (error) {
    return handleBackendError(error)
  }
}

/**
 * Staff Kit API facade — thin typed layer over Tauri IPC commands.
 * Each method maps 1:1 to a `#[tauri::command]` in the Rust backend.
 */
export const staffApi = {
  // ── Public / session-exempt (pre-login + diagnostics) ──────────────────────
  ping: () => callPublic<string>("ping"),

  initDatabase: () => callPublic<DatabaseStatus>("init_database"),

  getDatabaseStatus: () => callPublic<DatabaseStatus>("get_database_status"),

  // ── Guarded commands (require a session token) ─────────────────────────────
  writeExportFile: (path: string, contents: number[]) =>
    call<void>("write_export_file", { path, contents }),

  getBackupSettings: () => call<BackupSettings>("get_backup_settings"),

  updateBackupSettings: (payload: BackupSettingsUpdateInput) =>
    call<BackupSettings>("update_backup_settings", {
      payload: {
        backupDirectoryPath: payload.backupDirectoryPath,
        autoBackupEnabled: payload.autoBackupEnabled,
      },
    }),

  backupDatabaseNow: () => call<BackupRunResult>("backup_database_now"),

  // Phase C: run_auto_backup_if_due removed from IPC (internal lifecycle only).

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

  getBorrowLanSettings: () => call<BorrowLanSettings>("get_borrow_lan_settings"),

  detectBorrowLanHost: () => call<string | null>("detect_borrow_lan_host"),

  probeLanServer: (port: number) => call<boolean>("probe_lan_server", { port }),

  updateBorrowLanSettings: (payload: BorrowLanSettingsUpdateInput) =>
    call<BorrowLanSettings>("update_borrow_lan_settings", {
      payload: {
        enabled: payload.enabled ?? null,
        host: payload.host,
        port: payload.port,
      },
    }),

  getBorrowLanTokenStatus: () =>
    call<BorrowLanTokenStatus>("get_borrow_lan_token_status"),

  issueBorrowLanToken: () =>
    call<string>("issue_borrow_lan_token"),

  revokeBorrowLanToken: () =>
    call<void>("revoke_borrow_lan_token"),

  startBorrowLanServer: () =>
    call<BorrowLanServerStatus>("start_borrow_lan_server"),

  stopBorrowLanServer: () =>
    call<BorrowLanServerStatus>("stop_borrow_lan_server"),

  getBorrowLanStatus: () =>
    call<BorrowLanServerStatus>("get_borrow_lan_status"),

  detectImportFile: (filePath: string) =>
    call<ImportDetectionResult>('detect_import_file', { filePath }),

  inspectAssetImportFile: (payload: AssetImportInspectInput) =>
    call<AssetImportFileInspection>("inspect_asset_import_file", {
      payload: {
        filePath: payload.filePath,
        sheetName: payload.sheetName ?? null,
      },
    }),

  previewAssetImportFile: (payload: AssetDirectImportInput) =>
    call<AssetDirectImportPreview>("preview_asset_import_file", {
      payload: {
        importType: payload.importType,
        filePath: payload.filePath,
        sheetName: payload.sheetName ?? null,
      },
    }),

  importAssetImportFile: (payload: AssetDirectImportInput) =>
    call<AssetDirectImportReport>("import_asset_import_file", {
      payload: {
        importType: payload.importType,
        filePath: payload.filePath,
        sheetName: payload.sheetName ?? null,
      },
    }),

  createAssetImportBatch: (payload: AssetImportBatchCreateInput) =>
    call<AssetImportBatchDetail>("create_asset_import_batch", {
      payload: {
        importType: payload.importType,
        filePath: payload.filePath,
        sheetName: payload.sheetName ?? null,
        mapping: payload.mapping ?? null,
      },
    }),

  listAssetImportBatches: () =>
    call<AssetImportBatchSummary[]>("list_asset_import_batches"),

  getAssetImportBatchDetail: (batchId: number) =>
    call<AssetImportBatchDetail>("get_asset_import_batch_detail", { batchId }),

  updateAssetImportRow: (payload: AssetImportRowUpdateInput) =>
    call<AssetImportRowRecord>("update_asset_import_row", {
      payload: {
        rowId: payload.rowId,
        fieldKey: payload.fieldKey,
        value: payload.value ?? null,
      },
    }),

  setAssetImportRowSkipped: (payload: AssetImportRowSkipInput) =>
    call<AssetImportRowRecord>("set_asset_import_row_skipped", {
      payload: {
        rowId: payload.rowId,
        skipped: payload.skipped,
      },
    }),

  importAssetImportBatchValidRows: (batchId: number) =>
    call<AssetImportCommitResult>("import_asset_import_batch_valid_rows", { batchId }),

  deleteAssetImportBatch: (batchId: number) =>
    call<boolean>("delete_asset_import_batch", { batchId }),

  getAssetDashboardSummary: () =>
    call<AssetDashboardSummary>("get_asset_dashboard_summary"),

  listAssetDashboardSerialized: () =>
    call<AssetDashboardSerializedRecord[]>("list_asset_dashboard_serialized"),

  listAssetDashboardQuantity: () =>
    call<AssetDashboardQuantityRecord[]>("list_asset_dashboard_quantity"),

  updateStockItemQuantity: (payload: StockItemQuantityUpdateInput) =>
    call<AssetDashboardQuantityRecord>("update_stock_item_quantity", {
      payload: {
        stockItemId: payload.stockItemId,
        quantityOnHand: payload.quantityOnHand,
        assignedQuantity: payload.assignedQuantity,
      },
    }),

  createAssetManually: (payload: AssetSeedItemInput) =>
    call<AssetRecord>("create_asset_manually", {
      payload: {
        assetCode: payload.assetCode,
        categoryId: payload.categoryId ?? null,
        assetType: payload.assetType,
        displayName: payload.displayName,
        displayNameShort: payload.displayNameShort ?? null,
        brand: payload.brand ?? null,
        model: payload.model ?? null,
        serialNumber: payload.serialNumber ?? null,
        usageLocation: payload.usageLocation ?? null,
        warehouse: payload.warehouse ?? null,
        notes: payload.notes ?? null,
      },
    }),

  upsertAssets: (payload: AssetSeedItemInput[]) =>
    call<AssetRecord[]>("upsert_assets", {
      payload: payload.map((item) => ({
        assetCode: item.assetCode,
        categoryId: item.categoryId ?? null,
        assetType: item.assetType,
        displayName: item.displayName,
        displayNameShort: item.displayNameShort ?? null,
        brand: item.brand ?? null,
        model: item.model ?? null,
        serialNumber: item.serialNumber ?? null,
        usageLocation: item.usageLocation ?? null,
        warehouse: item.warehouse ?? null,
        notes: item.notes ?? null,
      })),
    }),

  listAssetCategories: () => call<AssetCategoryRecord[]>("list_asset_categories"),

  listAssetCategoryDetails: () =>
    call<AssetCategoryDetailRecord[]>("list_asset_category_details"),

  createAssetCategory: (payload: AssetCategoryUpsertInput) =>
    call<AssetCategoryDetailRecord>("create_asset_category", {
      payload: {
        id: null,
        categoryCode: payload.categoryCode,
        categoryName: payload.categoryName,
        trackingMode: payload.trackingMode,
        qrRequired: payload.qrRequired,
        prefixes: payload.prefixes.map((prefix) => ({
          prefixValue: prefix.prefixValue,
          isPrimary: prefix.isPrimary,
        })),
      },
    }),

  updateAssetCategory: (payload: AssetCategoryUpsertInput) =>
    call<AssetCategoryDetailRecord>("update_asset_category", {
      payload: {
        id: payload.id ?? null,
        categoryCode: payload.categoryCode,
        categoryName: payload.categoryName,
        trackingMode: payload.trackingMode,
        qrRequired: payload.qrRequired,
        prefixes: payload.prefixes.map((prefix) => ({
          prefixValue: prefix.prefixValue,
          isPrimary: prefix.isPrimary,
        })),
      },
    }),

  deactivateAssetCategory: (categoryId: number) =>
    call<AssetCategoryDetailRecord>("deactivate_asset_category", { categoryId }),

  listPendingBorrowRequests: () =>
    call<BorrowRequestRecord[]>("list_pending_borrow_requests"),

  getBorrowPolicy: () => call<BorrowPolicyRecord | null>("get_borrow_policy"),

  saveBorrowPolicy: (payload: { textEn: string; textVi: string }) =>
    call<BorrowPolicyRecord>("save_borrow_policy", {
      payload: {
        textEn: payload.textEn,
        textVi: payload.textVi,
      },
    }),

  getBorrowRequestDetail: (requestId: number) =>
    call<BorrowRequestRecord>("get_borrow_request_detail", { requestId }),

  getBorrowRequestEvidence: (requestId: number) =>
    call<BorrowRequestEvidenceRecord | null>("get_borrow_request_evidence", { requestId }),

  approveBorrowRequest: (requestId: number) =>
    call<BorrowRequestRecord>("approve_borrow_request", { requestId }),

  rejectBorrowRequest: (payload: BorrowRequestRejectInput) =>
    call<BorrowRequestRecord>("reject_borrow_request", {
      payload: {
        requestId: payload.requestId,
        note: payload.note,
      },
    }),

  cancelBorrowRequest: (requestId: number) =>
    call<BorrowRequestRecord>("cancel_borrow_request", { requestId }),

  listEmployees: (filters: EmployeeQueryInput) =>
    call<EmployeeListResponse>("list_employees", {
      filters,
    }),

  searchEmployees: (filters: EmployeeQueryInput) =>
    call<EmployeeListResponse>("search_employees", {
      filters,
    }),

  previewEmployeeAssetSeed: (payload: EmployeeAssetSeedInput) =>
    call<EmployeeAssetSeedPreview>("preview_employee_asset_seed", {
      payload: {
        snapshotId: payload.snapshotId ?? null,
        query: payload.query ?? null,
        teamName: payload.teamName ?? null,
        staffGroup: payload.staffGroup ?? null,
        startDateFrom: payload.startDateFrom ?? null,
        startDateTo: payload.startDateTo ?? null,
      },
    }),

  importEmployeeAssetSeed: (payload: EmployeeAssetSeedInput) =>
    call<EmployeeAssetSeedReport>("import_employee_asset_seed", {
      payload: {
        snapshotId: payload.snapshotId ?? null,
        query: payload.query ?? null,
        teamName: payload.teamName ?? null,
        staffGroup: payload.staffGroup ?? null,
        startDateFrom: payload.startDateFrom ?? null,
        startDateTo: payload.startDateTo ?? null,
      },
    }),

  listEmployeeGroupCounts: () => call<EmployeeGroupCounts>("list_employee_group_counts"),

  // Phase C: list_local_accounts is now super_admin-guarded. The login screen
  // uses the public listLoginAccountHints instead.
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

  loginLocalAccount: async (payload: LocalAccountLoginInput) => {
    // Login is session-exempt (it mints the session). Use callPublic.
    const result = await callPublic<LocalAccountLoginResult>("login_local_account", {
      payload: {
        username: payload.username,
        password: payload.password,
      },
    })
    // Persist the session in runtime memory only.
    setSession({ sessionToken: result.sessionToken, expiresAt: result.expiresAt })
    return result
  },

  logoutLocalAccount: async () => {
    // Logout uses the current token (if any) and is idempotent.
    const sessionToken = getSessionToken()
    if (sessionToken) {
      await callPublic<void>("logout_local_account", { sessionToken })
    }
    clearSession()
  },

  listLoginAccountHints: () =>
    callPublic<LoginAccountHint[]>("list_login_account_hints"),

  changeLocalAccountPassword: (payload: LocalPasswordChangeInput) =>
    call<boolean>("change_local_account_password", {
      payload: {
        id: payload.accountId,
        currentPassword: payload.currentPassword,
        newPassword: payload.newPassword,
      },
    }),

  adminResetLocalAccountPassword: (payload: LocalPasswordResetInput) =>
    call<boolean>("admin_reset_local_account_password", {
      payload: {
        id: payload.targetAccountId,
        newPassword: payload.newPassword,
      },
    }),

  forgotLocalAccountPassword: (payload: LocalForgotPasswordInput) =>
    callPublic<boolean>("forgot_local_account_password", {
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

  // Phase C: set_active_local_account has been removed from IPC. Account
  // switching is now explicit re-login via loginLocalAccount.

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
        sheetName: payload.sheetName ?? null,
        selectedColumnKeys: payload.selectedColumnKeys ?? null,
        targetStaffGroup: payload.targetStaffGroup ?? null,
      },
    }),

  previewImportExcel: (payload: ImportExcelInput = {}) =>
    call<ImportPreviewResult>("preview_import_excel", {
      payload: {
        filePath: payload.filePath ?? null,
        filePaths: payload.filePaths ?? null,
        sheetName: payload.sheetName ?? null,
        selectedColumnKeys: payload.selectedColumnKeys ?? null,
        targetStaffGroup: payload.targetStaffGroup ?? null,
      },
    }),

  inspectImportColumns: (payload: ImportExcelInput = {}) =>
    call<ImportColumnsPreview>("inspect_import_columns", {
      payload: {
        filePath: payload.filePath ?? null,
        filePaths: payload.filePaths ?? null,
        sheetName: payload.sheetName ?? null,
      },
    }),

  getMssqlConnectionDefaults: () =>
    call<MssqlConnectionDefaults>("get_mssql_connection_defaults"),

  testMssqlConnection: (host: string, port: number, user: string, password: string) =>
    call<boolean>("test_mssql_connection", { host, port, user, password }),

  previewMssqlStaff: (host: string, port: number, user: string, password: string, query?: string) =>
    call<MssqlImportPreview>("preview_mssql_staff", {
      host, port, user, password,
      query: query ?? null,
    }),

  importMssqlStaff: (host: string, port: number, user: string, password: string, query?: string, staffGroup?: string) =>
    call<MssqlImportReport>("import_mssql_staff", {
      host, port, user, password,
      query: query ?? null,
      staffGroup: staffGroup ?? null,
    }),
}
