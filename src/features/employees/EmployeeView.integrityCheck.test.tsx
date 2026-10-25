import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { EmployeeView } from "./EmployeeView"
import { staffApi } from "../../services/staff-api"
import type { EmployeeRecord, AssetIntegrityCheckResult } from "../../types/staff"
import type { UiColumnDefinition } from "../../types/app"

vi.mock("../../services/staff-api", () => ({
    staffApi: {
        searchEmployees: vi.fn(),
        checkAssetIntegrity: vi.fn(),
        moveEmployeesGroup: vi.fn(),
        getEmployeesByGroup: vi.fn().mockResolvedValue([]),
    },
}))

vi.mock("./MissingEmployeesModal", () => ({
    MissingEmployeesModal: () => null,
}))

describe("EmployeeView — Asset & Integrity Check Integration", () => {
    const mockColumns: UiColumnDefinition[] = [
        { key: "fullName", label: "Full Name", source: "core" },
        { key: "employeeId", label: "Staff ID", source: "core" },
        { key: "computerName", label: "Computer Name", source: "core" },
    ]

    const mockEmployee1: EmployeeRecord = {
        id: 101,
        employeeId: "ASWVN1001",
        fullName: "Alice Nguyen",
        nickName: null,
        teamId: null,
        teamName: "Core",
        project: null,
        jobTitle: "Engineer",
        email: "alice@example.com",
        cellphone: null,
        dateOfBirth: null,
        gender: null,
        aswStartDate: null,
        clientStartDate: null,
        contractEndDate: null,
        clientYearOfServices: null,
        startDate: null,
        computerName: "VNLAP101",
        storedComputerName: null,
        notes: null,
        staffGroup: "employee_list",
        isFloating: false,
        lifecycleStatus: null,
        dynamicFields: {},
        updatedAt: "2026-10-25T00:00:00Z",
    }

    const mockEmployee2: EmployeeRecord = {
        id: 102,
        employeeId: "ASWVN1002",
        fullName: "Bob Tran",
        nickName: null,
        teamId: null,
        teamName: "Core",
        project: null,
        jobTitle: "Designer",
        email: "bob@example.com",
        cellphone: null,
        dateOfBirth: null,
        gender: null,
        aswStartDate: null,
        clientStartDate: null,
        contractEndDate: null,
        clientYearOfServices: null,
        startDate: null,
        computerName: "VNLAP102",
        storedComputerName: null,
        notes: null,
        staffGroup: "employee_list",
        isFloating: false,
        lifecycleStatus: null,
        dynamicFields: {},
        updatedAt: "2026-10-25T00:00:00Z",
    }

    const defaultProps = () => ({
        employeeState: {
            employees: [mockEmployee1, mockEmployee2],
            totalEmployees: 2,
            currentPage: 1,
            rowsPerPage: 25,
            staffGroupFilter: "employee_list",
            selectedTeamId: null,
            searchTerm: "",
            setSearchTerm: vi.fn(),
            searchQuery: "",
            sortField: "fullName",
            sortDirection: "asc" as const,
            isLoading: false,
            error: null,
            mssqlSyncPreview: null,
            setCurrentPage: vi.fn(),
            setRowsPerPage: vi.fn(),
            setStaffGroupFilter: vi.fn(),
            setSelectedTeamId: vi.fn(),
            setSearchQuery: vi.fn(),
            handleSort: vi.fn(),
            fetchEmployees: vi.fn(),
            fetchAllFilteredEmployees: vi.fn().mockResolvedValue([mockEmployee1, mockEmployee2]),
            applyLocalEmployeeListDelta: vi.fn(),
        } as unknown as Parameters<typeof EmployeeView>[0]["employeeState"],
        columnState: {
            visibleColumns: mockColumns,
            availableColumns: mockColumns,
            allColumns: mockColumns,
            columnWidths: {},
            resolvedColumnWidths: { fullName: 170, employeeId: 170, computerName: 170 },
            isReordering: false,
            toggleColumnVisibility: vi.fn(),
            moveColumn: vi.fn(),
            resetToDefault: vi.fn(),
            handleColumnResizeMouseDown: vi.fn(),
            resetColumnWidths: vi.fn(),
        } as unknown as Parameters<typeof EmployeeView>[0]["columnState"],
        tableEdit: {
            isTableEditMode: false,
            hasPendingTableEdits: false,
            isSavingTableEdits: false,
            isMovingEmployees: false,
            canMoveSelectedRows: false,
            moveTargetGroup: "employee_list",
            selectedEmployeeIdsToMove: new Set<number>(),
            selectedMoveEmployeeIds: [],
            selectedMoveEmployeeIdSet: new Set<number>(),
            currentPageEmployeeIds: [101, 102],
            isCurrentPageFullySelected: false,
            selectedOnCurrentPageCount: 0,
            editedRowCount: 0,
            tableEditDrafts: {},
            activeTableEditCell: null,
            editingRows: {},
            rowErrors: {},
            handleToggleTableEditMode: vi.fn(),
            handleSaveTableEdits: vi.fn(),
            handleMoveSelectedEmployees: vi.fn(),
            setMoveTargetGroup: vi.fn(),
            toggleSelectAllForMove: vi.fn(),
            toggleEmployeeForMove: vi.fn(),
            enterTableEditMode: vi.fn(),
            enterTableEditModeForEmployee: vi.fn(),
            cancelTableEditMode: vi.fn(),
            saveTableEdits: vi.fn(),
            executeMoveEmployees: vi.fn(),
            clearMoveSelection: vi.fn(),
            handleCellChange: vi.fn(),
            isColumnSortable: vi.fn().mockReturnValue(false),
            isTableEditableColumn: vi.fn().mockReturnValue(true),
            getDraftCellText: vi.fn().mockReturnValue(""),
            getEditableCellText: vi.fn().mockReturnValue(""),
            readCellValue: (emp: EmployeeRecord, key: string) => String(emp[key as keyof EmployeeRecord] ?? "-"),
            readRawCellText: (emp: EmployeeRecord, key: string) => String(emp[key as keyof EmployeeRecord] ?? ""),
            selectCurrentPageEmployees: vi.fn(),
            selectAllEmployees: vi.fn(),
            unselectCurrentPageEmployees: vi.fn(),
            toggleMoveEmployeeSelection: vi.fn(),
        } as unknown as Parameters<typeof EmployeeView>[0]["tableEdit"],
        teamState: {
            teams: [],
            isLoading: false,
            error: null,
            fetchTeams: vi.fn(),
            createTeam: vi.fn(),
            updateTeam: vi.fn(),
            deleteTeam: vi.fn(),
        },
        canEditEmployeeTable: true,
        canEditEmployeeComputerName: true,
        isAdminAccount: true,
        selectedGroupLabel: "All Staff",
        selectedGroupTotal: 2,
        setGlobalError: vi.fn(),
    })

    beforeEach(() => {
        vi.clearAllMocks()
    })

    it("triggers checkAssetIntegrity and shows PASS state when clean", async () => {
        const cleanResult: AssetIntegrityCheckResult = {
            isClean: true,
            totalAssetsChecked: 50,
            totalActiveLoansChecked: 30,
            totalEmployeesChecked: 2,
            affectedEmployeeIds: [],
            findings: [],
        }

        vi.mocked(staffApi.checkAssetIntegrity).mockResolvedValue(cleanResult)
        vi.mocked(staffApi.searchEmployees).mockResolvedValue({
            items: [mockEmployee1, mockEmployee2],
            total: 2,
            limit: 99999,
            offset: 0,
        })

        render(<EmployeeView {...defaultProps()} />)

        const checkBtn = screen.getByRole("button", { name: /Check/i })
        fireEvent.click(checkBtn)

        await waitFor(() => {
            expect(staffApi.checkAssetIntegrity).toHaveBeenCalledTimes(1)
            expect(staffApi.searchEmployees).toHaveBeenCalledTimes(1)
        })

        await waitFor(() => {
            expect(screen.getByText("Check: PASS")).toBeTruthy()
            expect(screen.getByText("All Integrity Checks Passed")).toBeTruthy()
        })
    })

    it("triggers checkAssetIntegrity and surfaces duplicate active holder findings", async () => {
        const findingResult: AssetIntegrityCheckResult = {
            isClean: false,
            totalAssetsChecked: 50,
            totalActiveLoansChecked: 30,
            totalEmployeesChecked: 2,
            affectedEmployeeIds: [101, 102],
            findings: [
                {
                    id: "duplicate_holder_VNMON614",
                    category: "duplicate_holder",
                    severity: "error",
                    assetCode: "VNMON614",
                    assetId: 10,
                    assetName: "Dell 24 Monitor",
                    affectedEmployees: [
                        { id: 101, employeeId: "ASWVN1001", fullName: "Alice Nguyen", staffGroup: "employee_list" },
                        { id: 102, employeeId: "ASWVN1002", fullName: "Bob Tran", staffGroup: "employee_list" },
                    ],
                    reason: "Canonical asset 'VNMON614' is actively assigned to 2 different employees",
                    recommendation: "Perform physical inventory check to confirm true holder",
                },
            ],
        }

        vi.mocked(staffApi.checkAssetIntegrity).mockResolvedValue(findingResult)
        vi.mocked(staffApi.searchEmployees).mockResolvedValue({
            items: [mockEmployee1, mockEmployee2],
            total: 2,
            limit: 99999,
            offset: 0,
        })

        render(<EmployeeView {...defaultProps()} />)

        const checkBtn = screen.getByRole("button", { name: /Check/i })
        fireEvent.click(checkBtn)

        await waitFor(() => {
            expect(screen.getByText("2 Issues")).toBeTruthy()
            expect(screen.getByText("1 Finding")).toBeTruthy()
            expect(screen.getAllByText("Duplicate Active Holder").length).toBeGreaterThan(0)
            expect(screen.getByText("VNMON614")).toBeTruthy()
        })
    })
})
