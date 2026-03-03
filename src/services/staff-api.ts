import { invoke } from "@tauri-apps/api/core"
import type {
  DatabaseStatus,
  EmployeeColumnDefinition,
  EmployeeGroupCounts,
  EmployeeColumnUpsertInput,
  EmployeeListResponse,
  EmployeePayload,
  EmployeeQueryInput,
  EmployeeRecord,
  LocalAccountCreateInput,
  LocalAccountRecord,
  LocalAccountUpdateInput,
  ImportColumnsPreview,
  ImportExcelInput,
  ImportReport,
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
        role: payload.role,
      },
    }),

  updateLocalAccount: (payload: LocalAccountUpdateInput) =>
    call<LocalAccountRecord>("update_local_account", {
      payload: {
        id: payload.id,
        displayName: payload.displayName,
        role: payload.role,
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
