import { useEffect, useMemo, useRef, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { EmployeeRecord, TeamRecord } from "../../types/staff"
import type { StaffGroupKey, EmployeeSortState } from "../../types/app"
import { getUserErrorMessage } from "../../lib/errorHandling"
import { ALL_TEAMS_OPTION } from "../../lib/constants"

type UseEmployeeStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    reloadToken: number
    setGlobalError: (msg: string | null) => void
}

export type EmployeeState = ReturnType<typeof useEmployeeState>

export function useEmployeeState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
}: UseEmployeeStateOptions) {
    const [employees, setEmployees] = useState<EmployeeRecord[]>([])
    const [teams, setTeams] = useState<TeamRecord[]>([])
    const [totalEmployees, setTotalEmployees] = useState(0)
    const [searchTerm, setSearchTerm] = useState("")
    const [staffGroupFilter, setStaffGroupFilter] = useState<StaffGroupKey>("employee_list")
    const [teamFilter, setTeamFilter] = useState(ALL_TEAMS_OPTION)
    const [teamFilterSearchTerm, setTeamFilterSearchTerm] = useState("")
    const [startDateFilter, setStartDateFilter] = useState("")
    const [employeeSort, setEmployeeSort] = useState<EmployeeSortState>(null)
    const [isTeamFilterMenuOpen, setTeamFilterMenuOpen] = useState(false)
    const [employeeGroupCounts, setEmployeeGroupCounts] = useState({
        employeeList: 0,
        onboarding: 0,
        offboarding: 0,
        internalMovement: 0,
    })
    const [rowsPerPage, setRowsPerPage] = useState(500)
    const [currentPage, setCurrentPage] = useState(1)
    const [isLoadingEmployees, setLoadingEmployees] = useState(false)
    const [isLoadingTeams, setLoadingTeams] = useState(false)
    const [isUpdatingEmployeeListFromMssql, setUpdatingEmployeeListFromMssql] = useState(false)
    const [mssqlRefreshToken, setMssqlRefreshToken] = useState(0)

    const teamFilterMenuRef = useRef<HTMLDivElement | null>(null)

    // Load teams
    useEffect(() => {
        if (!dbReady || !isAuthenticated) return

        let disposed = false

        void (async () => {
            try {
                setLoadingTeams(true)
                const data = await staffApi.listTeams()
                if (!disposed) setTeams(data)
            } catch (error) {
                if (!disposed) setGlobalError(getUserErrorMessage(error))
            } finally {
                if (!disposed) setLoadingTeams(false)
            }
        })()

        return () => { disposed = true }
    }, [dbReady, isAuthenticated, reloadToken, mssqlRefreshToken, setGlobalError])

    // Load group counts
    useEffect(() => {
        if (!dbReady || !isAuthenticated) return

        let disposed = false

        void (async () => {
            try {
                const counts = await staffApi.listEmployeeGroupCounts()
                if (!disposed) setEmployeeGroupCounts(counts)
            } catch (error) {
                if (!disposed) setGlobalError(getUserErrorMessage(error))
            }
        })()

        return () => { disposed = true }
    }, [dbReady, isAuthenticated, reloadToken, mssqlRefreshToken, setGlobalError])

    // Reset to page 1 when filters change
    useEffect(() => {
        setCurrentPage(1)
    }, [searchTerm, staffGroupFilter, teamFilter, startDateFilter, rowsPerPage, employeeSort])

    // Load employees
    useEffect(() => {
        if (!dbReady || !isAuthenticated) return

        let disposed = false

        void (async () => {
            try {
                setLoadingEmployees(true)

                const response = await staffApi.searchEmployees({
                    query: searchTerm.trim() ? searchTerm.trim() : null,
                    staffGroup: staffGroupFilter,
                    teamName: teamFilter === ALL_TEAMS_OPTION ? null : teamFilter,
                    startDateFrom: startDateFilter || null,
                    sortKey: employeeSort?.key ?? null,
                    sortDirection: employeeSort?.direction ?? null,
                    limit: rowsPerPage,
                    offset: (currentPage - 1) * rowsPerPage,
                })

                if (disposed) return
                setEmployees(response.items)
                setTotalEmployees(response.total)
            } catch (error) {
                if (!disposed) setGlobalError(getUserErrorMessage(error))
            } finally {
                if (!disposed) setLoadingEmployees(false)
            }
        })()

        return () => { disposed = true }
    }, [
        dbReady,
        isAuthenticated,
        searchTerm,
        staffGroupFilter,
        teamFilter,
        startDateFilter,
        rowsPerPage,
        currentPage,
        reloadToken,
        mssqlRefreshToken,
        employeeSort,
        setGlobalError,
    ])

    // Clear on logout
    const resetEmployeeStateOnLogout = () => {
        setEmployees([])
        setTeams([])
        setTotalEmployees(0)
        setCurrentPage(1)
        setTeamFilterMenuOpen(false)
    }

    // Close team filter menu on outside click
    useEffect(() => {
        const handlePointerDown = (event: MouseEvent) => {
            const target = event.target as Node | null
            if (teamFilterMenuRef.current && target && !teamFilterMenuRef.current.contains(target)) {
                setTeamFilterMenuOpen(false)
                setTeamFilterSearchTerm("")
            }
        }

        const handleEscape = (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                setTeamFilterMenuOpen(false)
                setTeamFilterSearchTerm("")
            }
        }

        document.addEventListener("mousedown", handlePointerDown)
        document.addEventListener("keydown", handleEscape)
        return () => {
            document.removeEventListener("mousedown", handlePointerDown)
            document.removeEventListener("keydown", handleEscape)
        }
    }, [])

    // ── Derived ──────────────────────────────────────────────────────────────────

    const teamOptions = useMemo(
        () => teams.map((team) => team.name).sort((a, b) => a.localeCompare(b)),
        [teams],
    )

    const teamFilterOptions = useMemo(() => [ALL_TEAMS_OPTION, ...teamOptions], [teamOptions])

    const filteredTeamFilterOptions = useMemo(() => {
        const keyword = teamFilterSearchTerm.trim().toLowerCase()
        if (!keyword) return teamFilterOptions
        return teamFilterOptions.filter((name) => name.toLowerCase().includes(keyword))
    }, [teamFilterOptions, teamFilterSearchTerm])

    const totalPages = useMemo(() => {
        if (totalEmployees <= 0) return 1
        return Math.max(1, Math.ceil(totalEmployees / rowsPerPage))
    }, [rowsPerPage, totalEmployees])

    useEffect(() => {
        if (currentPage > totalPages) {
            setCurrentPage(totalPages)
        }
    }, [currentPage, totalPages])

    // ── Handlers ─────────────────────────────────────────────────────────────────

    const clearFilters = () => {
        setSearchTerm("")
        setTeamFilter(ALL_TEAMS_OPTION)
        setTeamFilterSearchTerm("")
        setStartDateFilter("")
        setTeamFilterMenuOpen(false)
    }

    const selectTeamFilter = (name: string) => {
        setTeamFilter(name)
        setTeamFilterSearchTerm("")
        setTeamFilterMenuOpen(false)
    }

    const commitTypedTeamSelection = () => {
        const keyword = teamFilterSearchTerm.trim().toLowerCase()
        if (keyword.length === 0) return
        const exact = teamFilterOptions.find((name) => name.toLowerCase() === keyword)
        if (exact) {
            selectTeamFilter(exact)
            return
        }
        if (filteredTeamFilterOptions.length === 1) {
            selectTeamFilter(filteredTeamFilterOptions[0])
        }
    }

    const toggleColumnSort = (columnKey: string, sortable: boolean) => {
        if (!sortable) return
        setEmployeeSort((prev) => {
            if (!prev || prev.key !== columnKey) {
                return { key: columnKey, direction: "asc" }
            }
            if (prev.direction === "asc") {
                return { key: columnKey, direction: "desc" }
            }
            return null
        })
    }

    const updateEmployeeListFromMssql = async () => {
        try {
            setUpdatingEmployeeListFromMssql(true)
            setGlobalError(null)
            const defaults = await staffApi.getMssqlConnectionDefaults()
            if (!defaults.host || !defaults.user || !defaults.password) {
                throw new Error("Missing MSSQL configuration. Set STAFFKIT_MSSQL_HOST, STAFFKIT_MSSQL_USER, and STAFFKIT_MSSQL_PASSWORD.")
            }
            const report = await staffApi.importMssqlStaff(
                defaults.host,
                defaults.port,
                defaults.user,
                defaults.password,
                undefined,
                "employee_list",
            )
            console.info("[Staff Kit] MSSQL employee update report:", report)
            setStaffGroupFilter("employee_list")
            setCurrentPage(1)
            setMssqlRefreshToken((value) => value + 1)
        } catch (error) {
            console.error("[Staff Kit] MSSQL employee update failed:", error)
            setGlobalError(getUserErrorMessage(error))
        } finally {
            setUpdatingEmployeeListFromMssql(false)
        }
    }

    return {
        employees,
        teams,
        totalEmployees,
        searchTerm,
        setSearchTerm,
        staffGroupFilter,
        setStaffGroupFilter,
        teamFilter,
        teamFilterSearchTerm,
        setTeamFilterSearchTerm,
        startDateFilter,
        setStartDateFilter,
        employeeSort,
        isTeamFilterMenuOpen,
        setTeamFilterMenuOpen,
        employeeGroupCounts,
        rowsPerPage,
        setRowsPerPage,
        currentPage,
        setCurrentPage,
        totalPages,
        isLoadingEmployees,
        isLoadingTeams,
        isUpdatingEmployeeListFromMssql,
        teamOptions,
        teamFilterOptions,
        filteredTeamFilterOptions,
        teamFilterMenuRef,
        clearFilters,
        selectTeamFilter,
        commitTypedTeamSelection,
        toggleColumnSort,
        updateEmployeeListFromMssql,
        resetEmployeeStateOnLogout,
    }
}
