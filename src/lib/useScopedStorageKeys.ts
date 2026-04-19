import { useMemo } from "react"
import type { StaffGroupKey } from "../types/app"
import {
  COLUMN_PREFS_KEY,
  COLUMN_PREFS_VERSION_KEY,
  COLUMN_LABEL_OVERRIDES_KEY,
  COLUMN_WIDTHS_KEY,
  DEFAULT_ACCOUNT_NAME,
} from "./constants"
import { normalizeUserScope, buildScopedStorageKey } from "./utils"

/**
 * Scoped storage key bundle for column preferences.
 * Each key is namespaced by user + staff group to support multi-profile / multi-table storage.
 */
export type ScopedColumnKeys = {
  scopedColumnPrefsKey: string
  scopedColumnPrefsVersionKey: string
  scopedColumnLabelOverridesKey: string
  scopedColumnWidthsKey: string
  scopedColumnPrefsFallbackKeys: string[]
  scopedColumnPrefsVersionFallbackKeys: string[]
  scopedColumnLabelOverridesFallbackKeys: string[]
  scopedColumnWidthsFallbackKeys: string[]
}

/**
 * Derives all scoped localStorage keys for column preferences.
 * Extracts the repetitive key-building logic from App.tsx into a reusable hook.
 *
 * @param accountKey - The active account's storage key (or null for default)
 * @param staffGroupFilter - Current staff group tab
 * @returns Memoized scoped column storage keys with legacy fallbacks
 */
export function useScopedStorageKeys(
  accountKey: string | undefined | null,
  staffGroupFilter: StaffGroupKey,
): ScopedColumnKeys {
  const activeUserScope = useMemo(
    () => accountKey ?? normalizeUserScope(DEFAULT_ACCOUNT_NAME),
    [accountKey],
  )

  const columnPrefsScope = useMemo(
    () => `${activeUserScope}:${staffGroupFilter}`,
    [activeUserScope, staffGroupFilter],
  )

  // Legacy (non-group-scoped) keys — used as fallback for employee_list tab
  const legacyScopedColumnPrefsKey = useMemo(
    () => buildScopedStorageKey(COLUMN_PREFS_KEY, activeUserScope),
    [activeUserScope],
  )
  const legacyScopedColumnPrefsVersionKey = useMemo(
    () => buildScopedStorageKey(COLUMN_PREFS_VERSION_KEY, activeUserScope),
    [activeUserScope],
  )
  const legacyScopedColumnLabelOverridesKey = useMemo(
    () => buildScopedStorageKey(COLUMN_LABEL_OVERRIDES_KEY, activeUserScope),
    [activeUserScope],
  )
  const legacyScopedColumnWidthsKey = useMemo(
    () => buildScopedStorageKey(COLUMN_WIDTHS_KEY, activeUserScope),
    [activeUserScope],
  )

  // Current group-scoped keys
  const scopedColumnPrefsKey = useMemo(
    () => buildScopedStorageKey(COLUMN_PREFS_KEY, columnPrefsScope),
    [columnPrefsScope],
  )
  const scopedColumnPrefsVersionKey = useMemo(
    () => buildScopedStorageKey(COLUMN_PREFS_VERSION_KEY, columnPrefsScope),
    [columnPrefsScope],
  )
  const scopedColumnLabelOverridesKey = useMemo(
    () => buildScopedStorageKey(COLUMN_LABEL_OVERRIDES_KEY, columnPrefsScope),
    [columnPrefsScope],
  )
  const scopedColumnWidthsKey = useMemo(
    () => buildScopedStorageKey(COLUMN_WIDTHS_KEY, columnPrefsScope),
    [columnPrefsScope],
  )

  // Fallbacks: only for employee_list (migration from old non-group-scoped keys)
  const isEmployeeList = staffGroupFilter === "employee_list"
  const scopedColumnPrefsFallbackKeys = useMemo(
    () => (isEmployeeList ? [legacyScopedColumnPrefsKey] : []),
    [isEmployeeList, legacyScopedColumnPrefsKey],
  )
  const scopedColumnPrefsVersionFallbackKeys = useMemo(
    () => (isEmployeeList ? [legacyScopedColumnPrefsVersionKey] : []),
    [isEmployeeList, legacyScopedColumnPrefsVersionKey],
  )
  const scopedColumnLabelOverridesFallbackKeys = useMemo(
    () => (isEmployeeList ? [legacyScopedColumnLabelOverridesKey] : []),
    [isEmployeeList, legacyScopedColumnLabelOverridesKey],
  )
  const scopedColumnWidthsFallbackKeys = useMemo(
    () => (isEmployeeList ? [legacyScopedColumnWidthsKey] : []),
    [isEmployeeList, legacyScopedColumnWidthsKey],
  )

  return {
    scopedColumnPrefsKey,
    scopedColumnPrefsVersionKey,
    scopedColumnLabelOverridesKey,
    scopedColumnWidthsKey,
    scopedColumnPrefsFallbackKeys,
    scopedColumnPrefsVersionFallbackKeys,
    scopedColumnLabelOverridesFallbackKeys,
    scopedColumnWidthsFallbackKeys,
  }
}
