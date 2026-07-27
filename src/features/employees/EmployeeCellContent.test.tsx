import { describe, expect, it, vi } from "vitest"
import { fireEvent, render, screen } from "@testing-library/react"
import "@testing-library/jest-dom/vitest"
import type { UiColumnDefinition } from "../../types/app"
import { EmployeeCellContent } from "./EmployeeCellContent"

const emailColumn: UiColumnDefinition = {
    key: "email",
    label: "Renamed contact column",
    source: "core",
    dataType: "email",
}

const azureColumn: UiColumnDefinition = {
    key: "azure_account",
    label: "Renamed directory column",
    source: "dynamic",
    dataType: "email",
}

const ordinaryColumn: UiColumnDefinition = {
    key: "department",
    label: "Renamed department",
    source: "dynamic",
}

function renderCell(
    column: UiColumnDefinition,
    rawValue: unknown,
    overrides: Partial<React.ComponentProps<typeof EmployeeCellContent>> = {},
) {
    return render(
        <EmployeeCellContent
            column={column}
            rawValue={rawValue}
            displayValue={String(rawValue ?? "-")}
            isEditable={false}
            isActiveCell={false}
            editor={<input aria-label="Edit value" />}
            {...overrides}
        />,
    )
}

describe("EmployeeCellContent", () => {
    it("renders semantic email columns even when labels are renamed", () => {
        renderCell(emailColumn, "person@example.com")
        expect(screen.getByRole("button", { name: "Copy email person@example.com" })).toBeInTheDocument()
    })

    it("renders Azure AD Account through semantic metadata", () => {
        renderCell(azureColumn, "directory@example.com")
        expect(screen.getByRole("button", { name: "Copy email directory@example.com" })).toBeInTheDocument()
    })

    it("preserves search-highlighted display content inside the copy button", () => {
        renderCell(emailColumn, "person@example.com", {
            displayValue: <mark data-testid="email-highlight">person</mark>,
        })
        expect(screen.getByRole("button")).toContainElement(screen.getByTestId("email-highlight"))
    })

    it("keeps ordinary and inert values as plain text", () => {
        const { rerender } = renderCell(ordinaryColumn, "Engineering")
        expect(screen.queryByRole("button")).not.toBeInTheDocument()
        expect(screen.getByText("Engineering")).toBeInTheDocument()

        rerender(
            <EmployeeCellContent
                column={emailColumn}
                rawValue=" - "
                displayValue="-"
                isEditable={false}
                isActiveCell={false}
                editor={<input aria-label="Edit value" />}
            />,
        )
        expect(screen.queryByRole("button")).not.toBeInTheDocument()
        expect(screen.getByText("-")).toBeInTheDocument()
    })

    it("preserves the active inline editor in Edit mode", () => {
        const writeText = vi.fn()
        const previousClipboard = navigator.clipboard
        Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } })
        const editorClick = vi.fn()
        renderCell(emailColumn, "person@example.com", {
            isEditable: true,
            isActiveCell: true,
            editor: <input aria-label="Edit email" onClick={editorClick} />,
        })
        fireEvent.click(screen.getByRole("textbox", { name: "Edit email" }))
        expect(editorClick).toHaveBeenCalledTimes(1)
        expect(writeText).not.toHaveBeenCalled()
        expect(screen.queryByRole("button")).not.toBeInTheDocument()
        Object.defineProperty(navigator, "clipboard", { configurable: true, value: previousClipboard })
    })

    it("restores copy behavior when Edit mode ends", () => {
        const { rerender } = renderCell(emailColumn, "person@example.com", {
            isEditable: true,
            isActiveCell: true,
            editor: <input aria-label="Edit email" />,
        })
        expect(screen.queryByRole("button")).not.toBeInTheDocument()

        rerender(
            <EmployeeCellContent
                column={emailColumn}
                rawValue="person@example.com"
                displayValue="person@example.com"
                isEditable={false}
                isActiveCell={false}
                editor={<input aria-label="Edit email" />}
            />,
        )
        expect(screen.getByRole("button", { name: "Copy email person@example.com" })).toBeInTheDocument()
    })

    it("does not call clipboard during a rerender", () => {
        const writeText = vi.fn()
        const previousClipboard = navigator.clipboard
        Object.defineProperty(navigator, "clipboard", {
            configurable: true,
            value: { writeText },
        })
        const { rerender } = renderCell(emailColumn, "person@example.com")
        rerender(
            <EmployeeCellContent
                column={emailColumn}
                rawValue="person@example.com"
                displayValue={<mark>person@example.com</mark>}
                isEditable={false}
                isActiveCell={false}
                editor={<input aria-label="Edit email" />}
            />,
        )
        expect(writeText).not.toHaveBeenCalled()
        Object.defineProperty(navigator, "clipboard", {
            configurable: true,
            value: previousClipboard,
        })
    })

    it("allows non-email cell clicks to bubble normally", () => {
        const onClick = vi.fn()
        render(
            <div onClick={onClick}>
                <EmployeeCellContent
                    column={ordinaryColumn}
                    rawValue="Engineering"
                    displayValue="Engineering"
                    isEditable={false}
                    isActiveCell={false}
                    editor={<input aria-label="Edit value" />}
                />
            </div>,
        )
        fireEvent.click(screen.getByText("Engineering"))
        expect(onClick).toHaveBeenCalledTimes(1)
    })
})
