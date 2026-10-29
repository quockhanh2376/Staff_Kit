import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import {
  FALLBACK_APP_VERSION,
  fetchAppVersion,
  formatAppVersion,
  useAppVersion,
  __resetAppVersionCacheForTesting,
} from "./appVersion"
import { staffApi } from "../services/staff-api"
import { renderHook, waitFor } from "@testing-library/react"

vi.mock("../services/staff-api", () => ({
  staffApi: {
    getAppVersion: vi.fn(),
  },
}))

describe("appVersion utility & hook", () => {
  beforeEach(() => {
    __resetAppVersionCacheForTesting()
    vi.clearAllMocks()
  })

  afterEach(() => {
    __resetAppVersionCacheForTesting()
  })

  describe("formatAppVersion", () => {
    it("returns empty string when version is null", () => {
      expect(formatAppVersion(null)).toBe("")
    })

    it("formats standard semver with 'Version ' prefix", () => {
      expect(formatAppVersion("2.2.1")).toBe("Version 2.2.1")
    })

    it("preserves prerelease and internal rc suffixes intact", () => {
      expect(formatAppVersion("2.2.1-internal-rc1")).toBe("Version 2.2.1-internal-rc1")
      expect(formatAppVersion("2.3.0-beta.1+build123")).toBe("Version 2.3.0-beta.1+build123")
    })

    it("does not duplicate 'Version ' if already prefixed", () => {
      expect(formatAppVersion("Version 2.2.1")).toBe("Version 2.2.1")
      expect(formatAppVersion("version 2.2.1-rc1")).toBe("version 2.2.1-rc1")
    })

    it("normalizes 'v' prefix to 'Version '", () => {
      expect(formatAppVersion("v2.2.1")).toBe("Version 2.2.1")
    })

    it("returns 'Version unavailable' for fallback or empty input", () => {
      expect(formatAppVersion(FALLBACK_APP_VERSION)).toBe(FALLBACK_APP_VERSION)
      expect(formatAppVersion("")).toBe(FALLBACK_APP_VERSION)
      expect(formatAppVersion("   ")).toBe(FALLBACK_APP_VERSION)
    })
  })

  describe("fetchAppVersion", () => {
    it("fetches version from staffApi and caches in memory", async () => {
      vi.mocked(staffApi.getAppVersion).mockResolvedValueOnce("2.2.1")

      const first = await fetchAppVersion()
      expect(first).toBe("2.2.1")
      expect(staffApi.getAppVersion).toHaveBeenCalledTimes(1)

      // Second call must use cache without calling IPC again
      const second = await fetchAppVersion()
      expect(second).toBe("2.2.1")
      expect(staffApi.getAppVersion).toHaveBeenCalledTimes(1)
    })

    it("preserves prerelease suffix when fetched from backend", async () => {
      vi.mocked(staffApi.getAppVersion).mockResolvedValueOnce("2.2.1-internal-rc1")

      const version = await fetchAppVersion()
      expect(version).toBe("2.2.1-internal-rc1")
    })

    it("fails safe to 'Version unavailable' when IPC throws", async () => {
      vi.mocked(staffApi.getAppVersion).mockRejectedValueOnce(new Error("IPC connection failed"))

      const version = await fetchAppVersion()
      expect(version).toBe(FALLBACK_APP_VERSION)
    })

    it("fails safe to 'Version unavailable' when returned version is blank", async () => {
      vi.mocked(staffApi.getAppVersion).mockResolvedValueOnce("   ")

      const version = await fetchAppVersion()
      expect(version).toBe(FALLBACK_APP_VERSION)
    })
  })

  describe("useAppVersion hook", () => {
    it("resolves and provides displayVersion from runtime source", async () => {
      vi.mocked(staffApi.getAppVersion).mockResolvedValueOnce("2.2.1")

      const { result } = renderHook(() => useAppVersion())

      await waitFor(() => {
        expect(result.current.version).toBe("2.2.1")
        expect(result.current.displayVersion).toBe("Version 2.2.1")
        expect(result.current.isLoading).toBe(false)
      })
    })

    it("provides fallback displayVersion when retrieval fails", async () => {
      vi.mocked(staffApi.getAppVersion).mockRejectedValueOnce(new Error("No Tauri runtime"))

      const { result } = renderHook(() => useAppVersion())

      await waitFor(() => {
        expect(result.current.version).toBe(FALLBACK_APP_VERSION)
        expect(result.current.displayVersion).toBe(FALLBACK_APP_VERSION)
        expect(result.current.isLoading).toBe(false)
      })
    })
  })
})
