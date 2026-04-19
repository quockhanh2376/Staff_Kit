import { invoke } from "@tauri-apps/api/core"
import type {
  DatabaseStatus,
  BackupRunResult,
  BackupSettings,
  BackupSettingsUpdateInput,
  BorrowLanSettings,
  BorrowLanSettingsUpdateInput,
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
  BorrowRequestRecord,
  BorrowRequestRejectInput,
  TeamRecord,
  TeamUpsertInput,
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
 * Type-safe wrapper around Tauri's `invoke`.
 * Ensures runtime check and consistent argument passing for all Tauri commands.
 */
const call = async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
  ensureTauriRuntime()
  return invoke<T>(command, args)
}

/**
 * Staff Kit API facade — thin typed layer over Tauri IPC commands.
 * Each method maps 1:1 to a `#[tauri::command]` in the Rust backend.
 */
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

  getBorrowLanSettings: () => call<BorrowLanSettings>("get_borrow_lan_settings"),

  detectBorrowLanHost: () => call<string | null>("detect_borrow_lan_host"),

  probeLanServer: (port: number) => call<boolean>("probe_lan_server", { port }),

  updateBorrowLanSettings: (payload: BorrowLanSettingsUpdateInput) =>
    call<BorrowLanSettings>("update_borrow_lan_settings", {
      payload: {
        host: payload.host,
        port: payload.port,
      },
    }),

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

  getBorrowRequestDetail: (requestId: number) =>
    call<BorrowRequestRecord>("get_borrow_request_detail", { requestId }),

  approveBorrowRequest: (requestId: number) =>
    call<BorrowRequestRecord>("approve_borrow_request", { requestId }),

  rejectBorrowRequest: (payload: BorrowRequestRejectInput) =>
    call<BorrowRequestRecord>("reject_borrow_request", {
      payload: {
        requestId: payload.requestId,
        note: payload.note,
      },
    }),

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
        id: payload.targetAccountId,
        newPassword: payload.newPassword,
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
