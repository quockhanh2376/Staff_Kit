export type ClipboardWriter = {
    writeText: (value: string) => Promise<void>
}

export type CopyEmailResult =
    | { status: "copied"; value: string }
    | { status: "failed" }
    | { status: "inert" }

export function isCopyableEmailValue(value: unknown): value is string {
    return typeof value === "string" && value.trim() !== "" && value.trim() !== "-"
}

export async function copyEmailValue(
    value: unknown,
    clipboard: ClipboardWriter,
): Promise<CopyEmailResult> {
    if (!isCopyableEmailValue(value)) return { status: "inert" }

    const trimmedValue = value.trim()
    try {
        await clipboard.writeText(trimmedValue)
        return { status: "copied", value: trimmedValue }
    } catch {
        return { status: "failed" }
    }
}
