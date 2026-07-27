import { afterEach, describe, expect, it, vi } from "vitest"
import { readFileSync } from "node:fs"
import { resolve } from "node:path"
import { act, createEvent, fireEvent, render, screen, waitFor } from "@testing-library/react"
import "@testing-library/jest-dom/vitest"
import { COPIED_STATE_DURATION_MS, EmailCopyCell } from "./EmailCopyCell.tsx"

const stylesheet = readFileSync(resolve(process.cwd(), "src/index.css"), "utf8")

function clipboardWriter() {
    return {
        writeText: vi.fn<(value: string) => Promise<void>>().mockResolvedValue(undefined),
    }
}

describe("EmailCopyCell", () => {
    afterEach(() => {
        vi.useRealTimers()
    })

    it("copies the trimmed email on click without bubbling to the parent", async () => {
        const clipboard = clipboardWriter()
        const parentClick = vi.fn()
        render(
            <div onClick={parentClick}>
                <EmailCopyCell email="  Person@Example.COM  " clipboard={clipboard} />
            </div>,
        )

        const button = screen.getByRole("button", { name: "Copy email Person@Example.COM" })
        button.focus()
        fireEvent.click(button)

        await waitFor(() => {
            expect(clipboard.writeText).toHaveBeenCalledWith("Person@Example.COM")
            expect(button).toHaveAttribute("title", "Copied")
            expect(button).toHaveClass("email-copy-cell-copied")
            expect(button).not.toHaveClass("email-copy-cell:hover")
            expect(button).not.toHaveClass("email-copy-cell-failed")
            expect(button).toHaveClass("email-copy-cell")
        })
        const copiedRule = stylesheet.match(/\.email-copy-cell-copied\s*\{([^}]*)\}/)?.[1] ?? ""
        expect(copiedRule).toContain("var(--warning)")
        expect(copiedRule).not.toContain("border-radius")
        expect(parentClick).not.toHaveBeenCalled()
        expect(document.activeElement).toBe(button)
    })

    it("copies on Enter and Space while preventing Space scrolling", async () => {
        const clipboard = clipboardWriter()
        render(<EmailCopyCell email="person@example.com" clipboard={clipboard} />)
        const button = screen.getByRole("button", { name: "Copy email person@example.com" })

        fireEvent.keyDown(button, { key: "Enter" })
        const spaceEvent = createEvent.keyDown(button, { key: " " })
        fireEvent(button, spaceEvent)

        await waitFor(() => {
            expect(clipboard.writeText).toHaveBeenCalledTimes(2)
        })
        expect(spaceEvent.defaultPrevented).toBe(true)
        expect(screen.getByRole("tooltip")).toHaveTextContent("Copied")
    })

    it("shows Copy failed when the clipboard rejects", async () => {
        const clipboard = {
            writeText: vi.fn<(value: string) => Promise<void>>().mockRejectedValue(new Error("denied")),
        }
        render(<EmailCopyCell email="person@example.com" clipboard={clipboard} />)

        fireEvent.click(screen.getByRole("button", { name: "Copy email person@example.com" }))

        await waitFor(() => {
            expect(screen.getByRole("button")).toHaveAttribute("title", "Copy failed")
            expect(screen.getByRole("button")).toHaveClass("email-copy-cell-failed")
        })
    })

    it("keeps the orange copied state through 9999ms and resets at 10000ms", async () => {
        vi.useFakeTimers()
        const clipboard = clipboardWriter()
        render(<EmailCopyCell email="person@example.com" clipboard={clipboard} />)
        const button = screen.getByRole("button", { name: "Copy email person@example.com" })

        fireEvent.click(button)
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        button.focus()
        expect(button).toHaveAttribute("title", "Copied")
        expect(button).toHaveFocus()
        expect(button).toHaveClass("email-copy-cell-copied")

        act(() => {
            vi.advanceTimersByTime(COPIED_STATE_DURATION_MS - 1)
        })
        expect(button).toHaveAttribute("title", "Copied")
        expect(button).toHaveClass("email-copy-cell-copied")
        expect(button).toHaveFocus()
        act(() => {
            vi.advanceTimersByTime(1)
        })
        expect(button).toHaveAttribute("title", "Click to copy email")
        expect(button).toHaveClass("email-copy-cell-idle")
    })

    it("restarts the full copied-state duration on repeated activation", async () => {
        vi.useFakeTimers()
        const clipboard = clipboardWriter()
        render(<EmailCopyCell email="person@example.com" clipboard={clipboard} />)
        const button = screen.getByRole("button", { name: "Copy email person@example.com" })

        fireEvent.click(button)
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        act(() => {
            vi.advanceTimersByTime(COPIED_STATE_DURATION_MS - 1)
        })
        fireEvent.click(button)
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        act(() => {
            vi.advanceTimersByTime(COPIED_STATE_DURATION_MS - 1)
        })
        expect(button).toHaveClass("email-copy-cell-copied")
        expect(button).toHaveAttribute("title", "Copied")
        act(() => {
            vi.advanceTimersByTime(1)
        })
        expect(button).toHaveClass("email-copy-cell-idle")
    })

    it("clears the copied-state timer when unmounted", async () => {
        vi.useFakeTimers()
        const clearTimeoutSpy = vi.spyOn(window, "clearTimeout")
        const clipboard = clipboardWriter()
        const { unmount } = render(<EmailCopyCell email="person@example.com" clipboard={clipboard} />)
        const button = screen.getByRole("button", { name: "Copy email person@example.com" })

        fireEvent.click(button)
        await act(async () => {
            await Promise.resolve()
            await Promise.resolve()
        })
        unmount()
        expect(clearTimeoutSpy).toHaveBeenCalled()
    })

    it("keeps copied state independent for each cell", async () => {
        const first = clipboardWriter()
        const second = clipboardWriter()
        render(
            <>
                <EmailCopyCell email="first@example.com" clipboard={first} />
                <EmailCopyCell email="second@example.com" clipboard={second} />
            </>,
        )

        fireEvent.click(screen.getByRole("button", { name: "Copy email first@example.com" }))
        await waitFor(() => {
            expect(first.writeText).toHaveBeenCalledWith("first@example.com")
            expect(screen.getByRole("button", { name: "Copy email first@example.com" })).toHaveAttribute("title", "Copied")
        })
        expect(screen.getByRole("button", { name: "Copy email second@example.com" })).toHaveAttribute("title", "Click to copy email")
    })
})
