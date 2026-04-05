import type { BorrowLanSettings } from "../../types/staff"

const EMPTY_BORROW_URL_PREVIEW = "Borrow URL will appear here after host is detected or entered."
const INVALID_BORROW_PORT_PREVIEW = "Enter a valid port to preview the Borrow URL."

export function chooseBorrowLanHostInput(savedHost: string, detectedHost: string | null): string {
  const detected = detectedHost?.trim() ?? ""
  if (detected.length > 0) {
    return detected
  }

  return savedHost.trim()
}

export function formatBorrowLanUrlHost(host: string): string {
  const trimmed = host.trim()
  if (!trimmed) {
    return trimmed
  }
  if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
    return trimmed
  }
  if (trimmed.includes(":")) {
    return `[${trimmed}]`
  }
  return trimmed
}

export function buildBorrowLanUrl(host: string, port: number): string {
  return `http://${formatBorrowLanUrlHost(host.trim())}:${port}/borrow`
}

export function applyDetectedBorrowLanSettings(
  settings: BorrowLanSettings | null,
  detectedHost: string | null,
): BorrowLanSettings | null {
  const normalizedHost = detectedHost?.trim() ?? ""
  if (!settings || !normalizedHost) {
    return settings
  }

  return {
    ...settings,
    host: normalizedHost,
    borrowUrl: buildBorrowLanUrl(normalizedHost, settings.port),
  }
}

export function buildBorrowLanUrlPreview(host: string, portInput: string): string {
  const normalizedHost = host.trim()
  if (!normalizedHost) {
    return EMPTY_BORROW_URL_PREVIEW
  }

  const port = Number.parseInt(portInput.trim(), 10)
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    return INVALID_BORROW_PORT_PREVIEW
  }

  return buildBorrowLanUrl(normalizedHost, port)
}
