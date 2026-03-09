import type { ColumnPreferences, ColumnWidthMap } from "../types/app"
import { MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH } from "./constants"

// ── Raw storage helpers ───────────────────────────────────────────────────────

export const readRawStorageValue = (storageKey: string, fallbackKeys: string[] = []): string | null => {
    const current = localStorage.getItem(storageKey)
    if (current) {
        return current
    }

    for (const key of fallbackKeys) {
        const fallback = localStorage.getItem(key)
        if (fallback) {
            return fallback
        }
    }

    return null
}

// ── Column preferences ────────────────────────────────────────────────────────

export const readColumnPreferences = (storageKey: string, fallbackKeys: string[] = []): ColumnPreferences => {
    try {
        const raw = readRawStorageValue(storageKey, fallbackKeys)
        if (!raw) {
            return { order: [], hidden: [] }
        }

        const parsed = JSON.parse(raw) as Partial<ColumnPreferences>
        return {
            order: Array.isArray(parsed.order)
                ? parsed.order.filter((item): item is string => typeof item === "string")
                : [],
            hidden: Array.isArray(parsed.hidden)
                ? parsed.hidden.filter((item): item is string => typeof item === "string")
                : [],
        }
    } catch {
        return { order: [], hidden: [] }
    }
}

export const readColumnLabelOverrides = (storageKey: string, fallbackKeys: string[] = []): Record<string, string> => {
    try {
        const raw = readRawStorageValue(storageKey, fallbackKeys)
        if (!raw) {
            return {}
        }

        const parsed = JSON.parse(raw) as Record<string, unknown>
        const entries = Object.entries(parsed).filter(
            ([key, value]) => key.trim().length > 0 && typeof value === "string",
        )
        return Object.fromEntries(entries) as Record<string, string>
    } catch {
        return {}
    }
}

export const readColumnWidths = (storageKey: string, fallbackKeys: string[] = []): ColumnWidthMap => {
    try {
        const raw = readRawStorageValue(storageKey, fallbackKeys)
        if (!raw) {
            return {}
        }

        const parsed = JSON.parse(raw) as Record<string, unknown>
        const next: ColumnWidthMap = {}
        for (const [key, value] of Object.entries(parsed)) {
            if (typeof value !== "number" || Number.isNaN(value)) {
                continue
            }
            next[key] = Math.max(MIN_COLUMN_WIDTH, Math.min(MAX_COLUMN_WIDTH, Math.round(value)))
        }
        return next
    } catch {
        return {}
    }
}

// ── Serializers (for change-detection) ───────────────────────────────────────

export const serializeColumnPreferences = (value: ColumnPreferences): string => {
    const order = [...value.order].join("|")
    const hidden = [...value.hidden].join("|")
    return `${order}__${hidden}`
}

export const serializeStringMap = (value: Record<string, string>): string => (
    Object.keys(value)
        .sort((a, b) => a.localeCompare(b))
        .map((key) => `${key}:${value[key]}`)
        .join("|")
)

export const serializeWidthMap = (value: ColumnWidthMap): string => (
    Object.keys(value)
        .sort((a, b) => a.localeCompare(b))
        .map((key) => `${key}:${value[key]}`)
        .join("|")
)
