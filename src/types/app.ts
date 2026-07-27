// ── App-level types ───────────────────────────────────────────────────────────

/** Top-level navigation views in the application shell. */
export type AppView = "employees" | "teams" | "borrow" | "settings"

/** Application color theme. */
export type Theme = "dark" | "light"

/** Source classification for employee table columns. */
export type UiColumnSource = "core" | "dynamic" | "system"

/** Staff group tabs — determines which employee subset is shown. */
export type StaffGroupKey = "employee_list" | "onboarding" | "offboarding" | "internal_movement"

/** Sort direction for table columns. */
export type SortDirection = "asc" | "desc"

/** Local account permission levels. */
export type LocalAccountRole = "super_admin" | "admin" | "user"

/** A single column definition rendered in the employee table. */
export type UiColumnDefinition = {
    key: string
    label: string
    source: UiColumnSource
    dataType?: "email"
}

/** User-specific column order and visibility preferences (persisted to localStorage). */
export type ColumnPreferences = {
    order: string[]
    hidden: string[]
}

/** Map of column key → pixel width for resizable columns. */
export type ColumnWidthMap = Record<string, number>

/** Active column resize drag state. */
export type ActiveResizeState = {
    key: string
    startX: number
    startWidth: number
}

/** Currently active inline edit cell in the employee table. */
export type ActiveTableEditCell = {
    employeeId: number
    columnKey: string
}

/** Pending inline edit drafts keyed by employee ID → column key → value. */
export type TableEditDrafts = Record<number, Record<string, string>>

/** Current sort state for the employee table. */
export type EmployeeSortState = {
    key: string
    direction: SortDirection
} | null
