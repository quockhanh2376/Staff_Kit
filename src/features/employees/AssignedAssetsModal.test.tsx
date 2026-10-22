import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { AssignedAssetsModal } from "./AssignedAssetsModal"
import { staffApi } from "../../services/staff-api"
import type { EmployeeAssignedAssetRecord, EmployeeRecord } from "../../types/staff"

vi.mock("../../services/staff-api", () => ({
    staffApi: {
        getEmployeeAssignedAssets: vi.fn(),
    },
}))

describe("AssignedAssetsModal", () => {
    const mockOnClose = vi.fn()

    const mockEmployee: EmployeeRecord = {
        id: 42,
        employeeId: "ASW123",
        fullName: "Jane Doe",
        nickName: null,
        teamId: 1,
        teamName: "Design Systems",
        project: "Alpha",
        jobTitle: "Lead UI",
        email: "jane@example.com",
        cellphone: null,
        dateOfBirth: null,
        gender: "Female",
        aswStartDate: null,
        clientStartDate: null,
        contractEndDate: null,
        clientYearOfServices: null,
        startDate: null,
        computerName: null,
        storedComputerName: null,
        notes: null,
        staffGroup: "employee_list",
        isFloating: false,
        lifecycleStatus: null,
        assignedAssetsCount: 2,
        dynamicFields: {},
        updatedAt: "2026-10-22",
    }

    const mockAssets: EmployeeAssignedAssetRecord[] = [
        {
            assetId: 101,
            assetCode: "LT-2026-001",
            assetName: "MacBook Pro 16",
            categoryName: "Laptop",
            categoryCode: "laptop",
            serialNumber: "C02XYZ123",
            usageLocation: "Floor 4, Pod B",
            borrowedAt: "2026-05-10T08:30:00Z",
        },
        {
            assetId: 102,
            assetCode: "HP-2026-042",
            assetName: "Samsung S24",
            categoryName: "H Phone",
            categoryCode: "hphone",
            serialNumber: "R5CNA9988",
            usageLocation: "Remote",
            borrowedAt: "2026-06-01T09:00:00Z",
        },
    ]

    beforeEach(() => {
        vi.clearAllMocks()
    })

    it("renders nothing when open is false", () => {
        render(
            <AssignedAssetsModal
                open={false}
                employee={mockEmployee}
                onClose={mockOnClose}
            />
        )
        expect(screen.queryByRole("dialog")).toBeNull()
    })

    it("renders nothing when employee is null", () => {
        render(
            <AssignedAssetsModal
                open={true}
                employee={null}
                onClose={mockOnClose}
            />
        )
        expect(screen.queryByRole("dialog")).toBeNull()
    })

    it("fetches and displays assigned assets list", async () => {
        vi.mocked(staffApi.getEmployeeAssignedAssets).mockResolvedValue(mockAssets)

        render(
            <AssignedAssetsModal
                open={true}
                employee={mockEmployee}
                onClose={mockOnClose}
            />
        )

        expect(screen.getByText("Loading assigned assets...")).toBeTruthy()
        expect(staffApi.getEmployeeAssignedAssets).toHaveBeenCalledWith(42)

        await waitFor(() => {
            expect(screen.getByText("Laptop (1)")).toBeTruthy()
            expect(screen.getByText("H Phone (1)")).toBeTruthy()
            expect(screen.getByText("LT-2026-001")).toBeTruthy()
            expect(screen.getByText("MacBook Pro 16")).toBeTruthy()
            expect(screen.getByText("C02XYZ123")).toBeTruthy()
            expect(screen.getByText("Floor 4, Pod B")).toBeTruthy()
            expect(screen.getByText("HP-2026-042")).toBeTruthy()
            expect(screen.getByText("Samsung S24")).toBeTruthy()
            expect(screen.getByText("Remote")).toBeTruthy()
        })

        expect(screen.getByText("Total assigned assets:")).toBeTruthy()
        expect(screen.getAllByText("2").length).toBeGreaterThanOrEqual(1)
    })

    it("groups multiple assets by category and displays individual mixed locations", async () => {
        const multipleAssets: EmployeeAssignedAssetRecord[] = [
            {
                assetId: 201,
                assetCode: "MON-001",
                assetName: "Dell UltraSharp 27",
                categoryName: "Monitor",
                categoryCode: "monitor",
                serialNumber: "SN-MON-1",
                usageLocation: "Tại nhà",
                borrowedAt: "2026-05-10T08:30:00Z",
            },
            {
                assetId: 202,
                assetCode: "MON-002",
                assetName: "Dell UltraSharp 27",
                categoryName: "Monitor",
                categoryCode: "monitor",
                serialNumber: "SN-MON-2",
                usageLocation: "Tại công ty",
                borrowedAt: "2026-05-11T08:30:00Z",
            },
            {
                assetId: 203,
                assetCode: "LT-001",
                assetName: "ThinkPad X1",
                categoryName: "Laptop",
                categoryCode: "laptop",
                serialNumber: "SN-LT-1",
                usageLocation: "Tại công ty",
                borrowedAt: "2026-05-12T08:30:00Z",
            },
        ]

        vi.mocked(staffApi.getEmployeeAssignedAssets).mockResolvedValue(multipleAssets)

        render(
            <AssignedAssetsModal
                open={true}
                employee={mockEmployee}
                onClose={mockOnClose}
            />
        )

        await waitFor(() => {
            expect(screen.getByText("Monitor (2)")).toBeTruthy()
            expect(screen.getByText("Laptop (1)")).toBeTruthy()
            expect(screen.getByText("MON-001")).toBeTruthy()
            expect(screen.getByText("MON-002")).toBeTruthy()
            expect(screen.getByText("Tại nhà")).toBeTruthy()
            expect(screen.getAllByText("Tại công ty").length).toBe(2)
        })

        expect(screen.getByText("Total assigned assets:")).toBeTruthy()
        expect(screen.getByText("3")).toBeTruthy()
    })

    it("displays empty state when employee has no assigned assets", async () => {
        vi.mocked(staffApi.getEmployeeAssignedAssets).mockResolvedValue([])

        render(
            <AssignedAssetsModal
                open={true}
                employee={mockEmployee}
                onClose={mockOnClose}
            />
        )

        await waitFor(() => {
            expect(screen.getByText("No active assigned assets")).toBeTruthy()
            expect(screen.getByText("This employee currently has no active asset loans.")).toBeTruthy()
        })
    })

    it("displays error message when fetch fails", async () => {
        vi.mocked(staffApi.getEmployeeAssignedAssets).mockRejectedValue(new Error("Failed to load assets"))

        render(
            <AssignedAssetsModal
                open={true}
                employee={mockEmployee}
                onClose={mockOnClose}
            />
        )

        await waitFor(() => {
            expect(screen.getByText("Failed to load assets")).toBeTruthy()
        })
    })

    it("calls onClose when Close button is clicked", async () => {
        vi.mocked(staffApi.getEmployeeAssignedAssets).mockResolvedValue([])

        render(
            <AssignedAssetsModal
                open={true}
                employee={mockEmployee}
                onClose={mockOnClose}
            />
        )

        await waitFor(() => {
            expect(screen.getByText("No active assigned assets")).toBeTruthy()
        })

        const closeBtn = screen.getByRole("button", { name: "Close" })
        fireEvent.click(closeBtn)
        expect(mockOnClose).toHaveBeenCalledTimes(1)
    })

    it("calls onClose when Escape key is pressed", async () => {
        vi.mocked(staffApi.getEmployeeAssignedAssets).mockResolvedValue([])

        render(
            <AssignedAssetsModal
                open={true}
                employee={mockEmployee}
                onClose={mockOnClose}
            />
        )

        await waitFor(() => {
            expect(screen.getByText("No active assigned assets")).toBeTruthy()
        })

        fireEvent.keyDown(window, { key: "Escape" })
        expect(mockOnClose).toHaveBeenCalledTimes(1)
    })
})
