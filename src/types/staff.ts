export type DatabaseStatus = {
  initialized: boolean
  dbPath: string
  sqliteVersion: string
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
  role: "admin" | "user"
  isActive: boolean
  createdAt: string
  updatedAt: string
}

export type LocalAccountCreateInput = {
  displayName: string
  role: "admin" | "user"
}

export type LocalAccountUpdateInput = {
  id: number
  displayName: string
  role: "admin" | "user"
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
  dynamicFields?: Record<string, string> | null
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
  memberCount: number
}

export type TeamUpsertInput = {
  id?: number | null
  name: string
}

export type ImportExcelInput = {
  filePath?: string | null
  filePaths?: string[] | null
  selectedColumnKeys?: string[] | null
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
