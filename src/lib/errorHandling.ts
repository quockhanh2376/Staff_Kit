/**
 * Centralized error handling utilities.
 *
 * Distinguishes between:
 * - **Business errors**: user-facing messages from known domain rules (e.g. "Asset code already exists")
 * - **Technical errors**: unexpected failures that should be logged but shown with a generic message
 *
 * Usage in hooks:
 *   catch (error) { setGlobalError(classifyError(error).userMessage) }
 */

// ── Business Error ───────────────────────────────────────────────────────────

/** Known business-rule error prefixes returned from Tauri commands */
const BUSINESS_ERROR_PREFIXES = [
  "Duplicate",
  "Not found",
  "Already exists",
  "Cannot",
  "Invalid",
  "Required",
  "Password",
  "Recovery code",
  "Account",
  "Asset code",
  "Team",
  "Column",
  "Import",
  "Batch",
  "Category",
  "No valid",
] as const

/**
 * Classified error result with both the raw message (for logging)
 * and a user-friendly message (for display).
 */
export type ClassifiedError = {
  /** Whether this is a known business-rule violation */
  isBusiness: boolean
  /** Raw error message (useful for console/audit logging) */
  rawMessage: string
  /** User-facing message — actionable and non-technical */
  userMessage: string
}

/**
 * Extracts a string message from any thrown value.
 * Handles Error instances, strings, and unknown types.
 */
function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message
  }
  return String(error)
}

/**
 * Classifies an error as business or technical.
 *
 * Business errors are returned verbatim as they already contain actionable
 * information. Technical errors are replaced with a generic message to
 * avoid exposing implementation details to end users.
 *
 * @param error - The caught error value
 * @returns ClassifiedError with user-safe messaging
 */
export function classifyError(error: unknown): ClassifiedError {
  const rawMessage = getErrorMessage(error)

  const isBusiness = BUSINESS_ERROR_PREFIXES.some(
    (prefix) => rawMessage.startsWith(prefix),
  )

  if (isBusiness) {
    return { isBusiness: true, rawMessage, userMessage: rawMessage }
  }

  // Technical error — log the original error (preserves stack), show generic to user
  console.error("[Staff Kit] Unexpected error:", error instanceof Error ? error : rawMessage)
  return {
    isBusiness: false,
    rawMessage,
    userMessage: "An unexpected error occurred. Please try again or contact support.",
  }
}

/**
 * Convenience wrapper: extracts just the user-facing message from any error.
 * Drop-in replacement for the previous `getErrorMessage` in catch blocks.
 */
export function getUserErrorMessage(error: unknown): string {
  return classifyError(error).userMessage
}
