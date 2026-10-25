/** Database initialization status returned on app startup. */
export type DatabaseStatus = {
  initialized: boolean
  dbPath: string
  sqliteVersion: string
}

/** Persisted backup configuration. */
export type BackupSettings = {
  backupDirectoryPath: string
  autoBackupEnabled: boolean
  retentionFiles: number
  autoBackupIntervalDays: number
  autoBackupRetentionDays: number
}

/** Information about a single database snapshot. */
export type SnapshotInfo = {
  filename: string
  label: string
  timestamp: string
  sizeMb: number
  fullPath: string
}

/** Result of a backup operation (manual or automatic). */
export type BackupRunResult = {
  backupFilePath: string
  retainedFiles: number
  performed: boolean
}

export type BackupSettingsUpdateInput = {
  backupDirectoryPath: string
  autoBackupEnabled: boolean
}

/** LAN server configuration for the Borrow/Return QR flow. */
export type BorrowLanSettings = {
  enabled: boolean
  host: string
  port: number
  borrowUrl: string
}

export type BorrowLanSettingsUpdateInput = {
  enabled?: boolean
  host: string
  port: number
}

export type BorrowLanTokenStatus = {
  ready: boolean
}

export type BorrowLanServerStatus = {
  running: boolean
  tokenReady: boolean
  bindHost: string
  port: number
}

/** Input payload for creating or upserting a single asset. */
export type AssetSeedItemInput = {
  assetCode: string
  categoryId?: number | null
  assetType: string
  displayName: string
  displayNameShort?: string | null
  brand?: string | null
  model?: string | null
  serialNumber?: string | null
  usageLocation?: string | null
  warehouse?: string | null
  notes?: string | null
}

/** Persisted asset record (core fields). */
export type AssetRecord = {
  id: number
  assetCode: string
  assetType: string
  displayName: string
  model: string | null
  serialNumber: string | null
  notes: string | null
  status: string
}

/** How an asset category tracks its items. */
export type AssetTrackingMode = "serialized" | "quantity"
export type AssetImportMode = AssetTrackingMode

export type AssetCategoryRecord = {
  id: number
  categoryCode: string
  categoryName: string
  trackingMode: AssetTrackingMode
  prefixCode: string | null
  qrRequired: boolean
  isActive: boolean
}

export type AssetCategoryPrefixRecord = {
  id: number
  prefixValue: string
  isPrimary: boolean
  isActive: boolean
}

export type AssetCategoryDetailRecord = {
  id: number
  categoryCode: string
  categoryName: string
  trackingMode: AssetTrackingMode
  prefixCode: string | null
  qrRequired: boolean
  isActive: boolean
  assetCount: number
  stockItemCount: number
  prefixes: AssetCategoryPrefixRecord[]
}

export type AssetCategoryPrefixInput = {
  prefixValue: string
  isPrimary: boolean
}

export type AssetCategoryUpsertInput = {
  id?: number | null
  categoryCode: string
  categoryName: string
  trackingMode: AssetTrackingMode
  qrRequired: boolean
  prefixes: AssetCategoryPrefixInput[]
}

export type AssetIntegrityEmployeeRef = {
  id: number
  employeeId: string
  fullName: string
  staffGroup: string | null
}

export type AssetIntegrityFinding = {
  id: string
  category: "duplicate_holder" | "multiple_active_loans" | "duplicate_asset_code" | "orphan_loan" | "status_mismatch" | "duplicate_computer_name"
  severity: "error" | "warning"
  assetCode: string | null
  assetId: number | null
  assetName: string | null
  affectedEmployees: AssetIntegrityEmployeeRef[]
  reason: string
  recommendation: string
}

export type AssetIntegrityCheckResult = {
  isClean: boolean
  totalAssetsChecked: number
  totalActiveLoansChecked: number
  totalEmployeesChecked: number
  affectedEmployeeIds: number[]
  findings: AssetIntegrityFinding[]
}

