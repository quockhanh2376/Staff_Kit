import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import type { DOMAttributes } from "react"

type IdleCollapseBinding = Pick<
    DOMAttributes<HTMLElement>,
    "onPointerDownCapture" | "onFocusCapture" | "onInputCapture" | "onKeyDownCapture" | "onScrollCapture"
>

export function useIdleCollapse(idleMs: number, initialExpanded = true) {
    const [isExpanded, setExpanded] = useState(initialExpanded)
    const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

    const clearIdleTimer = useCallback(() => {
        if (timeoutRef.current !== null) {
            clearTimeout(timeoutRef.current)
            timeoutRef.current = null
        }
    }, [])

    const notifyActivity = useCallback(() => {
        if (!isExpanded) {
            return
        }

        clearIdleTimer()
        timeoutRef.current = setTimeout(() => {
            setExpanded(false)
        }, idleMs)
    }, [clearIdleTimer, idleMs, isExpanded])

    useEffect(() => {
        if (isExpanded) {
            notifyActivity()
        } else {
            clearIdleTimer()
        }

        return clearIdleTimer
    }, [clearIdleTimer, isExpanded, notifyActivity])

    const bindActivityHandlers = useMemo<IdleCollapseBinding>(
        () => ({
            onPointerDownCapture: notifyActivity,
            onFocusCapture: notifyActivity,
            onInputCapture: notifyActivity,
            onKeyDownCapture: notifyActivity,
            onScrollCapture: notifyActivity,
        }),
        [notifyActivity],
    )

    return {
        isExpanded,
        setExpanded,
        bindActivityHandlers,
        notifyActivity,
    }
}
