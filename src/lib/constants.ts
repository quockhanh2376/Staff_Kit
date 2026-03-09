import type { StaffGroupKey, UiColumnDefinition, ColumnWidthMap } from "../types/app"

// ── Storage keys ──────────────────────────────────────────────────────────────

export const THEME_KEY = "staffkit-theme"
export const COLUMN_PREFS_KEY = "staffkit-column-prefs"
export const COLUMN_PREFS_VERSION_KEY = "staffkit-column-prefs-version"
export const COLUMN_PREFS_VERSION = "4"
export const COLUMN_LABEL_OVERRIDES_KEY = "staffkit-column-label-overrides"
export const COLUMN_WIDTHS_KEY = "staffkit-column-widths"

// ── App defaults ─────────────────────────────────────────────────────────────

export const DEFAULT_ACCOUNT_NAME = "adman"
export const DEFAULT_NEW_ACCOUNT_PASSWORD = "Welcome!"
export const ALL_TEAMS_OPTION = "All Teams"

export const STAFF_GROUP_BUTTONS: Array<{ key: StaffGroupKey; label: string }> = [
    { key: "employee_list", label: "Employee list" },
    { key: "onboarding", label: "Onboarding" },
    { key: "offboarding", label: "Offboarding" },
    { key: "internal_movement", label: "Internal Movement" },
]

export const DEFAULT_SYSTEM_COLUMNS: UiColumnDefinition[] = [
    { key: "rowNumber", label: "#", source: "system" },
]

export const DEFAULT_VISIBLE_COLUMN_KEYS = [
    "employeeId",
    "fullName",
    "teamName",
    "email",
    "computerName",
    "computer_name_2",
]

export const DATE_COLUMN_KEYS = new Set([
    "dateOfBirth",
    "aswStartDate",
    "clientStartDate",
])

// ── Column sizing ─────────────────────────────────────────────────────────────

export const MIN_COLUMN_WIDTH = 78
export const MAX_COLUMN_WIDTH = 720

export const DEFAULT_COLUMN_WIDTHS: ColumnWidthMap = {
    rowNumber: 64,
    employeeId: 116,
    fullName: 220,
    nickName: 130,
    teamName: 190,
    project: 170,
    jobTitle: 210,
    email: 230,
    cellphone: 145,
    dateOfBirth: 130,
    gender: 100,
    aswStartDate: 130,
    clientStartDate: 140,
    contractEndDate: 140,
    clientYearOfServices: 176,
    computerName: 170,
    notes: 220,
}
