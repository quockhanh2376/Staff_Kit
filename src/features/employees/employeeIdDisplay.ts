export function formatEmployeeIdForDisplay(value: string): string {
  const trimmed = value.trim()
  const match = trimmed.match(/^(?:[A-Za-z]+)(\d+)$/)
  if (!match) {
    return trimmed
  }
  return match[1]
}
