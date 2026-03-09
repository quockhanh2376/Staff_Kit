import type { EmployeeGroupCounts } from "../types/staff"
import type { StaffGroupKey } from "../types/app"

// ── Pure utilities ────────────────────────────────────────────────────────────

export const getErrorMessage = (error: unknown): string => {
    if (error instanceof Error) {
        return error.message
    }
    return String(error)
}

export const formatDate = (value: string | null) => {
    if (!value) {
        return "-"
    }

    const date = new Date(value)
    if (Number.isNaN(date.getTime())) {
        return value
    }

    return date.toLocaleDateString("en-US", {
        year: "numeric",
        month: "short",
        day: "2-digit",
    })
}

export const getGroupCount = (counts: EmployeeGroupCounts, key: StaffGroupKey): number => {
    switch (key) {
        case "employee_list":
            return counts.employeeList
        case "onboarding":
            return counts.onboarding
        case "offboarding":
            return counts.offboarding
        case "internal_movement":
            return counts.internalMovement
        default:
            return 0
    }
}

export const normalizeUserScope = (name: string): string => {
    const normalized = name
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "_")
        .replace(/^_+|_+$/g, "")

    return normalized || "default_user"
}

export const buildScopedStorageKey = (baseKey: string, scope: string): string =>
    `${baseKey}:${scope}`
