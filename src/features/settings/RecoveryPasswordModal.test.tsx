import { fireEvent, render, screen } from "@testing-library/react"
import "@testing-library/jest-dom/vitest"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { RecoveryPasswordModal } from "./RecoveryPasswordModal"

describe("RecoveryPasswordModal", () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it("does not render when isOpen is false", () => {
        render(
            <RecoveryPasswordModal
                isOpen={false}
                isProcessing={false}
                onConfirm={vi.fn()}
                onClose={vi.fn()}
            />
        )
        expect(screen.queryByTestId("recovery-reauth-password-input")).not.toBeInTheDocument()
    })

    it("renders modal with title, explanation, and masked input when isOpen is true", () => {
        render(
            <RecoveryPasswordModal
                isOpen={true}
                isProcessing={false}
                onConfirm={vi.fn()}
                onClose={vi.fn()}
            />
        )

        expect(screen.getByText(/Confirm Password \/ Xác nhận mật khẩu/i)).toBeInTheDocument()
        expect(screen.getByText(/Current Super Admin password is required to generate 8 new Recovery Codes/i)).toBeInTheDocument()

        const input = screen.getByTestId("recovery-reauth-password-input")
        expect(input).toBeInTheDocument()
        expect(input).toHaveAttribute("type", "password")
    })

    it("toggles password visibility with show/hide eye icon button", () => {
        render(
            <RecoveryPasswordModal
                isOpen={true}
                isProcessing={false}
                onConfirm={vi.fn()}
                onClose={vi.fn()}
            />
        )

        const input = screen.getByTestId("recovery-reauth-password-input")
        const toggleBtn = screen.getByTestId("recovery-reauth-toggle-visibility")

        // Initially masked
        expect(input).toHaveAttribute("type", "password")
        expect(toggleBtn).toHaveAttribute("aria-label", "Show password")

        // Click to show password
        fireEvent.click(toggleBtn)
        expect(input).toHaveAttribute("type", "text")
        expect(toggleBtn).toHaveAttribute("aria-label", "Hide password")

        // Click again to re-mask
        fireEvent.click(toggleBtn)
        expect(input).toHaveAttribute("type", "password")
        expect(toggleBtn).toHaveAttribute("aria-label", "Show password")
    })

    it("submits password via Confirm button click and clears password state", () => {
        const onConfirm = vi.fn()
        render(
            <RecoveryPasswordModal
                isOpen={true}
                isProcessing={false}
                onConfirm={onConfirm}
                onClose={vi.fn()}
            />
        )

        const input = screen.getByTestId("recovery-reauth-password-input")
        const confirmBtn = screen.getByTestId("recovery-reauth-confirm-button")

        // Disabled when empty
        expect(confirmBtn).toBeDisabled()

        // Enter password
        fireEvent.change(input, { target: { value: "SuperAdminSecretPass123!" } })
        expect(confirmBtn).toBeEnabled()

        // Click confirm
        fireEvent.click(confirmBtn)
        expect(onConfirm).toHaveBeenCalledWith("SuperAdminSecretPass123!")

        // Password input is cleared from state after submit
        expect(input).toHaveValue("")
    })

    it("submits password via Enter key", () => {
        const onConfirm = vi.fn()
        render(
            <RecoveryPasswordModal
                isOpen={true}
                isProcessing={false}
                onConfirm={onConfirm}
                onClose={vi.fn()}
            />
        )

        const input = screen.getByTestId("recovery-reauth-password-input")
        fireEvent.change(input, { target: { value: "SuperAdminSecretPass123!" } })

        // Submit form with Enter key
        fireEvent.submit(input.closest("form")!)
        expect(onConfirm).toHaveBeenCalledWith("SuperAdminSecretPass123!")
        expect(input).toHaveValue("")
    })

    it("cancels and clears password via Cancel button click", () => {
        const onClose = vi.fn()
        render(
            <RecoveryPasswordModal
                isOpen={true}
                isProcessing={false}
                onConfirm={vi.fn()}
                onClose={onClose}
            />
        )

        const input = screen.getByTestId("recovery-reauth-password-input")
        const cancelBtn = screen.getByTestId("recovery-reauth-cancel-button")

        fireEvent.change(input, { target: { value: "UnfinishedPassword" } })
        expect(input).toHaveValue("UnfinishedPassword")

        fireEvent.click(cancelBtn)
        expect(onClose).toHaveBeenCalledTimes(1)
        expect(input).toHaveValue("")
    })

    it("closes modal on Escape key press", () => {
        const onClose = vi.fn()
        render(
            <RecoveryPasswordModal
                isOpen={true}
                isProcessing={false}
                onConfirm={vi.fn()}
                onClose={onClose}
            />
        )

        fireEvent.keyDown(window, { key: "Escape" })
        expect(onClose).toHaveBeenCalledTimes(1)
    })

    it("disables repeated submission, input and cancel while isProcessing is true", () => {
        const onConfirm = vi.fn()
        const onClose = vi.fn()
        render(
            <RecoveryPasswordModal
                isOpen={true}
                isProcessing={true}
                onConfirm={onConfirm}
                onClose={onClose}
            />
        )

        const input = screen.getByTestId("recovery-reauth-password-input")
        const confirmBtn = screen.getByTestId("recovery-reauth-confirm-button")
        const cancelBtn = screen.getByTestId("recovery-reauth-cancel-button")

        expect(input).toBeDisabled()
        expect(confirmBtn).toBeDisabled()
        expect(confirmBtn).toHaveTextContent("Verifying...")
        expect(cancelBtn).toBeDisabled()

        // Escape key ignored while processing
        fireEvent.keyDown(window, { key: "Escape" })
        expect(onClose).not.toHaveBeenCalled()
    })

    it("displays error alert banner inside modal without clearing modal", () => {
        render(
            <RecoveryPasswordModal
                isOpen={true}
                isProcessing={false}
                error="Incorrect password. Please try again."
                onConfirm={vi.fn()}
                onClose={vi.fn()}
            />
        )

        const errorBanner = screen.getByTestId("recovery-reauth-error")
        expect(errorBanner).toBeInTheDocument()
        expect(errorBanner).toHaveTextContent("Incorrect password. Please try again.")
    })
})
