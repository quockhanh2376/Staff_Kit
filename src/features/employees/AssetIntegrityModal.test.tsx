import { fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { AssetIntegrityModal } from "./AssetIntegrityModal"
import type { AssetIntegrityCheckResult } from "../../types/staff"

describe("AssetIntegrityModal", () => {
    const mockOnClose = vi.fn()
    const mockOnClearHighlights = vi.fn()

    const cleanResult: AssetIntegrityCheckResult = {
        isClean: true,
        totalAssetsChecked: 150,
        totalActiveLoansChecked: 120,
        totalEmployeesChecked: 100,
        affectedEmployeeIds: [],
        findings: [],
    }

    const findingResult: AssetIntegrityCheckResult = {
        isClean: false,
        totalAssetsChecked: 150,
        totalActiveLoansChecked: 120,
        totalEmployeesChecked: 100,
        affectedEmployeeIds: [1, 2],
        findings: [
            {
                id: "duplicate_holder_VNMON614",
                category: "duplicate_holder",
                severity: "error",
                assetCode: "VNMON614",
                assetId: 10,
                assetName: "Dell 24 Monitor",
                affectedEmployees: [
                    { id: 1, employeeId: "ASWVN1001", fullName: "Alice Nguyen", staffGroup: "employee_list" },
                    { id: 2, employeeId: "ASWVN1002", fullName: "Bob Tran", staffGroup: "employee_list" },
                ],
                reason: "Canonical asset 'VNMON614' is actively assigned to 2 different employees",
                recommendation: "Perform physical inventory check to confirm true holder",
            },
            {
                id: "status_mismatch_active_loan_20",
                category: "status_mismatch",
                severity: "warning",
                assetCode: "VNMON540",
                assetId: 20,
                assetName: "HP 27",
                affectedEmployees: [
                    { id: 1, employeeId: "ASWVN1001", fullName: "Alice Nguyen", staffGroup: "employee_list" },
                ],
                reason: "Asset 'VNMON540' has active loan but status is 'in_stock'",
                recommendation: "Update asset status to 'assigned'",
            },
        ],
    }

    beforeEach(() => {
        vi.clearAllMocks()
    })

    it("does not render when open is false", () => {
        render(
            <AssetIntegrityModal
                open={false}
                result={cleanResult}
                computerDuplicatesCount={0}
                onClose={mockOnClose}
            />,
        )
        expect(screen.queryByRole("dialog")).toBeNull()
    })

    it("renders PASS state when database is clean and no duplicates exist", () => {
        render(
            <AssetIntegrityModal
                open={true}
                result={cleanResult}
                computerDuplicatesCount={0}
                onClose={mockOnClose}
            />,
        )

        expect(screen.getByText("Asset & Assignment Integrity Check")).toBeTruthy()
        expect(screen.getByText("PASS")).toBeTruthy()
        expect(screen.getByText("All Integrity Checks Passed")).toBeTruthy()
        expect(screen.getByText("150")).toBeTruthy()
        expect(screen.getByText("120")).toBeTruthy()
        expect(screen.getByText("100")).toBeTruthy()
        expect(screen.getByText(/Read-only validation/i)).toBeTruthy()
    })

    it("renders findings table when issues are detected", () => {
        render(
            <AssetIntegrityModal
                open={true}
                result={findingResult}
                computerDuplicatesCount={0}
                onClose={mockOnClose}
                onClearHighlights={mockOnClearHighlights}
            />,
        )

        expect(screen.getByText("2 Findings")).toBeTruthy()
        expect(screen.getAllByText("Duplicate Active Holder").length).toBeGreaterThan(0)
        expect(screen.getAllByText("Status Mismatch").length).toBeGreaterThan(0)
        expect(screen.getByText("VNMON614")).toBeTruthy()
        expect(screen.getByText("VNMON540")).toBeTruthy()
        expect(screen.getAllByText(/Alice Nguyen/).length).toBeGreaterThan(0)
        expect(screen.getByText(/Bob Tran/)).toBeTruthy()
    })

    it("filters findings by category when filter changes", () => {
        render(
            <AssetIntegrityModal
                open={true}
                result={findingResult}
                computerDuplicatesCount={0}
                onClose={mockOnClose}
            />,
        )

        const select = screen.getByRole("combobox")
        fireEvent.change(select, { target: { value: "status_mismatch" } })

        expect(screen.queryByText("VNMON614")).toBeNull()
        expect(screen.getByText("VNMON540")).toBeTruthy()
    })

    it("invokes onClose on Close button click and Escape key", () => {
        render(
            <AssetIntegrityModal
                open={true}
                result={cleanResult}
                computerDuplicatesCount={0}
                onClose={mockOnClose}
            />,
        )

        fireEvent.click(screen.getByText("Close"))
        expect(mockOnClose).toHaveBeenCalledTimes(1)

        fireEvent.keyDown(window, { key: "Escape" })
        expect(mockOnClose).toHaveBeenCalledTimes(2)
    })

    it("invokes onClearHighlights when button is clicked", () => {
        render(
            <AssetIntegrityModal
                open={true}
                result={findingResult}
                computerDuplicatesCount={0}
                onClose={mockOnClose}
                onClearHighlights={mockOnClearHighlights}
            />,
        )

        fireEvent.click(screen.getByText("Clear Table Highlights"))
        expect(mockOnClearHighlights).toHaveBeenCalledTimes(1)
    })
})
