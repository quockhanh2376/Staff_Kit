import { render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import "@testing-library/jest-dom/vitest"
import { EmployeeView } from "./EmployeeView"
import type { EmployeeRecord } from "../../types/staff"
import type { UiColumnDefinition } from "../../types/app"

// Mock sub-components/modules to keep EmployeeView isolated
vi.mock("../../services/staff-api", () => ({
    staffApi: {
        moveEmployeesGroup: vi.fn(),
        getEmployeesByGroup: vi.fn().mockResolvedValue([]),
    },
}))

vi.mock("./MissingEmployeesModal", () => ({
    MissingEmployeesModal: () => null,
}))

describe("EmployeeView — HR Lifecycle UI Label Cleanup", () => {
    const mockColumns: UiColumnDefinition[] = [
        { key: "fullName", label: "Full Name", source: "core" },
        { key: "employeeId", label: "Staff ID", source: "core" },
    ]

    const mockEmployee: EmployeeRecord = {
        id: 101,
        employeeId: "ASWVN9999",
        fullName: "Jane Doe",
        nickName: null,
        teamId: null,
        teamName: "Core",
        project: null,
        jobTitle: "Developer",
        email: "jane.doe@example.com",
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
        staffGroup: "offboarding",
        isFloating: false,
        lifecycleStatus: "rehired",
        dynamicFields: {},
        updatedAt: "2026-10-18T00:00:00Z",
    }

    const defaultProps = (staffGroupFilter = "offboarding") => ({
        employeeState: {
            employees: [mockEmployee],
            totalEmployees: 1,
            currentPage: 1,
            rowsPerPage: 25,
            staffGroupFilter,
            searchTerm: "",
            setSearchTerm: vi.fn(),
            teamFilter: "All Teams",
            setTeamFilter: vi.fn(),
            teamFilterSearchTerm: "",
            setTeamFilterSearchTerm: vi.fn(),
            startDateFilter: "",
            setStartDateFilter: vi.fn(),
            employeeSort: null,
            setEmployeeSort: vi.fn(),
            isFloatingFilterActive: false,
            setIsFloatingFilterActive: vi.fn(),
            employeeGroupCounts: {
                employeeList: 1,
                onboarding: 0,
                offboarding: 1,
                internalMovement: 0,
                floating: 0,
                total: 2,
            },
            teams: [],
            isLoadingEmployees: false,
            isLoadingTeams: false,
            isUpdatingEmployeeListFromMssql: false,
            mssqlUpdateMessage: null,
            mssqlUpdateStatus: "idle",
            mssqlSyncPreview: null,
            isApplyingMssqlSync: false,
            setCurrentPage: vi.fn(),
            setRowsPerPage: vi.fn(),
            setStaffGroupFilter: vi.fn(),
            loadEmployees: vi.fn(),
            updateEmployeeListFromMssql: vi.fn(),
            approveMssqlSyncPreview: vi.fn(),
            skipMssqlSyncPreview: vi.fn(),
            refreshMssqlSyncPreview: vi.fn(),
            fetchAllFilteredEmployees: vi.fn().mockResolvedValue([mockEmployee]),
            teamFilterMenuRef: { current: null },
            isTeamFilterMenuOpen: false,
            setTeamFilterMenuOpen: vi.fn(),
        } as unknown as Parameters<typeof EmployeeView>[0]["employeeState"],
        tableEdit: {
            isTableEditMode: true,
            hasPendingTableEdits: false,
            isSavingTableEdits: false,
            isMovingEmployees: false,
            canMoveSelectedRows: true,
            moveTargetGroup: "employee_list",
            selectedEmployeeIdsToMove: new Set([101]),
            selectedMoveEmployeeIds: [101],
            selectedMoveEmployeeIdSet: new Set([101]),
            currentPageEmployeeIds: [101],
            isCurrentPageFullySelected: true,
            selectedOnCurrentPageCount: 1,
            editedRowCount: 0,
            tableEditDrafts: {},
            activeTableEditCell: null,
            handleToggleTableEditMode: vi.fn(),
            handleSaveTableEdits: vi.fn(),
            handleMoveSelectedEmployees: vi.fn(),
            setMoveTargetGroup: vi.fn(),
            clearMoveSelection: vi.fn(),
            enterTableEditModeForEmployee: vi.fn(),
            isColumnSortable: vi.fn().mockReturnValue(false),
            isTableEditableColumn: vi.fn().mockReturnValue(false),
            getDraftCellText: vi.fn().mockReturnValue(""),
            getEditableCellText: vi.fn().mockReturnValue(""),
            readCellValue: (_emp: EmployeeRecord, key: string) => String(mockEmployee[key as keyof EmployeeRecord] ?? "-"),
            readRawCellText: (_emp: EmployeeRecord, key: string) => String(mockEmployee[key as keyof EmployeeRecord] ?? ""),
            selectCurrentPageEmployees: vi.fn(),
            selectAllEmployees: vi.fn(),
            unselectCurrentPageEmployees: vi.fn(),
            toggleMoveEmployeeSelection: vi.fn(),
        } as unknown as Parameters<typeof EmployeeView>[0]["tableEdit"],
        columnState: {
            visibleColumns: mockColumns,
            allColumns: mockColumns,
            columnWidths: {},
            resolvedColumnWidths: { fullName: 170, employeeId: 170 },
            toggleColumnVisibility: vi.fn(),
            setColumnWidth: vi.fn(),
            resetColumnPreferences: vi.fn(),
            isColumnChooserOpen: false,
            setIsColumnChooserOpen: vi.fn(),
            setColumnsDrawerOpen: vi.fn(),
            isColumnsDrawerOpen: false,
        } as unknown as Parameters<typeof EmployeeView>[0]["columnState"],
        canEditEmployeeTable: true,
        canEditEmployeeComputerName: true,
        isAdminAccount: true,
        setGlobalError: vi.fn(),
        selectedGroupLabel: "Offboarding",
        selectedGroupTotal: 1,
    })

    it("does NOT display 'Rehired' badge even when employee lifecycleStatus is 'rehired'", () => {
        render(<EmployeeView {...defaultProps("employee_list")} />)
        // Employee full name is rendered in desktop table and mobile card view
        expect(screen.getAllByText("Jane Doe").length).toBeGreaterThan(0)
        // But no 'Rehired' badge text in the document
        expect(screen.queryByText("Rehired")).not.toBeInTheDocument()
    })

    it("displays 'Employee List' instead of 'Rehire to Employee List' in the group move dropdown", () => {
        render(<EmployeeView {...defaultProps("offboarding")} />)
        // The dropdown option should now be "Employee List"
        const option = screen.getByRole("option", { name: "Employee List" }) as HTMLOptionElement
        expect(option).toBeInTheDocument()
        expect(option.value).toBe("employee_list")
        // "Rehire to Employee List" should not exist
        expect(screen.queryByRole("option", { name: "Rehire to Employee List" })).not.toBeInTheDocument()
    })
})
