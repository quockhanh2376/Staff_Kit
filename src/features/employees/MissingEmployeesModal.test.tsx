import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { MissingEmployeesModal } from "./MissingEmployeesModal"
import type { EmployeeRecord } from "../../types/staff"

describe("MissingEmployeesModal", () => {
    const mockOnClose = vi.fn()
    const mockOnOpenEditEmployee = vi.fn()
    const mockOnBulkMoveGroup = vi.fn()
    const mockFetchAllFilteredEmployees = vi.fn()

    const sampleMissingRows = [
        { code: "ASWVN1001", localName: "Nguyen Van A" },
        { code: "ASWVN1002", localName: "Tran Thi B" },
        { code: "ASWVN1003", localName: "Le Van C" },
    ]

    const sampleEmployees: EmployeeRecord[] = [
        {
            id: 1,
            employeeId: "ASWVN1001",
            fullName: "Nguyen Van A",
            nickName: null,
            teamId: null,
            teamName: "Core Team",
            project: null,
            jobTitle: "Software Engineer",
            email: "nva@example.com",
            cellphone: null,
            dateOfBirth: null,
            gender: null,
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
            lifecycleStatus: "active",
            dynamicFields: {},
            updatedAt: "2026-10-18T00:00:00Z",
        },
        {
            id: 2,
            employeeId: "ASWVN1002",
            fullName: "Tran Thi B",
            nickName: null,
            teamId: null,
            teamName: "Design",
            project: null,
            jobTitle: "UI/UX Designer",
            email: "ttb@example.com",
            cellphone: null,
            dateOfBirth: null,
            gender: null,
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
            lifecycleStatus: "active",
            dynamicFields: {},
            updatedAt: "2026-10-18T00:00:00Z",
        },
        {
            id: 3,
            employeeId: "ASWVN1003",
            fullName: "Le Van C",
            nickName: null,
            teamId: null,
            teamName: "DevOps",
            project: null,
            jobTitle: "DevOps Engineer",
            email: "lvc@example.com",
            cellphone: null,
            dateOfBirth: null,
            gender: null,
            aswStartDate: null,
            clientStartDate: null,
            contractEndDate: null,
            clientYearOfServices: null,
            startDate: null,
            computerName: null,
            storedComputerName: null,
            notes: null,
            staffGroup: "employee_list",
            isFloating: true,
            lifecycleStatus: "active",
            dynamicFields: {},
            updatedAt: "2026-10-18T00:00:00Z",
        },
    ]

    beforeEach(() => {
        vi.clearAllMocks()
        mockFetchAllFilteredEmployees.mockResolvedValue(sampleEmployees)
        mockOnBulkMoveGroup.mockResolvedValue(undefined)
    })

    it("renders nothing when open is false", () => {
        render(
            <MissingEmployeesModal
                open={false}
                onClose={mockOnClose}
                missingRows={sampleMissingRows}
                onOpenEditEmployee={mockOnOpenEditEmployee}
                onBulkMoveGroup={mockOnBulkMoveGroup}
                fetchAllFilteredEmployees={mockFetchAllFilteredEmployees}
                canEdit={true}
            />
        )
        expect(screen.queryByRole("dialog")).toBeNull()
    })

    it("renders missing employees with warning banner and counts when open", async () => {
        render(
            <MissingEmployeesModal
                open={true}
                onClose={mockOnClose}
                missingRows={sampleMissingRows}
                onOpenEditEmployee={mockOnOpenEditEmployee}
                onBulkMoveGroup={mockOnBulkMoveGroup}
                fetchAllFilteredEmployees={mockFetchAllFilteredEmployees}
                canEdit={true}
            />
        )

        expect(screen.getByRole("dialog")).toBeTruthy()
        expect(screen.getByText(/Missing Employees \(3\)/i)).toBeTruthy()
        expect(screen.getByText(/Missing in remote HRMS Query B does not automatically mean/i)).toBeTruthy()

        await waitFor(() => {
            expect(screen.getByText("Nguyen Van A")).toBeTruthy()
            expect(screen.getByText("Tran Thi B")).toBeTruthy()
            expect(screen.getByText("Le Van C")).toBeTruthy()
        })
    })

    it("supports search filtering by Staff ID or Name", async () => {
        render(
            <MissingEmployeesModal
                open={true}
                onClose={mockOnClose}
                missingRows={sampleMissingRows}
                onOpenEditEmployee={mockOnOpenEditEmployee}
                onBulkMoveGroup={mockOnBulkMoveGroup}
                fetchAllFilteredEmployees={mockFetchAllFilteredEmployees}
                canEdit={true}
            />
        )

        await waitFor(() => expect(screen.getByText("Nguyen Van A")).toBeTruthy())

        const searchInput = screen.getByLabelText("Search Missing Employees")
        fireEvent.change(searchInput, { target: { value: "Tran Thi" } })

        expect(screen.getByText("Tran Thi B")).toBeTruthy()
        expect(screen.queryByText("Nguyen Van A")).toBeNull()
        expect(screen.queryByText("Le Van C")).toBeNull()
    })

    it("supports single selection and Select All", async () => {
        render(
            <MissingEmployeesModal
                open={true}
                onClose={mockOnClose}
                missingRows={sampleMissingRows}
                onOpenEditEmployee={mockOnOpenEditEmployee}
                onBulkMoveGroup={mockOnBulkMoveGroup}
                fetchAllFilteredEmployees={mockFetchAllFilteredEmployees}
                canEdit={true}
            />
        )

        await waitFor(() => expect(screen.getByText("Nguyen Van A")).toBeTruthy())

        // Check single checkbox
        const cb1 = screen.getByLabelText("Select ASWVN1001")
        fireEvent.click(cb1)
        expect(screen.getByText("1 of 3 selected")).toBeTruthy()

        // Click Select All
        const selectAllCb = screen.getByLabelText("Select all filtered missing employees")
        fireEvent.click(selectAllCb)
        expect(screen.getByText("3 of 3 selected")).toBeTruthy()

        // Uncheck via Select All
        fireEvent.click(selectAllCb)
        expect(screen.getByText("0 of 3 selected")).toBeTruthy()
    })

    it("triggers onOpenEditEmployee when Edit button is clicked", async () => {
        render(
            <MissingEmployeesModal
                open={true}
                onClose={mockOnClose}
                missingRows={sampleMissingRows}
                onOpenEditEmployee={mockOnOpenEditEmployee}
                onBulkMoveGroup={mockOnBulkMoveGroup}
                fetchAllFilteredEmployees={mockFetchAllFilteredEmployees}
                canEdit={true}
            />
        )

        await waitFor(() => expect(screen.getAllByRole("button", { name: "Edit" })).toHaveLength(3))

        fireEvent.click(screen.getAllByRole("button", { name: "Edit" })[0])
        expect(mockOnClose).toHaveBeenCalled()
        expect(mockOnOpenEditEmployee).toHaveBeenCalledWith(sampleEmployees[0])
    })

    it("requires confirmation before applying bulk move group changes", async () => {
        render(
            <MissingEmployeesModal
                open={true}
                onClose={mockOnClose}
                missingRows={sampleMissingRows}
                onOpenEditEmployee={mockOnOpenEditEmployee}
                onBulkMoveGroup={mockOnBulkMoveGroup}
                fetchAllFilteredEmployees={mockFetchAllFilteredEmployees}
                canEdit={true}
            />
        )

        await waitFor(() => expect(screen.getByText("Nguyen Van A")).toBeTruthy())

        // Select two employees
        fireEvent.click(screen.getByLabelText("Select ASWVN1001"))
        fireEvent.click(screen.getByLabelText("Select ASWVN1002"))

        // Select target group Offboard
        const selectGroup = screen.getByLabelText("Target Group")
        fireEvent.change(selectGroup, { target: { value: "offboarding" } })

        // Click Move Group button
        const moveBtn = screen.getByRole("button", { name: "Move Group (2)" })
        fireEvent.click(moveBtn)

        // Confirmation dialog shown
        expect(screen.getByText("Confirm Group Transition")).toBeTruthy()
        expect(screen.getByText(/You are about to move/i)).toBeTruthy()
        expect(screen.getByText(/Offboard \(Resigned\)/i)).toBeTruthy()

        // Cancel causes zero writes
        fireEvent.click(screen.getByRole("button", { name: "Cancel" }))
        expect(mockOnBulkMoveGroup).not.toHaveBeenCalled()
        expect(screen.queryByText("Confirm Group Transition")).toBeNull()

        // Re-open confirmation and approve
        fireEvent.click(screen.getByRole("button", { name: "Move Group (2)" }))
        const approveBtn = screen.getByRole("button", { name: /Approve & Move/i })
        fireEvent.click(approveBtn)

        await waitFor(() => {
            expect(mockOnBulkMoveGroup).toHaveBeenCalledWith([1, 2], "offboarding")
            expect(mockOnClose).toHaveBeenCalled()
        })
    })

    it("displays error message if bulk move group fails and keeps modal open", async () => {
        mockOnBulkMoveGroup.mockRejectedValueOnce(new Error("Database write locked"))

        render(
            <MissingEmployeesModal
                open={true}
                onClose={mockOnClose}
                missingRows={sampleMissingRows}
                onOpenEditEmployee={mockOnOpenEditEmployee}
                onBulkMoveGroup={mockOnBulkMoveGroup}
                fetchAllFilteredEmployees={mockFetchAllFilteredEmployees}
                canEdit={true}
            />
        )

        await waitFor(() => expect(screen.getByText("Nguyen Van A")).toBeTruthy())

        fireEvent.click(screen.getByLabelText("Select ASWVN1001"))
        fireEvent.click(screen.getByRole("button", { name: "Move Group (1)" }))
        fireEvent.click(screen.getByRole("button", { name: /Approve & Move/i }))

        await waitFor(() => {
            expect(screen.getByText("Database write locked")).toBeTruthy()
            expect(mockOnClose).not.toHaveBeenCalled()
        })
    })
})
