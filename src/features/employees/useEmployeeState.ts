import { useCallback, useEffect, useMemo, useRef, useState } from "react"
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
    const [mssqlUpdateMessage, setMssqlUpdateMessage] = useState<string | null>(null)
    const [mssqlUpdateStatus, setMssqlUpdateStatus] = useState<"idle" | "success" | "error">("idle")
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

    const fetchAllFilteredEmployees = useCallback(async () => {
        const items: EmployeeRecord[] = []
        const limit = 5000
        let offset = 0
        let total = totalEmployees
        do {
            const response = await staffApi.searchEmployees({
                query: searchTerm.trim() ? searchTerm.trim() : null,
                staffGroup: staffGroupFilter,
                teamName: teamFilter === ALL_TEAMS_OPTION ? null : teamFilter,
                startDateFrom: startDateFilter || null,
                sortKey: employeeSort?.key ?? null,
                sortDirection: employeeSort?.direction ?? null,
                limit,
                offset,
            })
            items.push(...response.items)
            total = response.total
            offset += response.items.length
            if (response.items.length === 0) break
        } while (offset < total)
        return items
    }, [employeeSort, searchTerm, staffGroupFilter, startDateFilter, teamFilter, totalEmployees])

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
        if (isUpdatingEmployeeListFromMssql) return

        try {
            setUpdatingEmployeeListFromMssql(true)
            setMssqlUpdateMessage(null)
            setMssqlUpdateStatus("idle")
            setGlobalError(null)
            const defaults = await staffApi.getMssqlConnectionDefaults()
            const host = defaults.host.trim()
            const user = defaults.user.trim()
            if (!host || !user || !defaults.password) {
                throw new Error("Missing MSSQL configuration. Open Settings > Import from MSSQL, or set STAFFKIT_MSSQL_HOST, STAFFKIT_MSSQL_USER, and STAFFKIT_MSSQL_PASSWORD.")
            }
            await staffApi.testMssqlConnection(host, defaults.port, user, defaults.password)
            const report = await staffApi.importMssqlStaff(
                host,
                defaults.port,
                user,
                defaults.password,
                undefined,
                "employee_list",
            )
            console.info("[Staff Kit] MSSQL employee update report:", report)
            setMssqlUpdateStatus("success")
            setMssqlUpdateMessage(`Updated ${report.updated}, imported ${report.imported}, failed ${report.failed}.`)
            setStaffGroupFilter("employee_list")
            setTeamFilter(ALL_TEAMS_OPTION)
            setTeamFilterSearchTerm("")
            setStartDateFilter("")
            setTeamFilterMenuOpen(false)
            setCurrentPage(1)
            setMssqlRefreshToken((value) => value + 1)
        } catch (error) {
            console.error("[Staff Kit] MSSQL employee update failed")
            const message = getMssqlEmployeeUpdateErrorMessage(error)
            setMssqlUpdateStatus("error")
            setMssqlUpdateMessage(message)
            setGlobalError(message)
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
        fetchAllFilteredEmployees,
        isLoadingEmployees,
        isLoadingTeams,
        isUpdatingEmployeeListFromMssql,
        mssqlUpdateMessage,
        mssqlUpdateStatus,
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

export function getMssqlEmployeeUpdateErrorMessage(error: unknown): string {
    const raw = error instanceof Error ? error.message : String(error)
    const normalized = raw.toLowerCase()

    if (normalized.includes("missing mssql configuration")) {
        return "MSSQL configuration is incomplete. Open Settings > Import from MSSQL and provide the required connection settings."
    }

    if (
        normalized.includes("connection") ||
        normalized.includes("network") ||
        normalized.includes("timed out") ||
        normalized.includes("os error 10060") ||
        normalized.includes("os error 10061") ||
        normalized.includes("os error 10065")
    ) {
        return "Cannot connect to the MSSQL server. Check network or VPN access, server availability, port, and firewall settings, then try again."
    }

    if (
        normalized.includes("authentication") ||
        normalized.includes("login failed") ||
        normalized.includes("username") ||
        normalized.includes("password")
    ) {
        return "MSSQL authentication failed. Check the username and password, then try again."
    }

    if (
        normalized.includes("query") ||
        normalized.includes("read mssql results") ||
        normalized.includes("import failed during preview")
    ) {
        return "The MSSQL employee query failed. Check database permissions and query configuration, then try again."
    }

    return "The MSSQL employee update failed. Check the connection settings and try again."
}
