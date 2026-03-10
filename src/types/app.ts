// ── App-level types ───────────────────────────────────────────────────────────

export type AppView = "employees" | "teams" | "settings"
export type Theme = "dark" | "light"
export type UiColumnSource = "core" | "dynamic" | "system"
export type StaffGroupKey = "employee_list" | "onboarding" | "offboarding" | "internal_movement"
export type SortDirection = "asc" | "desc"
export type LocalAccountRole = "super_admin" | "admin" | "user"

export type UiColumnDefinition = {
    key: string
    label: string
    source: UiColumnSource
}

export type ColumnPreferences = {
    order: string[]
    hidden: string[]
}

export type ColumnWidthMap = Record<string, number>

export type ActiveResizeState = {
    key: string
    startX: number
    startWidth: number
}

export type ActiveTableEditCell = {
    employeeId: number
    columnKey: string
}

export type TableEditDrafts = Record<number, Record<string, string>>

export type EmployeeSortState = {
    key: string
    direction: SortDirection
} | null