/** Aggregate counts for the asset dashboard summary cards. */
export type AssetDashboardSummary = {
  totalSerializedAssets: number
  serializedInStock: number
  serializedAssigned: number
  totalQuantityOnHand: number
  totalQuantityAssigned: number
}

/** A single serialized asset row on the dashboard (joined with holder info). */
export type AssetDashboardSerializedRecord = {
  assetId: number
  assetCode: string
  categoryCode: string | null
  categoryName: string | null
  computerName: string | null
  displayName: string
  displayNameShort: string | null
  model: string | null
  serialNumber: string | null
  adapterNumber: string | null
  usageLocation: string | null
  notes: string | null
  status: string
  holderEmployeeId: string | null
  holderFullName: string | null
  signatureStatus?: string | null
}

export type AssetDashboardQuantityRecord = {
  stockItemId: number
  categoryCode: string
  categoryName: string
  itemName: string
  brand: string | null
  model: string | null
  warehouse: string | null
  quantityOnHand: number
  assignedQuantity: number
  note: string | null
}

export type StockItemQuantityUpdateInput = {
  stockItemId: number
  quantityOnHand: number
  assignedQuantity: number
}

export type AssetImportFieldMapping = {
  assetCode?: string | null
  assetType?: string | null
  displayName?: string | null
  computerName?: string | null
  displayNameShort?: string | null
  brand?: string | null
  model?: string | null
  serialNumber?: string | null
  adapterNumber?: string | null
  usageLocation?: string | null
  quantity?: string | null
  warehouse?: string | null
  notes?: string | null
}

export type AssetImportInspectInput = {
  filePath: string
  sheetName?: string | null
}

export type AssetDirectImportInput = {
  importType: AssetImportMode
  filePath: string
  sheetName?: string | null
}

export type AssetImportFileInspection = {
  fileName: string
  fileType: string
  selectedSheetName: string | null
  availableSheets: string[]
  headerRow: number
  headers: string[]
  mapping: AssetImportFieldMapping
  requiresManualMapping: boolean
}

export type AssetImportBatchCreateInput = {
  importType: AssetImportMode
  filePath: string
  sheetName?: string | null
  mapping?: AssetImportFieldMapping | null
}

export type AssetImportBatchSummary = {
  id: number
  batchKey: string
  importType: AssetImportMode
  sourceFileName: string
  sourceFilePath: string
  sourceFileType: string
  sheetName: string | null
  headerRow: number
  status: string
  totalRows: number
  validRows: number
  errorRows: number
  importedRows: number
  skippedRows: number
  createdAt: string
  updatedAt: string
}

export type AssetImportRawValue = {
  header: string
  value: string
}

export type AssetImportRowRecord = {
  id: number
  batchId: number
  rowNumber: number
  rawValues: AssetImportRawValue[]
  assetCode: string | null
  assetType: string | null
  displayName: string | null
  displayNameShort?: string | null
  computerName?: string | null
  brand: string | null
  model: string | null
  serialNumber: string | null
  adapterNumber?: string | null
  usageLocation?: string | null
  quantity: string | null
  warehouse: string | null
  notes: string | null
  submittedStaffId: string | null
  submittedFullName: string | null
  submittedTeam: string | null
  submittedPhoneNumber: string | null
  resolvedEmployeeId: string | null
  resolvedEmployeeRowId: number | null
  resolvedFullName: string | null
  resolvedTeamName: string | null
  ownerMatchStatus: string
  ownerWarnings: string[]
  validationErrors: string[]
  status: string
  isEdited: boolean
  editedFields: string[]
  importedAssetId: number | null
}

export type AssetImportBatchDetail = {
  summary: AssetImportBatchSummary
  headers: string[]
  mapping: AssetImportFieldMapping
  rows: AssetImportRowRecord[]
}

