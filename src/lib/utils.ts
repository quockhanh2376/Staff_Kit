import type { EmployeeGroupCounts } from "../types/staff"
import type { StaffGroupKey } from "../types/app"

// ── Pure utilities ────────────────────────────────────────────────────────────

/**
 * Extracts a human-readable message from any thrown value.
 * @deprecated Use `getUserErrorMessage` from `lib/errorHandling` for user-facing errors.
 */
export const getErrorMessage = (error: unknown): string => {
    if (error instanceof Error) {
        return error.message
    }
    return String(error)
}

/** Formats an ISO date string into a localized short date (e.g. "Apr 09, 2026"). Returns "-" for null/empty. */
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

/** Returns the employee count for a given staff group key. */
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

/** Normalizes a display name into a safe localStorage scope key (lowercase, underscore-separated). */
export const normalizeUserScope = (name: string): string => {
    const normalized = name
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "_")
        .replace(/^_+|_+$/g, "")

    return normalized || "default_user"
}

/** Builds a namespaced localStorage key: `{baseKey}:{scope}`. */
export const buildScopedStorageKey = (baseKey: string, scope: string): string =>
    `${baseKey}:${scope}`

