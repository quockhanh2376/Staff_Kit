export type DatabaseStatus = {
  initialized: boolean
  dbPath: string
  sqliteVersion: string
}

export type BackupSettings = {
  backupDirectoryPath: string
  autoBackupEnabled: boolean
  retentionFiles: number
  autoBackupIntervalDays: number
  autoBackupRetentionDays: number
}

export type SnapshotInfo = {
  filename: string
  label: string
  timestamp: string
  sizeMb: number
  fullPath: string
}

export type BackupRunResult = {
  backupFilePath: string
  retainedFiles: number
  performed: boolean
}

export type BackupSettingsUpdateInput = {
  backupDirectoryPath: string
  autoBackupEnabled: boolean
}

export type EmployeeRecord = {
  id: number
  employeeId: string
  fullName: string
  nickName: string | null
  teamId: number | null
  teamName: string | null
  project: string | null
  jobTitle: string | null
  email: string | null
  cellphone: string | null
  dateOfBirth: string | null
  gender: string | null
  aswStartDate: string | null
  clientStartDate: string | null
  contractEndDate: string | null
  clientYearOfServices: string | null
  startDate: string | null
  computerName: string | null
  notes: string | null
  staffGroup: string
  dynamicFields: Record<string, string>
  updatedAt: string
}

export type EmployeeListResponse = {
  items: EmployeeRecord[]
  total: number
  limit: number
  offset: number
}

export type EmployeeQueryInput = {
  query?: string | null
  teamName?: string | null
  staffGroup?: string | null
  startDateFrom?: string | null
  startDateTo?: string | null
  sortKey?: string | null
  sortDirection?: "asc" | "desc" | null
  limit?: number
  offset?: number
}

export type EmployeeGroupCounts = {
  employeeList: number
  onboarding: number
  offboarding: number
  internalMovement: number
}

export type LocalAccountRecord = {
  id: number
  accountKey: string
  displayName: string
  username: string
  role: "super_admin" | "admin" | "user"
  isSuperAdmin: boolean
  isActive: boolean
  forcePasswordReset: boolean
  createdAt: string
  updatedAt: string
}

export type LocalAccountCreateInput = {
  displayName: string
  username: string
  password?: string | null
  recoveryCode?: string | null
  role: "super_admin" | "admin" | "user"
}

export type LocalAccountUpdateInput = {
  id: number
  displayName: string
  username: string
  role: "super_admin" | "admin" | "user"
}

export type LocalAccountLoginInput = {
  username: string
  password: string
}

export type LocalPasswordChangeInput = {
  accountId: number
  currentPassword: string
  newPassword: string
  newRecoveryCode?: string | null
}

export type LocalPasswordResetInput = {
  adminAccountId: number
  targetAccountId: number
  newPassword: string
  newRecoveryCode?: string | null
}

export type LocalForgotPasswordInput = {
  username: string
  recoveryCode: string
  newPassword: string
}

export type EmployeePayload = {
  employeeId: string
  fullName: string
  nickName?: string | null
  teamName?: string | null
  project?: string | null
  jobTitle?: string | null
  email?: string | null
  cellphone?: string | null
  dateOfBirth?: string | null
  gender?: string | null
  aswStartDate?: string | null
  clientStartDate?: string | null
  contractEndDate?: string | null
  clientYearOfServices?: string | null
  computerName?: string | null
  notes?: string | null
  staffGroup?: string | null
  dynamicFields?: Record<string, string> | null
}

export type MoveEmployeesGroupInput = {
  employeeIds: number[]
  targetStaffGroup: string
}

export type EmployeeColumnDefinition = {
  key: string
  label: string
  source: "core" | "dynamic"
}

export type EmployeeColumnUpsertInput = {
  key?: string | null
  label: string
}

export type TeamRecord = {
  id: number
  name: string
  parentId: number | null
  parentName: string | null
  memberCount: number
}

export type TeamUpsertInput = {
  id?: number | null
  name: string
  parentName?: string | null
}

export type ImportExcelInput = {
  filePath?: string | null
  filePaths?: string[] | null
  selectedColumnKeys?: string[] | null
  targetStaffGroup?: string | null
}

export type ImportColumnOption = {
  key: string
  label: string
  source: "core" | "dynamic" | "required"
  required: boolean
}

export type ImportColumnsPreview = {
  sourceFiles: string[]
  detectedColumns: ImportColumnOption[]
}

export type ImportErrorItem = {
  row: number
  employeeId: string | null
  reason: string
}

export type ImportReport = {
  sourceFile: string
  sourceFiles: string[]
  sheetName: string
  headerRow: number
  processedSheets: string[]
  totalRows: number
  inserted: number
  updated: number
  skipped: number
  failed: number
  errors: ImportErrorItem[]
}

export type FieldChange = {
  fieldKey: string
  fieldLabel: string
  oldValue: string | null
  newValue: string | null
}

export type ImportPreviewRow = {
  rowNumber: number
  employeeId: string
  fullName: string
  isUpdate: boolean
  changes: FieldChange[]
}

export type ImportPreviewResult = {
  sourceFiles: string[]
  sheetName: string
  previewRows: ImportPreviewRow[]
  totalChanges: number
  totalNew: number
  totalUpdates: number
  errors: ImportErrorItem[]
}