export type AssetImportRowUpdateInput = {
  rowId: number
  fieldKey:
    | "assetCode"
    | "assetType"
    | "displayName"
    | "displayNameShort"
    | "computerName"
    | "brand"
    | "model"
    | "serialNumber"
    | "adapterNumber"
    | "usageLocation"
    | "quantity"
    | "warehouse"
    | "notes"
    | "submittedStaffId"
    | "submittedFullName"
    | "submittedTeam"
    | "submittedPhoneNumber"
  value?: string | null
}

export type AssetImportRowSkipInput = {
  rowId: number
  skipped: boolean
}

export type AssetImportCommitResult = {
  batchId: number
  importedRowIds: number[]
  importedAssetCodes: string[]
  importedCount: number
  remainingErrorRows: number
  batchStatus: string
}

export type AssetDirectImportErrorItem = {
  rowNumber: number
  entityKey: string | null
  reason: string
}

export type AssetDirectImportPreviewRow = {
  rowNumber: number
  assetCode: string | null
  assetType: string | null
  computerName: string | null
  displayName: string | null
  model: string | null
  serialNumber: string | null
  adapterNumber: string | null
  quantity: string | null
  usageLocation: string | null
  notes: string | null
  status: string
  holderLabel: string | null
  validationErrors: string[]
}

export type AssetDirectImportNewCategorySummary = {
  categoryName: string
  assetCount: number
}

export type AssetDirectImportPreview = {
  fileName: string
  sheetName: string | null
  importType: AssetImportMode
  totalRows: number
  validRows: number
  errorRows: number
  skippedRows: number
  newCategories: AssetDirectImportNewCategorySummary[]
  rows: AssetDirectImportPreviewRow[]
  errors: AssetDirectImportErrorItem[]
}

export type AssetDirectImportReport = {
  fileName: string
  sheetName: string | null
  importType: AssetImportMode
  totalRows: number
  imported: number
  skipped: number
  failed: number
  importedAssetCodes: string[]
  errors: AssetDirectImportErrorItem[]
}

/** Borrow/Return request submission from the LAN QR page. */
export type BorrowRequestSubmitInput = {
  submittedEmployeeId: string
  submittedFullName: string
  assetCodes: string[]
  manualEmployeeEmail?: string | null
  manualEmployeeTeam?: string | null
}

/** A pending or processed borrow/return request (Core Rule: Request Pending → Approve). */
export type BorrowRequestRecord = {
  id: number
  requestKey: string
  submittedEmployeeId: string
  submittedFullName: string
  status: string
  requestType: string
  assetCodes: string[]
  submittedAt: string
  decisionNote: string | null
  manualEmployeeEmail?: string | null
  manualEmployeeTeam?: string | null
  borrowerEmployeeIdFk?: number | null
  borrowerStaffId?: string | null
  borrowerName?: string | null
  submittedByEmployeeIdFk?: number | null
  submittedByStaffId?: string | null
  submittedByName?: string | null
  hasAcknowledgment?: boolean
  confirmationMethod?: string | null
  hasSignature?: boolean
  hasTypedName?: boolean
}

export type BorrowPolicyRecord = {
  version: number
  textEn: string
  textVi: string
}

export type BorrowRequestEvidenceRecord = {
  borrowRequestId: number
  policyVersion: number | null
  policyTextEnSnapshot: string | null
  policyTextViSnapshot: string | null
  policyAcknowledged: boolean
  confirmationMethod: string
  typedName: string | null
  hasSignature: boolean
  confirmedAt: string
  signaturePngBase64: string | null
}

export type BorrowRequestRejectInput = {
  requestId: number
  note: string
}

/** Full employee record with core, computed, and dynamic fields. */
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
  storedComputerName: string | null
  notes: string | null
  staffGroup: string
  isFloating: boolean
  lifecycleStatus: string | null
  assignedAssetsCount?: number
  dynamicFields: Record<string, string>
  updatedAt: string
}

