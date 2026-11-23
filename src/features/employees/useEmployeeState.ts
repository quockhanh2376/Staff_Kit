import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { EmployeeRecord, EmployeeSyncBatch, EmployeeSyncRowUpdate, TeamRecord } from "../../types/staff"
import type { StaffGroupKey, EmployeeSortState } from "../../types/app"
import { getUserErrorMessage } from "../../lib/errorHandling"
import { ALL_TEAMS_OPTION } from "../../lib/constants"

type UseEmployeeStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    isAdmin: boolean
    reloadToken: number
    setGlobalError: (msg: string | null) => void
}

export type EmployeeState = ReturnType<typeof useEmployeeState>

export function useEmployeeState({
    dbReady,
    isAuthenticated,
    isAdmin,
    reloadToken,
    setGlobalError,
}: UseEmployeeStateOptions) {
    const [employees, setEmployees] = useState<EmployeeRecord[]>([])
    const [teams, setTeams] = useState<TeamRecord[]>([])
    const [totalEmployees, setTotalEmployees] = useState(0)
    const [searchTerm, setSearchTerm] = useState("")
    const [debouncedSearchTerm, setDebouncedSearchTerm] = useState("")
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
        floating: 0,
        total: 0,
    })
    const [isFloatingFilterActive, setIsFloatingFilterActive] = useState(false)
    const [rowsPerPage, setRowsPerPage] = useState(500)
    const [currentPage, setCurrentPage] = useState(1)
    const [isLoadingEmployees, setLoadingEmployees] = useState(false)
    const [isLoadingTeams, setLoadingTeams] = useState(false)
    const [syncBatch, setSyncBatch] = useState<EmployeeSyncBatch | null>(null)
    const [isSyncBusy, setSyncBusy] = useState(false)
    const [syncError, setSyncError] = useState<string | null>(null)
    const [mssqlRefreshToken, setMssqlRefreshToken] = useState(0)

    const teamFilterMenuRef = useRef<HTMLDivElement | null>(null)
    const searchRequestIdRef = useRef(0)

    // Debounce search term by 250ms (instant on empty)
    useEffect(() => {
        if (!searchTerm) {
            setDebouncedSearchTerm("")
            return
        }
        const timer = setTimeout(() => {
            setDebouncedSearchTerm(searchTerm)
        }, 250)
        return () => clearTimeout(timer)
    }, [searchTerm])

    // Load a batch left open by an earlier session (admins only; once per login).
    useEffect(() => {
        if (!dbReady || !isAuthenticated || !isAdmin) return
        let disposed = false
        staffApi.getOpenEmployeeSyncBatch().then(
            (batch) => { if (!disposed) setSyncBatch(batch) },
            (error) => { if (!disposed) setSyncError(getMssqlEmployeeUpdateErrorMessage(error)) },
        )
        return () => { disposed = true }
    }, [dbReady, isAuthenticated, isAdmin])

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
    }, [debouncedSearchTerm, staffGroupFilter, isFloatingFilterActive, teamFilter, startDateFilter, rowsPerPage, employeeSort])

    // Load employees
    useEffect(() => {
        if (!dbReady || !isAuthenticated) return

        let disposed = false
        const requestId = ++searchRequestIdRef.current

        void (async () => {
            try {
                setLoadingEmployees(true)

                const response = await staffApi.searchEmployees({
                    query: debouncedSearchTerm.trim() ? debouncedSearchTerm.trim() : null,
                    staffGroup: staffGroupFilter,
                    isFloating: isFloatingFilterActive ? true : null,
                    teamName: teamFilter === ALL_TEAMS_OPTION ? null : teamFilter,
                    startDateFrom: startDateFilter || null,
                    sortKey: employeeSort?.key ?? null,
                    sortDirection: employeeSort?.direction ?? null,
                    limit: rowsPerPage,
                    offset: (currentPage - 1) * rowsPerPage,
                })

                if (disposed || requestId !== searchRequestIdRef.current) return
                setEmployees(response.items)
                setTotalEmployees(response.total)
            } catch (error) {
                if (!disposed && requestId === searchRequestIdRef.current) {
                    setGlobalError(getUserErrorMessage(error))
                }
            } finally {
                if (!disposed && requestId === searchRequestIdRef.current) {
                    setLoadingEmployees(false)
                }
            }
        })()

        return () => { disposed = true }
    }, [
        dbReady,
        isAuthenticated,
        debouncedSearchTerm,
        staffGroupFilter,
        isFloatingFilterActive,
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
        setSyncBatch(null)
        setSyncError(null)
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
                isFloating: isFloatingFilterActive ? true : null,
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
    }, [employeeSort, isFloatingFilterActive, searchTerm, staffGroupFilter, startDateFilter, teamFilter, totalEmployees])

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

    const runSync = async <T,>(task: () => Promise<T>): Promise<{ ok: true; value: T } | { ok: false }> => {
        setSyncBusy(true)
        setSyncError(null)
        setGlobalError(null)
        try {
            return { ok: true, value: await task() }
        } catch (error) {
            const message = getMssqlEmployeeUpdateErrorMessage(error)
            setSyncError(message)
            setGlobalError(message)
            return { ok: false }
        } finally {
            setSyncBusy(false)
        }
    }

    const stage = async (replace: boolean) => {
        const res = await runSync(() => staffApi.stageEmployeeListSync(replace))
        if (res.ok) setSyncBatch(res.value)
        return res.ok
    }

    const startEmployeeSync = async (): Promise<"open-exists" | "staged" | "error"> => {
        if (syncBatch) return "open-exists"
        return (await stage(false)) ? "staged" : "error"
    }

    const replaceEmployeeSync = () => stage(true)

    const resumeEmployeeSync = async () => {
        const res = await runSync(() => staffApi.getOpenEmployeeSyncBatch())
        if (!res.ok) return syncBatch // keep what we have on a failed call
        setSyncBatch(res.value)
        return res.value
    }

    const replaceSyncRow = (res: Awaited<ReturnType<typeof runSync<EmployeeSyncBatch["rows"][number]>>>) => {
        if (!res.ok) return
        const row = res.value
        setSyncBatch((prev) => prev && { ...prev, rows: prev.rows.map((r) => (r.id === row.id ? row : r)) })
    }

    const updateSyncRow = async (rowId: number, update: EmployeeSyncRowUpdate) =>
        replaceSyncRow(await runSync(() => staffApi.updateEmployeeSyncRow(rowId, update)))

    const refreshSyncRow = async (rowId: number) =>
        replaceSyncRow(await runSync(() => staffApi.refreshEmployeeSyncRow(rowId)))

    const discardEmployeeSync = async () => {
        if (!syncBatch) return
        const batchId = syncBatch.id
        const res = await runSync(() => staffApi.discardEmployeeSyncBatch(batchId))
        if (res.ok) setSyncBatch(null)
    }

    const applyEmployeeSync = async () => {
        if (!syncBatch) return null
        const batchId = syncBatch.id
        const res = await runSync(() => staffApi.applyEmployeeSyncBatch(batchId))
        if (!res.ok) {
            // Reload outside runSync so the apply error stays shown while rows come back marked stale.
            try {
                setSyncBatch(await staffApi.getOpenEmployeeSyncBatch())
            } catch {
                // keep the batch we have
            }
            return null
        }
        setSyncBatch(null)
        setStaffGroupFilter("employee_list")
        setTeamFilter(ALL_TEAMS_OPTION)
        setTeamFilterSearchTerm("")
        setStartDateFilter("")
        setTeamFilterMenuOpen(false)
        setCurrentPage(1)
        setMssqlRefreshToken((value) => value + 1)
        return res.value
    }

    return {
        employees,
        teams,
        totalEmployees,
        searchTerm,
        setSearchTerm,
        staffGroupFilter,
        setStaffGroupFilter,
        isFloatingFilterActive,
        setIsFloatingFilterActive,
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
        syncBatch,
        isSyncBusy,
        syncError,
        teamOptions,
        teamFilterOptions,
        filteredTeamFilterOptions,
        teamFilterMenuRef,
        clearFilters,
        selectTeamFilter,
        commitTypedTeamSelection,
        toggleColumnSort,
        startEmployeeSync,
        replaceEmployeeSync,
        resumeEmployeeSync,
        updateSyncRow,
        refreshSyncRow,
        discardEmployeeSync,
        applyEmployeeSync,
        resetEmployeeStateOnLogout,
    }
}

