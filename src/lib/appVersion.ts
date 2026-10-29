import { useState, useEffect } from "react"
import { staffApi } from "../services/staff-api"

export const FALLBACK_APP_VERSION = "Version unavailable"

let cachedVersion: string | null = null
let inFlightPromise: Promise<string> | null = null

/**
 * Resets the in-memory cache. Only used in test suites.
 */
export function __resetAppVersionCacheForTesting(): void {
  cachedVersion = null
  inFlightPromise = null
}

/**
 * Single source of truth for runtime application version.
 * Caches in-memory after first resolution.
 * If reading fails or returns an empty string, falls back gracefully to
 * "Version unavailable" without throwing or crashing.
 */
export async function fetchAppVersion(): Promise<string> {
  if (cachedVersion !== null) {
    return cachedVersion
  }
  if (!inFlightPromise) {
    inFlightPromise = staffApi
      .getAppVersion()
      .then((ver) => {
        const trimmed = (ver ?? "").trim()
        if (!trimmed) {
          return FALLBACK_APP_VERSION
        }
        cachedVersion = trimmed
        return trimmed
      })
      .catch(() => {
        return FALLBACK_APP_VERSION
      })
      .finally(() => {
        inFlightPromise = null
      })
  }
  return inFlightPromise
}

/**
 * Formats a raw version string for UI display.
 * - null -> "" (still resolving)
 * - "Version unavailable" -> "Version unavailable"
 * - "2.2.1" -> "Version 2.2.1"
 * - "2.2.1-internal-rc1" -> "Version 2.2.1-internal-rc1" (preserves suffixes)
 */
export function formatAppVersion(version: string | null): string {
  if (version === null) {
    return ""
  }
  const trimmed = version.trim()
  if (!trimmed || trimmed === FALLBACK_APP_VERSION) {
    return FALLBACK_APP_VERSION
  }
  if (trimmed.toLowerCase().startsWith("version ")) {
    return trimmed
  }
  if (/^v\d/i.test(trimmed)) {
    return `Version ${trimmed.slice(1)}`
  }
  return `Version ${trimmed}`
}

export interface UseAppVersionResult {
  version: string | null
  displayVersion: string
  isLoading: boolean
}

/**
 * Shared React hook for consuming the runtime app version.
 * Both the Login screen and the post-login App shell consume this hook.
 */
export function useAppVersion(): UseAppVersionResult {
  const [version, setVersion] = useState<string | null>(() => cachedVersion)

  useEffect(() => {
    let active = true
    if (cachedVersion !== null) {
      return
    }

    fetchAppVersion().then((resolved) => {
      if (active) {
        setVersion(resolved)
      }
    })

    return () => {
      active = false
    }
  }, [])

  return {
    version,
    displayVersion: formatAppVersion(version),
    isLoading: version === null,
  }
}