export type EmployeeAssignedAssetRecord = {
  assetId: number
  assetCode: string
  assetName: string
  categoryName: string
  categoryCode: string | null
  serialNumber: string | null
  usageLocation: string | null
  borrowedAt: string
  signatureStatus?: string | null
  handoverDate?: string | null
  importedAt?: string | null
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
  isFloating?: boolean | null
  startDateFrom?: string | null
  startDateTo?: string | null
  sortKey?: string | null
  sortDirection?: "asc" | "desc" | null
  limit?: number
  offset?: number
}

export type EmployeeAssetSeedInput = {
  snapshotId?: number | null
  query?: string | null
  teamName?: string | null
  staffGroup?: string | null
  startDateFrom?: string | null
  startDateTo?: string | null
}

export type EmployeeAssetSeedPreviewRow = {
  employeeId: string
  fullName: string
  sourceComputerName: string
  assetCode: string | null
  computerName: string | null
  categoryCode: string | null
  categoryName: string | null
  status: string
}

export type EmployeeAssetSeedPreview = {
  snapshotId: number
  sourceLabel: string
  matchedEmployeeCount: number
  excludedRows: number
  totalRows: number
  validRows: number
  errorRows: number
  rows: EmployeeAssetSeedPreviewRow[]
  errors: SharedImportErrorItem[]
}

export type EmployeeAssetSeedReport = {
  snapshotId: number
  sourceLabel: string
  matchedEmployeeCount: number
  excludedRows: number
  totalRows: number
  imported: number
  skipped: number
  failed: number
  importedAssetCodes: string[]
  errors: SharedImportErrorItem[]
}

