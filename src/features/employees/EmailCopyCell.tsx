import { useEffect, useRef, useState } from "react"
import type { ReactNode } from "react"
import type { KeyboardEvent, MouseEvent } from "react"
import { copyEmailValue, type ClipboardWriter } from "./emailCopyUtils"

type EmailCopyCellProps = {
    email: string
    children?: ReactNode
    clipboard?: ClipboardWriter
}

type CopyState = "idle" | "copied" | "failed"

export const COPIED_STATE_DURATION_MS = 10_000

export function EmailCopyCell({ email, children = email, clipboard }: EmailCopyCellProps) {
    const [copyState, setCopyState] = useState<CopyState>("idle")
    const resetTimerRef = useRef<number | null>(null)

    useEffect(() => () => {
        if (resetTimerRef.current !== null) {
            window.clearTimeout(resetTimerRef.current)
        }
    }, [])

    const activate = async () => {
        if (resetTimerRef.current !== null) {
            window.clearTimeout(resetTimerRef.current)
            resetTimerRef.current = null
        }

        const targetClipboard = clipboard ?? (
            typeof navigator !== "undefined" ? navigator.clipboard : undefined
        )
        const result = targetClipboard
            ? await copyEmailValue(email, targetClipboard)
            : { status: "failed" as const }

        if (result.status === "copied") {
            setCopyState("copied")
            resetTimerRef.current = window.setTimeout(() => {
                setCopyState("idle")
                resetTimerRef.current = null
            }, COPIED_STATE_DURATION_MS)
        } else if (result.status === "failed") {
            setCopyState("failed")
        }
    }

    const handleClick = (event: MouseEvent<HTMLButtonElement>) => {
        event.stopPropagation()
        void activate()
    }

    const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
        if (event.key !== "Enter" && event.key !== " ") return
        event.preventDefault()
        event.stopPropagation()
        void activate()
    }

    const title =
        copyState === "copied"
            ? "Copied"
            : copyState === "failed"
                ? "Copy failed"
                : "Click to copy email"

    return (
        <span className="email-copy-cell-wrapper">
            <button
                type="button"
                className={"email-copy-cell email-copy-cell-" + copyState}
                title={title}
                aria-label={"Copy email " + email}
                onClick={handleClick}
                onKeyDown={handleKeyDown}
            >
                {children}
            </button>
            <span className="email-copy-tooltip" role="tooltip">
                {title}
            </span>
        </span>
    )
}