export function getMssqlEmployeeUpdateErrorMessage(error: unknown): string {
    const raw = error instanceof Error ? error.message : String(error)
    const normalized = raw.toLowerCase()

    if (
        // Local staging errors (stale rows, nothing to write, bad action or name) carry no MSSQL secrets.
        (normalized.startsWith("read-only permission diagnostic:") ||
            /^(cannot apply:|invalid action|invalid full name)/.test(normalized)) &&
        raw.length <= 350 &&
        !/[\r\n]/.test(raw) &&
        !/(?:password|token|secret)\s*=/.test(normalized)
    ) {
        return raw
    }

    const verifiedWriteAccess = "Read-only verification found effective write-level permissions for this SQL login; syncing is blocked."
    if (raw === verifiedWriteAccess) return verifiedWriteAccess

    if (
        normalized.includes("missing mssql configuration") ||
        normalized.includes("remote ee sync settings") ||
        normalized.includes("config sync settings") ||
        normalized.includes("configuration is incomplete")
    ) {
        return "Config Sync settings are incomplete. Open Settings > Config Sync and verify the connection profile."
    }

    if (normalized.includes("read-only") || normalized.includes("write-level permissions")) {
        return "The MSSQL source login must be read-only. Ask your database administrator to remove write permissions, then try again."
    }

    if (normalized.includes("preview") && (normalized.includes("expired") || normalized.includes("changed"))) {
        return "The source or local EE List changed after this preview. Run Update again to review the latest data."
    }

    if (normalized.includes("conflict")) {
        return "Some Staff Codes belong to another employee group. Resolve those records before applying the preview."
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
        normalized.includes("source results") ||
        normalized.includes("source query") ||
        normalized.includes("read mssql results") ||
        normalized.includes("import failed during preview")
    ) {
        return "The MSSQL employee query failed. Check database permissions and query configuration, then try again."
    }

    return "The MSSQL employee update failed. Check the connection settings and try again."
}