export type EmployeeGroupCounts = {
  employeeList: number
  onboarding: number
  offboarding: number
  internalMovement: number
  floating: number
  total: number
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

/** Successful login result. Carries the opaque session token + UX-only expiry
 *  plus the safe account metadata the UI needs. Never contains password hashes
 *  or recovery codes. */
export type LocalAccountLoginResult = {
  sessionToken: string
  expiresAt: string
  account: LocalAccountRecord
}

/** Minimal public account hint for the login screen (username + display name). */
export type LoginAccountHint = {
  username: string
  displayName: string
}

export type LocalPasswordChangeInput = {
  accountId: number
  currentPassword: string
  newPassword: string
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
  isFloating?: boolean | null
  lifecycleStatus?: string | null
  dynamicFields?: Record<string, string> | null
}

export type MoveEmployeesGroupInput = {
  employeeIds: number[]
  targetStaffGroup: string
  action?: string | null
  reason?: string | null
}

export type EmployeeColumnDefinition = {
  key: string
  label: string
  source: "core" | "dynamic"
  dataType?: "email"
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
  sheetName?: string | null
  selectedColumnKeys?: string[] | null
  targetStaffGroup?: string | null
}

export type ImportDetectionKind = 'employee' | 'asset' | 'unknown' | 'ambiguous'

export type ImportDetectionSubtype =
  | 'employee_list'
  | 'onboarding'
  | 'offboarding'
  | 'internal_movement'
  | 'serialized'
  | 'quantity'

export type ImportDetectionResult = {
  type: ImportDetectionKind
  subtype: ImportDetectionSubtype | null
  confidence: number
  sheetName: string | null
  headerRow: number | null
  rowCount: number
  evidenceHeaders: string[]
  reason: string
  warnings: string[]
  candidateTypes: string[]
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

export type SharedImportPreviewSummary = {
  totalRows: number
  validRows: number
  errorRows: number
}

export type SharedImportStatItem = {
  label: string
  value: string | number
}

export type SharedImportPreviewCell = {
  key: string
  label: string
  value: string
}

export type SharedImportPreviewRow = {
  id: string
  title: string
  subtitle: string | null
  badge: string | null
  cells: SharedImportPreviewCell[]
}

export type SharedImportErrorItem = {
  rowNumber: number
  entityKey: string | null
  reason: string
}

export type SharedImportSourceInfo = {
  sourceFiles: string[]
  sheetName: string | null
}

export type SharedImportReport = {
  imported: number
  skipped: number
  failed: number
  errors: SharedImportErrorItem[]
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

export type MssqlAuthType = "sql_server" | "windows_integrated"

export type RemoteEeSyncSettings = {
  server: string
  database: string
  port: number
  authType: MssqlAuthType
  username: string
  approvedUsername: string
  hasSavedPassword: boolean
  environmentOverride: boolean
}

export type MssqlConnectionTestResult = {
  status: "PASS" | "FAIL"
  reason: string
  rowCount: number | null
}

export type EmployeeSyncCounts = {
  new: number
  changed: number
  unchanged: number
  missing: number
  conflicts: number
}

export type EmployeeSyncRow = {
  code: string
  status: "new" | "changed" | "unchanged" | "missing" | "conflict"
  sourceName: string | null
  localName: string | null
}

export type EmployeeSyncConflict = {
  code: string
  reason: string
}

export type EmployeeListSyncPreview = {
  previewId: string
  createdAt: string
  counts: EmployeeSyncCounts
  rows: EmployeeSyncRow[]
  conflicts: EmployeeSyncConflict[]
}

export type EmployeeListSyncResult = {
  counts: EmployeeSyncCounts
  completedAt: string
}

export type HistoricalAssignmentRowStatus =
  | "newAsset"
  | "newAssignment"
  | "alreadyAssigned"
  | "locationChanged"
  | "differentHolder"
  | "existingLoanMissingFromWorkbook"
  | "missingAsset"
  | "missingEmployee"
  | "conflict"
  | "error"

export type HistoricalAssignmentItem = {
  rowNumber: number
  rawStaffId: string
  rawEmployeeName: string
  rawAssetCode: string
  location: string
  status: HistoricalAssignmentRowStatus
  statusReason: string | null
  employeeId: number | null
  canonicalStaffId: string | null
  canonicalEmployeeName: string | null
  assetId: number | null
  canonicalAssetCode: string | null
  currentHolderStaffId: string | null
  currentHolderName: string | null
  currentHolderEmployeeId?: number | null
  currentLocation?: string | null
  currentLoanId?: number | null
  suggestedAction?: string | null
  selectedAction?: string | null
}

export type HistoricalAssignmentPreviewSummary = {
  totalRows: number
  totalAssignments: number
  newAssets: number
  newAssignments: number
  existingSkipped: number
  locationChanged?: number
  differentHolders?: number
  existingLoansMissing?: number
  excludedConflicts: number
  errors: number
  canApproveRemainder: boolean
  alreadyAssigned?: number
  missingAssets?: number
  missingEmployees?: number
  conflicts?: number
  hasBlockingConflicts?: boolean
}

export type HistoricalAssignmentPreview = {
  fileName: string
  summary: HistoricalAssignmentPreviewSummary
  items: HistoricalAssignmentItem[]
}

export type HistoricalAssignmentRowOverride = {
  rowNumber: number
  rawAssetCode: string
  correctedStaffId?: string | null
  correctedAssetCode?: string | null
}

export type HistoricalAssignmentActionDecision = {
  rowNumber: number
  rawAssetCode: string
  action: "approve" | "update_location" | "confirm_transfer" | "quarantine" | "skip"
  correctedStaffId?: string | null
  correctedAssetCode?: string | null
}

export type HistoricalAssignmentImportInput = {
  filePath: string
  sheetName?: string | null
  rowOverrides?: HistoricalAssignmentRowOverride[] | null
  rowActions?: HistoricalAssignmentActionDecision[] | null
}

export type HistoricalAssignmentImportReport = {
  totalProcessed: number
  createdAssets: number
  createdAssignments: number
  skippedAlreadyAssigned: number
  updatedLocations?: number
  transferredAssignments?: number
  excludedConflicts: number
  errors: string[]
}

export type SignHistoricalBorrowRequestInput = {
  requestId: number
  signaturePngBlob?: number[] | null
  typedName?: string | null
}
