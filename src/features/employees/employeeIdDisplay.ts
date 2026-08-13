export function formatEmployeeIdForDisplay(value: string): string {
  const trimmed = value.trim()
  const match = trimmed.match(/^ASWVN(\d+)$/i)
  if (!match) {
    return trimmed
  }
  return match[1]
}
