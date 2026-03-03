import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from "react"
import {
  Calendar,
  Check,
  ChevronDown,
  Columns3,
  GripVertical,
  LogIn,
  LogOut,
  LoaderCircle,
  Moon,
  Pencil,
  Plus,
  Search,
  Settings,
  Sun,
  Trash2,
  Upload,
  Users,
  X,
} from "lucide-react"
import { open as openFileDialog } from "@tauri-apps/plugin-dialog"
import { staffApi } from "./services/staff-api"
import type {
  DatabaseStatus,
  EmployeeColumnDefinition,
  EmployeeGroupCounts,
  EmployeePayload,
  EmployeeRecord,
  ImportColumnOption,
  ImportReport,
  LocalAccountRecord,
  TeamRecord,
} from "./types/staff"

type AppView = "employees" | "teams" | "settings"
type Theme = "dark" | "light"
type UiColumnSource = "core" | "dynamic" | "system"
type StaffGroupKey = "employee_list" | "onboarding" | "offboarding" | "internal_movement"

type UiColumnDefinition = {
  key: string
  label: string
  source: UiColumnSource
}

type ColumnPreferences = {
  order: string[]
  hidden: string[]
}

type LocalAccountRole = "admin" | "user"

type ColumnWidthMap = Record<string, number>
type ActiveResizeState = {
  key: string
  startX: number
  startWidth: number
}

const THEME_KEY = "staffkit-theme"
const COLUMN_PREFS_KEY = "staffkit-column-prefs"
const COLUMN_PREFS_VERSION_KEY = "staffkit-column-prefs-version"
const COLUMN_PREFS_VERSION = "3"
const COLUMN_LABEL_OVERRIDES_KEY = "staffkit-column-label-overrides"
const COLUMN_WIDTHS_KEY = "staffkit-column-widths"
const DEFAULT_ACCOUNT_NAME = "IT Admin"
const ALL_TEAMS_OPTION = "All Teams"
const STAFF_GROUP_BUTTONS: Array<{ key: StaffGroupKey; label: string }> = [
  { key: "employee_list", label: "Employee list" },
  { key: "onboarding", label: "Onboarding" },
  { key: "offboarding", label: "Offboarding" },
  { key: "internal_movement", label: "Internal Movement" },
]

const DEFAULT_SYSTEM_COLUMNS: UiColumnDefinition[] = [
  { key: "rowNumber", label: "#", source: "system" },
  { key: "actions", label: "Actions", source: "system" },
]

const DEFAULT_VISIBLE_COLUMN_KEYS = [
  "employeeId",
  "fullName",
  "teamName",
  "email",
  "computerName",
]

const DATE_COLUMN_KEYS = new Set([
  "dateOfBirth",
  "aswStartDate",
  "clientStartDate",
])

const MIN_COLUMN_WIDTH = 78
const MAX_COLUMN_WIDTH = 720
const DEFAULT_COLUMN_WIDTHS: ColumnWidthMap = {
  rowNumber: 64,
  employeeId: 116,
  fullName: 220,
  nickName: 130,
  teamName: 190,
  project: 170,
  jobTitle: 210,
  email: 230,
  cellphone: 145,
  dateOfBirth: 130,
  gender: 100,
  aswStartDate: 130,
  clientStartDate: 140,
  contractEndDate: 140,
  clientYearOfServices: 176,
  computerName: 170,
  notes: 220,
  actions: 96,
}

const getErrorMessage = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message
  }

  return String(error)
}

const formatDate = (value: string | null) => {
  if (!value) {
    return "-"
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return date.toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "2-digit",
  })
}

const getGroupCount = (counts: EmployeeGroupCounts, key: StaffGroupKey): number => {
  switch (key) {
    case "employee_list":
      return counts.employeeList
    case "onboarding":
      return counts.onboarding
    case "offboarding":
      return counts.offboarding
    case "internal_movement":
      return counts.internalMovement
    default:
      return 0
  }
}

const normalizeUserScope = (name: string): string => {
  const normalized = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")

  return normalized || "default_user"
}

const buildScopedStorageKey = (baseKey: string, scope: string): string => `${baseKey}:${scope}`

const readRawStorageValue = (storageKey: string, fallbackKeys: string[] = []): string | null => {
  const current = localStorage.getItem(storageKey)
  if (current) {
    return current
  }

  for (const key of fallbackKeys) {
    const fallback = localStorage.getItem(key)
    if (fallback) {
      return fallback
    }
  }

  return null
}

const readColumnPreferences = (storageKey: string, fallbackKeys: string[] = []): ColumnPreferences => {
  try {
    const raw = readRawStorageValue(storageKey, fallbackKeys)
    if (!raw) {
      return { order: [], hidden: [] }
    }

    const parsed = JSON.parse(raw) as Partial<ColumnPreferences>
    return {
      order: Array.isArray(parsed.order) ? parsed.order.filter((item): item is string => typeof item === "string") : [],
      hidden: Array.isArray(parsed.hidden) ? parsed.hidden.filter((item): item is string => typeof item === "string") : [],
    }
  } catch {
    return { order: [], hidden: [] }
  }
}

const readColumnLabelOverrides = (storageKey: string, fallbackKeys: string[] = []): Record<string, string> => {
  try {
    const raw = readRawStorageValue(storageKey, fallbackKeys)
    if (!raw) {
      return {}
    }

    const parsed = JSON.parse(raw) as Record<string, unknown>
    const entries = Object.entries(parsed).filter(
      ([key, value]) => key.trim().length > 0 && typeof value === "string",
    )
    return Object.fromEntries(entries) as Record<string, string>
  } catch {
    return {}
  }
}

const readColumnWidths = (storageKey: string, fallbackKeys: string[] = []): ColumnWidthMap => {
  try {
    const raw = readRawStorageValue(storageKey, fallbackKeys)
    if (!raw) {
      return {}
    }

    const parsed = JSON.parse(raw) as Record<string, unknown>
    const next: ColumnWidthMap = {}
    for (const [key, value] of Object.entries(parsed)) {
      if (typeof value !== "number" || Number.isNaN(value)) {
        continue
      }
      next[key] = Math.max(MIN_COLUMN_WIDTH, Math.min(MAX_COLUMN_WIDTH, Math.round(value)))
    }
    return next
  } catch {
    return {}
  }
}

const serializeColumnPreferences = (value: ColumnPreferences): string => {
  const order = [...value.order].join("|")
  const hidden = [...value.hidden].join("|")
  return `${order}__${hidden}`
}

const serializeStringMap = (value: Record<string, string>): string => (
  Object.keys(value)
    .sort((a, b) => a.localeCompare(b))
    .map((key) => `${key}:${value[key]}`)
    .join("|")
)

const serializeWidthMap = (value: ColumnWidthMap): string => (
  Object.keys(value)
    .sort((a, b) => a.localeCompare(b))
    .map((key) => `${key}:${value[key]}`)
    .join("|")
)

const reconcileColumnPreferences = (
  preferences: ColumnPreferences,
  configurableKeys: string[],
): ColumnPreferences => {
  const keySet = new Set(configurableKeys)
  const existingOrder = preferences.order.filter((key) => keySet.has(key))
  const missing = configurableKeys.filter((key) => !existingOrder.includes(key))

  return {
    order: [...existingOrder, ...missing],
    hidden: preferences.hidden.filter((key) => keySet.has(key)),
  }
}

const buildDefaultColumnPreferences = (configurableKeys: string[]): ColumnPreferences => {
  const visibleSet = new Set(DEFAULT_VISIBLE_COLUMN_KEYS)
  const prioritized = DEFAULT_VISIBLE_COLUMN_KEYS.filter((key) => configurableKeys.includes(key))
  const remaining = configurableKeys.filter((key) => !prioritized.includes(key))
  const order = [...prioritized, ...remaining]

  return {
    order,
    hidden: order.filter((key) => !visibleSet.has(key)),
  }
}

function Drawer({
  open,
  onClose,
  title,
  widthClass,
  children,
}: {
  open: boolean
  onClose: () => void
  title: string
  widthClass: string
  children: ReactNode
}) {
  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/50 backdrop-blur-[1px]">
      <div className={`h-full ${widthClass} max-w-[100vw] border-l border-[var(--border)] bg-[var(--surface)] shadow-2xl`}>
        <div className="flex h-16 items-center justify-between border-b border-[var(--border)] px-5">
          <h3 className="text-lg font-semibold text-[var(--text-primary)]">{title}</h3>
          <button
            className="rounded-md p-2 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
            onClick={onClose}
            type="button"
          >
            <X size={18} />
          </button>
        </div>
        <div className="h-[calc(100%-4rem)] overflow-auto p-5">{children}</div>
      </div>
    </div>
  )
}

function App() {
  const [theme, setTheme] = useState<Theme>("dark")
  const [activeView, setActiveView] = useState<AppView>("employees")
  const [dbStatus, setDbStatus] = useState<DatabaseStatus | null>(null)
  const [dbReady, setDbReady] = useState(false)

  const [employees, setEmployees] = useState<EmployeeRecord[]>([])
  const [teams, setTeams] = useState<TeamRecord[]>([])
  const [columnDefinitions, setColumnDefinitions] = useState<EmployeeColumnDefinition[]>([])

  const [totalEmployees, setTotalEmployees] = useState(0)
  const [searchTerm, setSearchTerm] = useState("")
  const [staffGroupFilter, setStaffGroupFilter] = useState<StaffGroupKey>("employee_list")
  const [teamFilter, setTeamFilter] = useState(ALL_TEAMS_OPTION)
  const [startDateFilter, setStartDateFilter] = useState("")
  const [isTeamFilterMenuOpen, setTeamFilterMenuOpen] = useState(false)
  const [employeeGroupCounts, setEmployeeGroupCounts] = useState<EmployeeGroupCounts>({
    employeeList: 0,
    onboarding: 0,
    offboarding: 0,
    internalMovement: 0,
  })

  const [rowsPerPage, setRowsPerPage] = useState(15)
  const [currentPage, setCurrentPage] = useState(1)
  const [reloadToken, setReloadToken] = useState(0)

  const [isLoadingEmployees, setLoadingEmployees] = useState(false)
  const [isLoadingTeams, setLoadingTeams] = useState(false)
  const [isLoadingColumns, setLoadingColumns] = useState(false)
  const [isLoadingAccounts, setLoadingAccounts] = useState(false)
  const [isBootstrapping, setBootstrapping] = useState(true)
  const [globalError, setGlobalError] = useState<string | null>(null)

  const [isImportDrawerOpen, setImportDrawerOpen] = useState(false)
  const [isImporting, setImporting] = useState(false)
  const [importReport, setImportReport] = useState<ImportReport | null>(null)
  const [importSelectedFiles, setImportSelectedFiles] = useState<string[]>([])
  const [importColumnOptions, setImportColumnOptions] = useState<ImportColumnOption[]>([])
  const [importSelectedColumnKeys, setImportSelectedColumnKeys] = useState<string[]>([])

  const [isEmployeeDrawerOpen, setEmployeeDrawerOpen] = useState(false)
  const [editingEmployee, setEditingEmployee] = useState<EmployeeRecord | null>(null)

  const [newTeamName, setNewTeamName] = useState("")
  const [isSavingTeam, setSavingTeam] = useState(false)
  const [isResettingData, setResettingData] = useState(false)
  const [accounts, setAccounts] = useState<LocalAccountRecord[]>([])
  const [activeAccountId, setActiveAccountId] = useState<number | null>(null)
  const [newAccountName, setNewAccountName] = useState("")
  const [newAccountRole, setNewAccountRole] = useState<LocalAccountRole>("user")
  const [isMutatingAccounts, setMutatingAccounts] = useState(false)
  const [isAuthenticated, setAuthenticated] = useState(false)
  const [loginAccountId, setLoginAccountId] = useState<number | null>(null)
  const [isSigningIn, setSigningIn] = useState(false)

  const [isColumnsDrawerOpen, setColumnsDrawerOpen] = useState(false)
  const [draggingColumnKey, setDraggingColumnKey] = useState<string | null>(null)
  const [activeResize, setActiveResize] = useState<ActiveResizeState | null>(null)
  const activeAccount = useMemo(
    () => accounts.find((item) => item.id === activeAccountId) ?? accounts.find((item) => item.isActive) ?? accounts[0] ?? null,
    [accounts, activeAccountId],
  )
  const activeUserScope = useMemo(
    () => activeAccount?.accountKey ?? normalizeUserScope(DEFAULT_ACCOUNT_NAME),
    [activeAccount],
  )
  const scopedColumnPrefsKey = useMemo(
    () => buildScopedStorageKey(COLUMN_PREFS_KEY, activeUserScope),
    [activeUserScope],
  )
  const scopedColumnPrefsVersionKey = useMemo(
    () => buildScopedStorageKey(COLUMN_PREFS_VERSION_KEY, activeUserScope),
    [activeUserScope],
  )
  const scopedColumnLabelOverridesKey = useMemo(
    () => buildScopedStorageKey(COLUMN_LABEL_OVERRIDES_KEY, activeUserScope),
    [activeUserScope],
  )
  const scopedColumnWidthsKey = useMemo(
    () => buildScopedStorageKey(COLUMN_WIDTHS_KEY, activeUserScope),
    [activeUserScope],
  )

  const [columnPreferences, setColumnPreferences] = useState<ColumnPreferences>(() =>
    readColumnPreferences(scopedColumnPrefsKey),
  )
  const [columnLabelOverrides, setColumnLabelOverrides] = useState<Record<string, string>>(
    () => readColumnLabelOverrides(scopedColumnLabelOverridesKey),
  )
  const [columnWidths, setColumnWidths] = useState<ColumnWidthMap>(() =>
    readColumnWidths(scopedColumnWidthsKey),
  )
  const [savedColumnPreferences, setSavedColumnPreferences] = useState<ColumnPreferences>(() =>
    readColumnPreferences(scopedColumnPrefsKey),
  )
  const [savedColumnLabelOverrides, setSavedColumnLabelOverrides] = useState<Record<string, string>>(
    () => readColumnLabelOverrides(scopedColumnLabelOverridesKey),
  )
  const [savedColumnWidths, setSavedColumnWidths] = useState<ColumnWidthMap>(() =>
    readColumnWidths(scopedColumnWidthsKey),
  )
  const [columnSearchTerm, setColumnSearchTerm] = useState("")
  const [newColumnTitle, setNewColumnTitle] = useState("")
  const [isMutatingColumns, setMutatingColumns] = useState(false)

  const teamFilterMenuRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const savedTheme = (localStorage.getItem(THEME_KEY) as Theme | null) ?? "dark"
    setTheme(savedTheme)
  }, [])

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark")
    localStorage.setItem(THEME_KEY, theme)
  }, [theme])

  useEffect(() => {
    if (!dbReady) return

    let disposed = false

    void (async () => {
      try {
        setLoadingAccounts(true)
        const data = await staffApi.listLocalAccounts()
        if (disposed) return

        setAccounts(data)
        const active = data.find((item) => item.isActive) ?? data[0] ?? null
        setActiveAccountId((current) => {
          if (current !== null && data.some((item) => item.id === current)) {
            return current
          }
          return active?.id ?? null
        })
        setLoginAccountId((current) => {
          if (current !== null && data.some((item) => item.id === current)) {
            return current
          }
          return active?.id ?? null
        })

        if (data.length === 0) {
          setAuthenticated(false)
        }
      } catch (error) {
        if (!disposed) {
          setGlobalError(getErrorMessage(error))
        }
      } finally {
        if (!disposed) {
          setLoadingAccounts(false)
        }
      }
    })()

    return () => {
      disposed = true
    }
  }, [dbReady, reloadToken])

  useEffect(() => {
    const nextPrefs = readColumnPreferences(scopedColumnPrefsKey)
    const nextLabels = readColumnLabelOverrides(scopedColumnLabelOverridesKey)
    const nextWidths = readColumnWidths(scopedColumnWidthsKey)

    setColumnPreferences(nextPrefs)
    setColumnLabelOverrides(nextLabels)
    setColumnWidths(nextWidths)
    setSavedColumnPreferences(nextPrefs)
    setSavedColumnLabelOverrides(nextLabels)
    setSavedColumnWidths(nextWidths)
    setColumnSearchTerm("")
  }, [scopedColumnLabelOverridesKey, scopedColumnPrefsKey, scopedColumnWidthsKey])

  useEffect(() => {
    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node | null
      if (teamFilterMenuRef.current && target && !teamFilterMenuRef.current.contains(target)) {
        setTeamFilterMenuOpen(false)
      }
    }

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setTeamFilterMenuOpen(false)
      }
    }

    document.addEventListener("mousedown", handlePointerDown)
    document.addEventListener("keydown", handleEscape)
    return () => {
      document.removeEventListener("mousedown", handlePointerDown)
      document.removeEventListener("keydown", handleEscape)
    }
  }, [])

  useEffect(() => {
    if (!activeResize) {
      return
    }

    document.body.classList.add("column-resize-active")

    const handlePointerMove = (event: MouseEvent) => {
      const deltaX = event.clientX - activeResize.startX
      const nextWidth = Math.max(MIN_COLUMN_WIDTH, Math.min(MAX_COLUMN_WIDTH, activeResize.startWidth + deltaX))
      setColumnWidths((prev) => ({
        ...prev,
        [activeResize.key]: Math.round(nextWidth),
      }))
    }

    const handlePointerUp = () => {
      setActiveResize(null)
    }

    window.addEventListener("mousemove", handlePointerMove)
    window.addEventListener("mouseup", handlePointerUp)
    return () => {
      document.body.classList.remove("column-resize-active")
      window.removeEventListener("mousemove", handlePointerMove)
      window.removeEventListener("mouseup", handlePointerUp)
    }
  }, [activeResize])

  useEffect(() => {
    let disposed = false

    void (async () => {
      try {
        setBootstrapping(true)
        setGlobalError(null)

        const status = await staffApi.initDatabase()
        if (disposed) return

        setDbStatus(status)
        setDbReady(true)
      } catch (error) {
        if (!disposed) {
          setGlobalError(getErrorMessage(error))
        }
      } finally {
        if (!disposed) {
          setBootstrapping(false)
        }
      }
    })()

    return () => {
      disposed = true
    }
  }, [])

  useEffect(() => {
    if (!dbReady || !isAuthenticated) return

    let disposed = false

    void (async () => {
      try {
        setLoadingTeams(true)
        const data = await staffApi.listTeams()
        if (!disposed) {
          setTeams(data)
        }
      } catch (error) {
        if (!disposed) {
          setGlobalError(getErrorMessage(error))
        }
      } finally {
        if (!disposed) {
          setLoadingTeams(false)
        }
      }
    })()

    return () => {
      disposed = true
    }
  }, [dbReady, isAuthenticated, reloadToken])

  useEffect(() => {
    if (!dbReady || !isAuthenticated) return

    let disposed = false

    void (async () => {
      try {
        const counts = await staffApi.listEmployeeGroupCounts()
        if (!disposed) {
          setEmployeeGroupCounts(counts)
        }
      } catch (error) {
        if (!disposed) {
          setGlobalError(getErrorMessage(error))
        }
      }
    })()

    return () => {
      disposed = true
    }
  }, [dbReady, isAuthenticated, reloadToken])

  useEffect(() => {
    if (!dbReady || !isAuthenticated) return

    let disposed = false

    void (async () => {
      try {
        setLoadingColumns(true)
        const columns = await staffApi.listEmployeeColumns()
        if (!disposed) {
          setColumnDefinitions(columns)
        }
      } catch (error) {
        if (!disposed) {
          setGlobalError(getErrorMessage(error))
        }
      } finally {
        if (!disposed) {
          setLoadingColumns(false)
        }
      }
    })()

    return () => {
      disposed = true
    }
  }, [dbReady, isAuthenticated, reloadToken])

  useEffect(() => {
    setCurrentPage(1)
  }, [searchTerm, staffGroupFilter, teamFilter, startDateFilter, rowsPerPage])

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
          limit: rowsPerPage,
          offset: (currentPage - 1) * rowsPerPage,
        })

        if (disposed) return

        setEmployees(response.items)
        setTotalEmployees(response.total)
      } catch (error) {
        if (!disposed) {
          setGlobalError(getErrorMessage(error))
        }
      } finally {
        if (!disposed) {
          setLoadingEmployees(false)
        }
      }
    })()

    return () => {
      disposed = true
    }
  }, [dbReady, isAuthenticated, searchTerm, staffGroupFilter, teamFilter, startDateFilter, rowsPerPage, currentPage, reloadToken])

  useEffect(() => {
    if (accounts.length > 0) {
      return
    }

    setActiveAccountId(null)
    setLoginAccountId(null)
    setAuthenticated(false)
  }, [accounts])

  useEffect(() => {
    if (isAuthenticated) {
      return
    }

    setEmployees([])
    setTeams([])
    setColumnDefinitions([])
    setTotalEmployees(0)
    setCurrentPage(1)
    setTeamFilterMenuOpen(false)
  }, [isAuthenticated])

  const teamOptions = useMemo(
    () => teams.map((team) => team.name).sort((a, b) => a.localeCompare(b)),
    [teams],
  )

  const teamFilterOptions = useMemo(
    () => [ALL_TEAMS_OPTION, ...teamOptions],
    [teamOptions],
  )

  const selectedGroupLabel = useMemo(
    () => STAFF_GROUP_BUTTONS.find((item) => item.key === staffGroupFilter)?.label ?? "Employee list",
    [staffGroupFilter],
  )

  const selectedGroupTotal = useMemo(
    () => getGroupCount(employeeGroupCounts, staffGroupFilter),
    [employeeGroupCounts, staffGroupFilter],
  )

  const activeAccountName = activeAccount?.displayName ?? DEFAULT_ACCOUNT_NAME

  const totalPages = useMemo(() => {
    if (totalEmployees <= 0) {
      return 1
    }

    return Math.max(1, Math.ceil(totalEmployees / rowsPerPage))
  }, [rowsPerPage, totalEmployees])

  useEffect(() => {
    if (currentPage > totalPages) {
      setCurrentPage(totalPages)
    }
  }, [currentPage, totalPages])

  const uiColumns = useMemo<UiColumnDefinition[]>(() => {
    const dbColumns = columnDefinitions.map((column) => ({
      key: column.key,
      label: columnLabelOverrides[column.key]?.trim() || column.label,
      source: column.source,
    }))

    return [DEFAULT_SYSTEM_COLUMNS[0], ...dbColumns, DEFAULT_SYSTEM_COLUMNS[1]]
  }, [columnDefinitions, columnLabelOverrides])

  const configurableColumns = useMemo(
    () => uiColumns.filter((column) => column.key !== "actions"),
    [uiColumns],
  )

  const configurableColumnMap = useMemo(
    () => new Map(configurableColumns.map((column) => [column.key, column])),
    [configurableColumns],
  )

  const configurableKeys = useMemo(
    () => configurableColumns.map((column) => column.key),
    [configurableColumns],
  )

  const effectiveColumnPreferences = useMemo(
    () => reconcileColumnPreferences(columnPreferences, configurableKeys),
    [columnPreferences, configurableKeys],
  )

  useEffect(() => {
    if (configurableKeys.length === 0) {
      return
    }

    const currentVersion = localStorage.getItem(scopedColumnPrefsVersionKey)
    if (currentVersion !== COLUMN_PREFS_VERSION) {
      const defaults = buildDefaultColumnPreferences(configurableKeys)
      localStorage.setItem(scopedColumnPrefsKey, JSON.stringify(defaults))
      localStorage.setItem(scopedColumnLabelOverridesKey, JSON.stringify({}))
      localStorage.setItem(scopedColumnWidthsKey, JSON.stringify({}))
      setColumnPreferences(defaults)
      setColumnLabelOverrides({})
      setColumnWidths({})
      setSavedColumnPreferences(defaults)
      setSavedColumnLabelOverrides({})
      setSavedColumnWidths({})
      localStorage.setItem(scopedColumnPrefsVersionKey, COLUMN_PREFS_VERSION)
      return
    }

    const reconciled = reconcileColumnPreferences(columnPreferences, configurableKeys)
    const shouldUpdateOrder = reconciled.order.join("|") !== columnPreferences.order.join("|")
    const shouldUpdateHidden = reconciled.hidden.join("|") !== columnPreferences.hidden.join("|")

    if (shouldUpdateOrder || shouldUpdateHidden) {
      setColumnPreferences(reconciled)
    }
  }, [
    columnPreferences,
    configurableKeys,
    scopedColumnLabelOverridesKey,
    scopedColumnPrefsKey,
    scopedColumnPrefsVersionKey,
    scopedColumnWidthsKey,
  ])

  const visibleColumnKeys = useMemo(() => {
    const visible = effectiveColumnPreferences.order.filter(
      (key) => !effectiveColumnPreferences.hidden.includes(key),
    )

    if (!visible.includes("actions")) {
      visible.push("actions")
    }

    return visible
  }, [effectiveColumnPreferences.hidden, effectiveColumnPreferences.order])

  const visibleColumns = useMemo(
    () => visibleColumnKeys.map((key) => uiColumns.find((column) => column.key === key)).filter(Boolean) as UiColumnDefinition[],
    [uiColumns, visibleColumnKeys],
  )

  const resolvedColumnWidths = useMemo<ColumnWidthMap>(() => {
    const next: ColumnWidthMap = {}
    for (const column of uiColumns) {
      const savedWidth = columnWidths[column.key]
      const fallback = DEFAULT_COLUMN_WIDTHS[column.key] ?? 170
      const width = typeof savedWidth === "number" && !Number.isNaN(savedWidth) ? savedWidth : fallback
      next[column.key] = Math.max(MIN_COLUMN_WIDTH, Math.min(MAX_COLUMN_WIDTH, Math.round(width)))
    }
    return next
  }, [columnWidths, uiColumns])

  const requiredImportColumnKeys = useMemo(
    () => importColumnOptions.filter((column) => column.required).map((column) => column.key),
    [importColumnOptions],
  )

  const effectiveImportColumnKeySet = useMemo(() => {
    const next = new Set(importSelectedColumnKeys)
    for (const key of requiredImportColumnKeys) {
      next.add(key)
    }
    return next
  }, [importSelectedColumnKeys, requiredImportColumnKeys])

  const filteredColumnKeys = useMemo(() => {
    const keyword = columnSearchTerm.trim().toLowerCase()
    if (!keyword) {
      return effectiveColumnPreferences.order
    }

    return effectiveColumnPreferences.order.filter((key) => {
      const column = configurableColumnMap.get(key)
      if (!column) {
        return false
      }

      return `${column.label} ${column.key}`.toLowerCase().includes(keyword)
    })
  }, [columnSearchTerm, configurableColumnMap, effectiveColumnPreferences.order])

  const startColumnResize = (event: ReactMouseEvent<HTMLButtonElement>, key: string) => {
    event.preventDefault()
    event.stopPropagation()
    setActiveResize({
      key,
      startX: event.clientX,
      startWidth: resolvedColumnWidths[key] ?? DEFAULT_COLUMN_WIDTHS[key] ?? 170,
    })
  }

  const hasUnsavedColumnProfileChanges = useMemo(() => (
    serializeColumnPreferences(columnPreferences) !== serializeColumnPreferences(savedColumnPreferences)
      || serializeStringMap(columnLabelOverrides) !== serializeStringMap(savedColumnLabelOverrides)
      || serializeWidthMap(columnWidths) !== serializeWidthMap(savedColumnWidths)
  ), [
    columnLabelOverrides,
    columnPreferences,
    columnWidths,
    savedColumnLabelOverrides,
    savedColumnPreferences,
    savedColumnWidths,
  ])

  const triggerReload = () => {
    setReloadToken((value) => value + 1)
  }

  const openCreateDrawer = () => {
    setEditingEmployee(null)
    setEmployeeDrawerOpen(true)
  }

  const openEditDrawer = (employee: EmployeeRecord) => {
    setEditingEmployee(employee)
    setEmployeeDrawerOpen(true)
  }

  const clearFilters = () => {
    setSearchTerm("")
    setTeamFilter(ALL_TEAMS_OPTION)
    setStartDateFilter("")
    setTeamFilterMenuOpen(false)
  }

  const handleLoginSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    const targetId = loginAccountId
    if (targetId === null) {
      return
    }

    try {
      setSigningIn(true)
      setGlobalError(null)
      await staffApi.setActiveLocalAccount(targetId)
      setActiveAccountId(targetId)
      setAuthenticated(true)
      setActiveView("employees")
      setColumnsDrawerOpen(false)
      setImportDrawerOpen(false)
      setEmployeeDrawerOpen(false)
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setSigningIn(false)
    }
  }

  const handleLogout = () => {
    setAuthenticated(false)
    setActiveView("employees")
    setColumnsDrawerOpen(false)
    setImportDrawerOpen(false)
    setEmployeeDrawerOpen(false)
    setTeamFilterMenuOpen(false)
  }

  const handleCreateAccount = async () => {
    const name = newAccountName.trim()
    if (!name) {
      return
    }

    try {
      setMutatingAccounts(true)
      setGlobalError(null)
      await staffApi.createLocalAccount({
        displayName: name,
        role: newAccountRole,
      })
      setNewAccountName("")
      setNewAccountRole("user")
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setMutatingAccounts(false)
    }
  }

  const handleActivateAccount = async (id: number) => {
    if (id === activeAccountId) {
      return
    }

    try {
      setMutatingAccounts(true)
      setGlobalError(null)
      await staffApi.setActiveLocalAccount(id)
      setActiveAccountId(id)
      setLoginAccountId(id)
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setMutatingAccounts(false)
    }
  }

  const handleRenameAccount = async (account: LocalAccountRecord) => {
    const nextName = window.prompt("Rename account", account.displayName)
    if (!nextName || nextName.trim().length === 0) {
      return
    }

    try {
      setMutatingAccounts(true)
      setGlobalError(null)
      await staffApi.updateLocalAccount({
        id: account.id,
        displayName: nextName.trim(),
        role: account.role,
      })
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setMutatingAccounts(false)
    }
  }

  const handleDeleteAccount = async (account: LocalAccountRecord) => {
    const accepted = window.confirm(`Delete account '${account.displayName}'?`)
    if (!accepted) {
      return
    }

    try {
      setMutatingAccounts(true)
      setGlobalError(null)
      await staffApi.deleteLocalAccount(account.id)
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setMutatingAccounts(false)
    }
  }

  const toggleColumnVisibility = (key: string) => {
    setColumnPreferences((prev) => {
      const reconciled = reconcileColumnPreferences(prev, configurableKeys)
      const isHidden = reconciled.hidden.includes(key)

      return {
        ...reconciled,
        hidden: isHidden
          ? reconciled.hidden.filter((item) => item !== key)
          : [...reconciled.hidden, key],
      }
    })
  }

  const reorderColumns = (sourceKey: string, targetKey: string) => {
    if (sourceKey === targetKey) {
      return
    }

    setColumnPreferences((prev) => {
      const reconciled = reconcileColumnPreferences(prev, configurableKeys)
      const nextOrder = [...reconciled.order]
      const from = nextOrder.indexOf(sourceKey)
      const to = nextOrder.indexOf(targetKey)
      if (from === -1 || to === -1) {
        return reconciled
      }

      const [moved] = nextOrder.splice(from, 1)
      nextOrder.splice(to, 0, moved)

      return {
        ...reconciled,
        order: nextOrder,
      }
    })
  }

  const handleSaveColumnProfile = () => {
    localStorage.setItem(scopedColumnPrefsKey, JSON.stringify(columnPreferences))
    localStorage.setItem(scopedColumnLabelOverridesKey, JSON.stringify(columnLabelOverrides))
    localStorage.setItem(scopedColumnWidthsKey, JSON.stringify(columnWidths))
    localStorage.setItem(scopedColumnPrefsVersionKey, COLUMN_PREFS_VERSION)

    setSavedColumnPreferences(columnPreferences)
    setSavedColumnLabelOverrides(columnLabelOverrides)
    setSavedColumnWidths(columnWidths)
  }

  const resetColumnPreferences = () => {
    setColumnPreferences(buildDefaultColumnPreferences(configurableKeys))
    setColumnWidths({})
  }

  const handleRenameColumn = async (column: UiColumnDefinition) => {
    const currentTitle = column.label
    const nextTitle = window.prompt("Column title", currentTitle)
    if (nextTitle === null) {
      return
    }

    const normalized = nextTitle.trim()
    if (normalized.length === 0) {
      setColumnLabelOverrides((prev) => {
        const next = { ...prev }
        delete next[column.key]
        return next
      })
      return
    }

    if (column.source === "dynamic") {
      try {
        setMutatingColumns(true)
        setGlobalError(null)
        await staffApi.upsertEmployeeColumn({
          key: column.key,
          label: normalized,
        })
        setColumnLabelOverrides((prev) => {
          const next = { ...prev }
          delete next[column.key]
          return next
        })
        triggerReload()
      } catch (error) {
        setGlobalError(getErrorMessage(error))
      } finally {
        setMutatingColumns(false)
      }
      return
    }

    setColumnLabelOverrides((prev) => ({
      ...prev,
      [column.key]: normalized,
    }))
  }

  const handleAddColumn = async () => {
    const title = newColumnTitle.trim()
    if (!title) {
      return
    }

    try {
      setMutatingColumns(true)
      setGlobalError(null)
      await staffApi.upsertEmployeeColumn({
        key: null,
        label: title,
      })
      setNewColumnTitle("")
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setMutatingColumns(false)
    }
  }

  const handleDeleteColumn = async (column: UiColumnDefinition) => {
    if (column.source !== "dynamic") {
      return
    }

    const accepted = window.confirm(`Delete column '${column.label}'?`)
    if (!accepted) {
      return
    }

    try {
      setMutatingColumns(true)
      setGlobalError(null)
      await staffApi.deleteEmployeeColumn(column.key)
      setColumnLabelOverrides((prev) => {
        const next = { ...prev }
        delete next[column.key]
        return next
      })
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setMutatingColumns(false)
    }
  }

  const readCellValue = (employee: EmployeeRecord, columnKey: string, rowIndex: number): ReactNode => {
    if (columnKey === "rowNumber") {
      return (currentPage - 1) * rowsPerPage + rowIndex + 1
    }

    if (columnKey === "actions") {
      return (
        <button
          className="rounded-md p-2 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
          onClick={() => openEditDrawer(employee)}
          type="button"
        >
          <Pencil size={16} />
        </button>
      )
    }

    const dynamicValue = employee.dynamicFields[columnKey]
    if (dynamicValue !== undefined) {
      return dynamicValue || "-"
    }

    const coreValueMap: Record<string, string | null | number> = {
      employeeId: employee.employeeId,
      fullName: employee.fullName,
      nickName: employee.nickName,
      teamName: employee.teamName,
      project: employee.project,
      jobTitle: employee.jobTitle,
      email: employee.email,
      cellphone: employee.cellphone,
      dateOfBirth: employee.dateOfBirth,
      gender: employee.gender,
      aswStartDate: employee.aswStartDate,
      clientStartDate: employee.clientStartDate,
      contractEndDate: employee.contractEndDate,
      clientYearOfServices: employee.clientYearOfServices,
      computerName: employee.computerName,
      notes: employee.notes,
    }

    const value = coreValueMap[columnKey]
    if (value === null || value === undefined || value === "") {
      return "-"
    }

    if (typeof value === "string" && DATE_COLUMN_KEYS.has(columnKey)) {
      return formatDate(value)
    }

    return value
  }

  const handlePickImportFiles = async () => {
    if (isImporting) {
      return
    }

    try {
      const selected = await openFileDialog({
        title: "Select Excel File(s)",
        multiple: true,
        directory: false,
        filters: [
          {
            name: "Excel",
            extensions: ["xlsx", "xlsm", "xls"],
          },
        ],
      })

      if (!selected) {
        return
      }

      const filePaths = Array.isArray(selected) ? selected : [selected]
      if (filePaths.length === 0) {
        return
      }

      setImporting(true)
      setGlobalError(null)

      const preview = await staffApi.inspectImportColumns({
        filePaths,
      })

      setImportSelectedFiles(preview.sourceFiles)
      setImportColumnOptions(preview.detectedColumns)
      setImportSelectedColumnKeys(preview.detectedColumns.filter((item) => !item.required).map((item) => item.key))
      setImportReport(null)
      setImportDrawerOpen(true)
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setImporting(false)
    }
  }

  const toggleImportColumn = (column: ImportColumnOption) => {
    if (column.required) {
      return
    }

    setImportSelectedColumnKeys((prev) => {
      if (prev.includes(column.key)) {
        return prev.filter((key) => key !== column.key)
      }
      return [...prev, column.key]
    })
  }

  const selectAllOptionalImportColumns = () => {
    setImportSelectedColumnKeys(importColumnOptions.filter((item) => !item.required).map((item) => item.key))
  }

  const clearOptionalImportColumns = () => {
    setImportSelectedColumnKeys([])
  }

  const handleImportSelectedColumns = async () => {
    if (isImporting || importSelectedFiles.length === 0 || importColumnOptions.length === 0) {
      return
    }

    try {
      setImporting(true)
      setGlobalError(null)

      const report = await staffApi.importExcel({
        filePaths: importSelectedFiles,
        selectedColumnKeys: Array.from(effectiveImportColumnKeySet),
      })

      setImportReport(report)
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setImporting(false)
    }
  }

  const handleCreateTeam = async () => {
    if (newTeamName.trim().length === 0) {
      return
    }

    try {
      setSavingTeam(true)
      setGlobalError(null)

      await staffApi.upsertTeam({
        name: newTeamName.trim(),
      })

      setNewTeamName("")
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setSavingTeam(false)
    }
  }

  const handleRenameTeam = async (team: TeamRecord) => {
    const nextName = window.prompt("Rename team", team.name)
    if (!nextName || nextName.trim().length === 0) {
      return
    }

    try {
      setGlobalError(null)
      await staffApi.upsertTeam({ id: team.id, name: nextName.trim() })
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    }
  }

  const handleDeleteTeam = async (team: TeamRecord) => {
    const accepted = window.confirm(`Delete team '${team.name}'?`)
    if (!accepted) {
      return
    }

    try {
      setGlobalError(null)
      await staffApi.deleteTeam(team.id)
      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    }
  }

  const handleResetAllData = async () => {
    if (isResettingData) {
      return
    }

    const firstConfirm = window.confirm(
      "This will permanently delete all employees, teams, and imported columns/data. Continue?",
    )
    if (!firstConfirm) {
      return
    }

    const secondConfirm = window.confirm(
      "Final confirmation: this action cannot be undone. Delete ALL app data now?",
    )
    if (!secondConfirm) {
      return
    }

    try {
      setResettingData(true)
      setGlobalError(null)

      await staffApi.resetAllData()

      localStorage.removeItem(COLUMN_PREFS_KEY)
      localStorage.removeItem(COLUMN_PREFS_VERSION_KEY)
      localStorage.removeItem(COLUMN_LABEL_OVERRIDES_KEY)
      localStorage.removeItem(COLUMN_WIDTHS_KEY)
      Object.keys(localStorage).forEach((key) => {
        if (
          key.startsWith(`${COLUMN_PREFS_KEY}:`)
          || key.startsWith(`${COLUMN_PREFS_VERSION_KEY}:`)
          || key.startsWith(`${COLUMN_LABEL_OVERRIDES_KEY}:`)
          || key.startsWith(`${COLUMN_WIDTHS_KEY}:`)
        ) {
          localStorage.removeItem(key)
        }
      })

      setColumnPreferences({ order: [], hidden: [] })
      setColumnLabelOverrides({})
      setColumnWidths({})
      setSavedColumnPreferences({ order: [], hidden: [] })
      setSavedColumnLabelOverrides({})
      setSavedColumnWidths({})
      setImportReport(null)
      setImportSelectedFiles([])
      setImportColumnOptions([])
      setImportSelectedColumnKeys([])
      setSearchTerm("")
      setStaffGroupFilter("employee_list")
      setTeamFilter(ALL_TEAMS_OPTION)
      setStartDateFilter("")
      setEmployeeGroupCounts({
        employeeList: 0,
        onboarding: 0,
        offboarding: 0,
        internalMovement: 0,
      })
      setCurrentPage(1)
      setImportDrawerOpen(false)

      triggerReload()
    } catch (error) {
      setGlobalError(getErrorMessage(error))
    } finally {
      setResettingData(false)
    }
  }

  if (!isAuthenticated) {
    return (
      <div className="min-h-screen bg-[var(--bg)] text-[var(--text-primary)]">
        <header className="sticky top-0 z-40 h-16 border-b border-[var(--border)] bg-[var(--bg)]/95 px-5 backdrop-blur">
          <div className="flex h-full w-full items-center gap-4">
            <div className="flex min-w-[220px] items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-[8px] bg-[var(--primary)]/15 text-[var(--primary)]">
                <Users size={18} />
              </div>
              <span className="text-[30px] font-bold leading-none">Staff Kit</span>
            </div>

            <div className="ml-auto flex items-center gap-4">
              <div className="flex items-center rounded-[8px] border border-[var(--border)] p-1">
                <button
                  className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${
                    theme === "dark"
                      ? "bg-[var(--primary)]/20 text-[var(--primary)]"
                      : "text-[var(--text-secondary)]"
                  }`}
                  onClick={() => setTheme("dark")}
                  type="button"
                >
                  <span className="inline-flex items-center gap-1">
                    <Moon size={14} />
                    Dark
                  </span>
                </button>
                <button
                  className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${
                    theme === "light"
                      ? "bg-[var(--primary)]/20 text-[var(--primary)]"
                      : "text-[var(--text-secondary)]"
                  }`}
                  onClick={() => setTheme("light")}
                  type="button"
                >
                  <span className="inline-flex items-center gap-1">
                    <Sun size={14} />
                    Light
                  </span>
                </button>
              </div>
            </div>
          </div>
        </header>

        <main className="flex h-[calc(100vh-4rem)] items-center justify-center px-4 py-6">
          <section className="w-full max-w-[440px] rounded-[14px] border border-[var(--border)] bg-[var(--surface)] p-5 shadow-[0_8px_20px_-12px_rgba(0,0,0,0.6)]">
            <h1 className="text-2xl font-bold">Login</h1>
            <p className="mt-1 text-sm text-[var(--text-secondary)]">
              Chọn local account để vào hệ thống. Mọi cài đặt hiển thị cột sẽ lưu theo profile user.
            </p>

            {globalError && (
              <div className="mt-4 rounded-[10px] border border-red-500/50 bg-red-500/10 px-3 py-2 text-sm text-red-300">
                {globalError}
              </div>
            )}

            {(isBootstrapping || isLoadingAccounts) && (
              <div className="mt-4 flex items-center gap-2 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/40 px-3 py-2 text-sm text-[var(--text-secondary)]">
                <LoaderCircle className="animate-spin" size={15} />
                Loading local accounts...
              </div>
            )}

            {!isBootstrapping && !isLoadingAccounts && accounts.length === 0 && (
              <div className="mt-4 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 px-3 py-2 text-sm text-[var(--text-secondary)]">
                Chưa có account local. Vui lòng tạo account trong database trước.
              </div>
            )}

            {accounts.length > 0 && (
              <form className="mt-4 space-y-3" onSubmit={(event) => void handleLoginSubmit(event)}>
                <label className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                  Local Account
                </label>
                <select
                  className="form-input"
                  value={loginAccountId ?? ""}
                  onChange={(event) => {
                    const nextValue = Number(event.target.value)
                    setLoginAccountId(Number.isFinite(nextValue) ? nextValue : null)
                  }}
                  disabled={isSigningIn}
                >
                  {accounts.map((account) => (
                    <option key={account.id} value={account.id}>
                      {account.displayName} ({account.role})
                    </option>
                  ))}
                </select>

                <button
                  className="inline-flex w-full items-center justify-center gap-2 rounded-[8px] bg-[var(--primary)] px-4 py-2.5 font-semibold text-[#00131c] transition hover:brightness-110 disabled:opacity-50"
                  type="submit"
                  disabled={loginAccountId === null || isSigningIn}
                >
                  {isSigningIn ? <LoaderCircle className="animate-spin" size={16} /> : <LogIn size={16} />}
                  {isSigningIn ? "Signing in..." : "Login"}
                </button>
              </form>
            )}
          </section>
        </main>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--text-primary)]">
      <header className="sticky top-0 z-40 h-16 border-b border-[var(--border)] bg-[var(--bg)]/95 px-5 backdrop-blur">
        <div className="flex h-full w-full items-center gap-4">
          <div className="flex min-w-[220px] items-center gap-3">
            <div className="flex h-9 w-9 items-center justify-center rounded-[8px] bg-[var(--primary)]/15 text-[var(--primary)]">
              <Users size={18} />
            </div>
            <span className="text-[30px] font-bold leading-none">Staff Kit</span>
          </div>

          <div className="relative hidden flex-1 lg:block">
            <Search className="absolute left-4 top-1/2 -translate-y-1/2 text-[var(--text-secondary)]" size={18} />
            <input
              className="h-10 w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] pl-11 pr-4 text-sm text-[var(--text-primary)] outline-none transition focus:border-[var(--primary)]"
              placeholder="Search by employee ID, name, email, project..."
              value={searchTerm}
              onChange={(event) => setSearchTerm(event.target.value)}
            />
          </div>

          <div className="ml-auto flex items-center gap-4">
            <div className="flex items-center rounded-[8px] border border-[var(--border)] p-1">
              <button
                className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${
                  theme === "dark"
                    ? "bg-[var(--primary)]/20 text-[var(--primary)]"
                    : "text-[var(--text-secondary)]"
                }`}
                onClick={() => setTheme("dark")}
                type="button"
              >
                <span className="inline-flex items-center gap-1">
                  <Moon size={14} />
                  Dark
                </span>
              </button>
              <button
                className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${
                  theme === "light"
                    ? "bg-[var(--primary)]/20 text-[var(--primary)]"
                    : "text-[var(--text-secondary)]"
                }`}
                onClick={() => setTheme("light")}
                type="button"
              >
                <span className="inline-flex items-center gap-1">
                  <Sun size={14} />
                  Light
                </span>
              </button>
            </div>

            <div className="hidden items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs text-[var(--text-secondary)] md:flex">
              <span className="font-semibold text-[var(--text-primary)]">{activeAccountName}</span>
              <span className="rounded-[999px] border border-[var(--border)] px-2 py-[1px] uppercase tracking-[0.06em]">
                {activeAccount?.role ?? "user"}
              </span>
            </div>

            <button
              className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
              onClick={handleLogout}
              type="button"
            >
              <LogOut size={14} />
              Logout
            </button>

            <button className="hidden rounded-[8px] border border-[var(--border)] p-2 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] md:block" type="button">
              <Settings size={18} />
            </button>
          </div>
        </div>
      </header>

      <div className="flex w-full">
        <aside className="hidden h-[calc(100vh-4rem)] w-64 shrink-0 flex-col border-r border-[var(--border)] px-5 py-5 lg:flex">
          <nav className="space-y-2">
            <button className={`nav-button ${activeView === "employees" ? "nav-button-active" : ""}`} onClick={() => setActiveView("employees")} type="button">
              <Users size={18} />
              Employees
            </button>
            <button className={`nav-button ${activeView === "teams" ? "nav-button-active" : ""}`} onClick={() => setActiveView("teams")} type="button">
              <Users size={18} />
              Teams
            </button>
            <button className={`nav-button ${activeView === "settings" ? "nav-button-active" : ""}`} onClick={() => setActiveView("settings")} type="button">
              <Settings size={18} />
              Settings
            </button>
          </nav>

          <div className="mt-8 flex flex-col items-center gap-3">
            {STAFF_GROUP_BUTTONS.map((button) => {
              const count = getGroupCount(employeeGroupCounts, button.key)
              const active = staffGroupFilter === button.key
              return (
                <button
                  key={button.key}
                  className={`group-capsule-button ${active ? "group-capsule-button-active" : ""}`}
                  onClick={() => {
                    setStaffGroupFilter(button.key)
                    setActiveView("employees")
                  }}
                  type="button"
                >
                  <span>{button.label}</span>
                  <span className="group-capsule-count">{count}</span>
                </button>
              )
            })}
          </div>

          <div className="mt-auto rounded-[12px] border border-[var(--border)] bg-[linear-gradient(135deg,rgba(0,180,216,0.18),rgba(21,21,25,0.95))] p-4 shadow-[0_4px_6px_-1px_rgba(0,0,0,0.1)]">
            <div className="text-xs font-semibold tracking-[0.08em] text-[var(--primary)]">DATABASE</div>
            <div className="mt-2 text-xs text-[var(--text-secondary)]">{dbStatus?.initialized ? "Ready" : "Not initialized"}</div>
            <div className="mt-1 break-all text-[10px] text-[var(--text-secondary)]">{dbStatus?.dbPath ?? "-"}</div>
          </div>
        </aside>

        <main className="h-[calc(100vh-4rem)] flex-1 overflow-y-auto pb-24">
          {globalError && (
            <div className="mx-4 mt-4 rounded-[10px] border border-red-500/50 bg-red-500/10 px-4 py-3 text-sm text-red-300 md:mx-8">
              {globalError}
            </div>
          )}

          {isBootstrapping && (
            <div className="mx-4 mt-4 flex items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-4 py-3 text-sm text-[var(--text-secondary)] md:mx-8">
              <LoaderCircle className="animate-spin" size={16} />
              Initializing local database...
            </div>
          )}

          {activeView === "employees" && (
            <section>
              <div className="flex flex-col gap-4 border-b border-[var(--border)] px-4 py-7 md:px-8">
                <div className="flex items-start gap-4">
                  <div>
                    <h1 className="text-[34px] font-bold leading-tight">{selectedGroupLabel}</h1>
                    <p className="mt-1 text-[15px] text-[var(--text-secondary)]">
                      Current employees in this group: <span className="font-semibold text-[var(--text-primary)]">{selectedGroupTotal}</span>. User can choose visible columns and reorder them by drag-and-drop.
                    </p>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-3">
                  <div className="relative" ref={teamFilterMenuRef}>
                    <div className={`filter-chip ${isTeamFilterMenuOpen ? "filter-chip-open" : ""}`}>
                      <Users size={15} />
                      <button
                        className="filter-select-button"
                        onClick={() => setTeamFilterMenuOpen((value) => !value)}
                        type="button"
                      >
                        <span className="truncate">{teamFilter}</span>
                        <ChevronDown size={14} className={`transition ${isTeamFilterMenuOpen ? "rotate-180" : ""}`} />
                      </button>
                    </div>

                    {isTeamFilterMenuOpen && (
                      <div className="filter-menu">
                        {teamFilterOptions.map((name) => {
                          const isActive = teamFilter === name
                          return (
                            <button
                              key={name}
                              className={`filter-menu-item ${isActive ? "filter-menu-item-active" : ""}`}
                              onClick={() => {
                                setTeamFilter(name)
                                setTeamFilterMenuOpen(false)
                              }}
                              type="button"
                            >
                              <span className="truncate">{name}</span>
                              {isActive ? <Check size={14} /> : null}
                            </button>
                          )
                        })}
                      </div>
                    )}
                  </div>

                  <div className="filter-chip">
                    <Calendar size={15} />
                    <input
                      type="date"
                      value={startDateFilter}
                      onChange={(event) => setStartDateFilter(event.target.value)}
                      className="filter-date"
                    />
                  </div>

                  <button className="text-sm font-semibold text-[var(--primary)] transition hover:opacity-80" onClick={clearFilters} type="button">
                    Clear Filters
                  </button>

                  <div className="ml-auto flex items-center gap-2">
                    <button className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium" onClick={() => setColumnsDrawerOpen(true)} type="button">
                      <Columns3 size={16} />
                      Columns
                    </button>
                    <button
                      className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-4 py-2.5 font-semibold text-[#00131c] shadow-[0_4px_6px_-1px_rgba(0,0,0,0.1)] transition hover:brightness-110"
                      onClick={openCreateDrawer}
                      type="button"
                    >
                      <Plus size={18} />
                      Add Employee
                    </button>
                  </div>
                </div>
              </div>

              <div className="px-4 py-6 md:px-8">
                <div className="overflow-hidden rounded-[12px] border border-[var(--border)] bg-[var(--surface)] shadow-[0_4px_6px_-1px_rgba(0,0,0,0.1)]">
                  <div className="hidden max-h-[calc(100vh-350px)] overflow-auto xl:block">
                    <table className="min-w-[1500px] text-left text-sm">
                      <thead className="table-head text-xs uppercase tracking-[0.04em] text-[var(--text-secondary)]">
                        <tr>
                          {visibleColumns.map((column) => (
                            <th
                              key={column.key}
                              className={`relative px-3 py-3 ${column.key === "actions" ? "text-right" : ""}`}
                              style={{
                                width: `${resolvedColumnWidths[column.key] ?? 170}px`,
                                minWidth: `${resolvedColumnWidths[column.key] ?? 170}px`,
                              }}
                            >
                              <div className={`flex items-center gap-2 ${column.key === "actions" ? "justify-end" : "justify-between"}`}>
                                <span>{column.label}</span>
                                {column.key !== "actions" && (
                                  <button
                                    className="column-resize-handle"
                                    onMouseDown={(event) => startColumnResize(event, column.key)}
                                    type="button"
                                    aria-label={`Resize ${column.label} column`}
                                  />
                                )}
                              </div>
                            </th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {employees.map((employee, index) => (
                          <tr key={employee.id} className="table-row border-t border-[var(--border)] transition hover:bg-[var(--surface-hover)]">
                            {visibleColumns.map((column) => (
                              <td
                                key={`${employee.id}-${column.key}`}
                                className={`px-3 py-3 text-[var(--text-secondary)] ${column.key === "employeeId" ? "font-semibold text-[var(--primary)]" : ""} ${column.key === "fullName" ? "font-medium text-[var(--text-primary)]" : ""} ${column.key === "actions" ? "text-right" : ""}`}
                                style={{
                                  width: `${resolvedColumnWidths[column.key] ?? 170}px`,
                                  minWidth: `${resolvedColumnWidths[column.key] ?? 170}px`,
                                }}
                              >
                                {readCellValue(employee, column.key, index)}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>

                  <div className="space-y-3 p-4 xl:hidden">
                    {employees.map((employee, index) => (
                      <article key={employee.id} className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/35 p-4">
                        <div className="flex items-start justify-between gap-3">
                          <div>
                            <div className="font-semibold text-[var(--text-primary)]">{employee.fullName}</div>
                            <div className="text-xs text-[var(--text-secondary)]">{employee.employeeId}</div>
                          </div>
                          <span className="text-xs text-[var(--text-secondary)]">#{(currentPage - 1) * rowsPerPage + index + 1}</span>
                        </div>
                        <div className="mt-3 space-y-1 text-xs text-[var(--text-secondary)]">
                          <div>{employee.teamName ?? "-"}</div>
                          <div>{employee.project ?? "-"}</div>
                          <div>{employee.email ?? "-"}</div>
                        </div>
                      </article>
                    ))}
                  </div>
                </div>

                <div className="mt-3 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] px-4 py-3">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div className="text-[var(--text-secondary)]">
                      Showing <span className="text-[var(--text-primary)]">{employees.length}</span> of{" "}
                      <span className="text-[var(--text-primary)]">{totalEmployees}</span> employees
                    </div>

                    <div className="flex items-center gap-2">
                      {(isLoadingEmployees || isLoadingColumns) && (
                        <LoaderCircle className="animate-spin text-[var(--text-secondary)]" size={16} />
                      )}
                      <label className="text-xs text-[var(--text-secondary)]">Rows:</label>
                      <select className="rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-2 py-1 text-sm" value={rowsPerPage} onChange={(event) => setRowsPerPage(Number(event.target.value))}>
                        <option value={15}>15</option>
                        <option value={30}>30</option>
                        <option value={50}>50</option>
                        <option value={100}>100</option>
                        <option value={500}>500</option>
                      </select>

                      <div className="ml-3 flex items-center gap-1">
                        <button className="pager-button" disabled={currentPage <= 1} onClick={() => setCurrentPage((value) => Math.max(1, value - 1))} type="button">
                          Previous
                        </button>
                        <span className="px-2 text-sm text-[var(--text-secondary)]">{currentPage}/{totalPages}</span>
                        <button className="pager-button" disabled={currentPage >= totalPages} onClick={() => setCurrentPage((value) => Math.min(totalPages, value + 1))} type="button">
                          Next
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </section>
          )}

          {activeView === "teams" && (
            <section className="px-4 py-7 md:px-8">
              <div className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
                <div>
                  <h2 className="text-[30px] font-bold">Teams</h2>
                  <p className="mt-1 text-[15px] text-[var(--text-secondary)]">Manage team names used by employee records.</p>
                </div>
                <div className="flex items-center gap-2">
                  <input className="form-input w-[260px]" placeholder="New team name" value={newTeamName} onChange={(event) => setNewTeamName(event.target.value)} />
                  <button className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-4 py-2.5 font-semibold text-[#00131c] transition hover:brightness-110" onClick={handleCreateTeam} type="button" disabled={isSavingTeam}>
                    {isSavingTeam ? <LoaderCircle className="animate-spin" size={16} /> : <Plus size={18} />}
                    Add
                  </button>
                </div>
              </div>

              <div className="grid gap-3">
                {teams.map((team) => (
                  <div key={team.id} className="flex items-center justify-between rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-4 py-3">
                    <div>
                      <div className="font-semibold">{team.name}</div>
                      <div className="text-sm text-[var(--text-secondary)]">{team.memberCount} employees</div>
                    </div>
                    <div className="flex items-center gap-2">
                      <button className="icon-button" onClick={() => handleRenameTeam(team)} type="button">Rename</button>
                      <button className="icon-button text-[var(--error)]" onClick={() => handleDeleteTeam(team)} type="button">Delete</button>
                    </div>
                  </div>
                ))}
              </div>
              {isLoadingTeams && <div className="mt-3 text-sm text-[var(--text-secondary)]">Refreshing teams...</div>}
            </section>
          )}

          {activeView === "settings" && (
            <section className="px-4 py-7 md:px-8">
              <h2 className="text-[30px] font-bold">Settings</h2>
              <div className="mt-4 max-w-3xl rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
                <div className="text-sm font-semibold text-[var(--text-primary)]">Admin Portal (Local Accounts)</div>
                <p className="mt-1 text-xs text-[var(--text-secondary)]">
                  Manage local accounts in this app. Column layout is saved per account profile.
                </p>

                <div className="mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_130px_110px]">
                  <input
                    className="form-input"
                    placeholder="New account name..."
                    value={newAccountName}
                    onChange={(event) => setNewAccountName(event.target.value)}
                    disabled={isMutatingAccounts}
                  />
                  <select
                    className="rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-2 py-2 text-sm"
                    value={newAccountRole}
                    onChange={(event) => setNewAccountRole(event.target.value === "admin" ? "admin" : "user")}
                    disabled={isMutatingAccounts}
                  >
                    <option value="user">User</option>
                    <option value="admin">Admin</option>
                  </select>
                  <button
                    className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                    onClick={() => void handleCreateAccount()}
                    type="button"
                    disabled={isMutatingAccounts}
                  >
                    {isMutatingAccounts ? "Saving..." : "Add Account"}
                  </button>
                </div>

                <div className="mt-3 space-y-2">
                  {isLoadingAccounts && (
                    <div className="text-xs text-[var(--text-secondary)]">Loading accounts...</div>
                  )}
                  {accounts.map((account) => {
                    const isActive = activeAccountId === account.id
                    return (
                      <div
                        key={account.id}
                        className={`rounded-[8px] border px-3 py-2 ${isActive ? "border-[var(--primary)]/60 bg-[var(--primary)]/10" : "border-[var(--border)] bg-[var(--surface-hover)]/25"}`}
                      >
                        <div className="flex flex-wrap items-center gap-2">
                          <div className="text-sm font-semibold text-[var(--text-primary)]">{account.displayName}</div>
                          <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                            {account.role}
                          </span>
                          {isActive && (
                            <span className="rounded-[999px] border border-[var(--primary)]/45 px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--primary)]">
                              active
                            </span>
                          )}
                          <div className="ml-auto flex items-center gap-2">
                            <button className="icon-button text-xs" onClick={() => void handleActivateAccount(account.id)} type="button" disabled={isMutatingAccounts || isLoadingAccounts}>
                              Use
                            </button>
                            <button className="icon-button text-xs" onClick={() => void handleRenameAccount(account)} type="button" disabled={isMutatingAccounts || isLoadingAccounts}>
                              Rename
                            </button>
                            <button className="icon-button text-xs text-[var(--error)]" onClick={() => void handleDeleteAccount(account)} type="button" disabled={isMutatingAccounts || isLoadingAccounts}>
                              Delete
                            </button>
                          </div>
                        </div>
                        <div className="mt-1 text-[11px] text-[var(--text-secondary)]">Profile ID: {account.accountKey}</div>
                      </div>
                    )
                  })}
                </div>
              </div>

              <div className="mt-4 max-w-3xl space-y-3 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4 text-sm text-[var(--text-secondary)]">
                <div>
                  <span className="font-semibold text-[var(--text-primary)]">SQLite status:</span>{" "}
                  {dbStatus?.initialized ? "initialized" : "not initialized"}
                </div>
                <div>
                  <span className="font-semibold text-[var(--text-primary)]">SQLite version:</span>{" "}
                  {dbStatus?.sqliteVersion || "-"}
                </div>
                <div>
                  <span className="font-semibold text-[var(--text-primary)]">Database path:</span>
                  <div className="mt-1 break-all text-xs">{dbStatus?.dbPath || "-"}</div>
                </div>
              </div>

              <div className="mt-4 max-w-3xl rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
                <div className="text-sm font-semibold text-[var(--text-primary)]">Import Excel</div>
                <p className="mt-1 text-xs text-[var(--text-secondary)]">
                  Select one or multiple Excel files, then choose the columns before importing into app data.
                </p>
                <button
                  className="mt-3 inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
                  onClick={() => void handlePickImportFiles()}
                  type="button"
                  disabled={isImporting}
                >
                  {isImporting ? <LoaderCircle className="animate-spin" size={14} /> : <Upload size={14} />}
                  {isImporting ? "Preparing import..." : "Import Excel"}
                </button>
              </div>

              <div className="mt-4 grid max-w-3xl grid-cols-1 gap-2 text-sm text-[var(--text-secondary)] sm:grid-cols-2">
                {STAFF_GROUP_BUTTONS.map((item) => (
                  <div key={item.key} className="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2">
                    <div className="text-xs uppercase tracking-[0.06em]">{item.label}</div>
                    <div className="mt-1 text-[18px] font-semibold text-[var(--text-primary)]">{getGroupCount(employeeGroupCounts, item.key)}</div>
                  </div>
                ))}
              </div>

              <div className="mt-4 max-w-3xl rounded-[12px] border border-red-500/45 bg-red-500/10 p-4">
                <div className="text-sm font-semibold text-red-300">Temporary Reset (Data Wipe)</div>
                <p className="mt-1 text-xs text-red-200/90">
                  Delete all employees, teams, imported dynamic columns, and imported values. Use this only while preparing data.
                </p>
                <button
                  className="mt-3 inline-flex items-center gap-2 rounded-[8px] border border-red-400/60 bg-red-500/20 px-3 py-2 text-sm font-semibold text-red-100 disabled:opacity-50"
                  onClick={() => void handleResetAllData()}
                  type="button"
                  disabled={isResettingData}
                >
                  {isResettingData ? <LoaderCircle className="animate-spin" size={14} /> : null}
                  {isResettingData ? "Resetting..." : "Reset All Data (Temporary)"}
                </button>
              </div>
            </section>
          )}
        </main>
      </div>

      <nav className="fixed inset-x-0 bottom-0 z-30 border-t border-[var(--border)] bg-[var(--surface)] px-2 py-2 lg:hidden">
        <div className="grid grid-cols-3 gap-2">
          <button className={`mobile-nav ${activeView === "employees" ? "mobile-nav-active" : ""}`} onClick={() => setActiveView("employees")} type="button">
            <Users size={16} />
            Employees
          </button>
          <button className={`mobile-nav ${activeView === "teams" ? "mobile-nav-active" : ""}`} onClick={() => setActiveView("teams")} type="button">
            <Users size={16} />
            Teams
          </button>
          <button className={`mobile-nav ${activeView === "settings" ? "mobile-nav-active" : ""}`} onClick={() => setActiveView("settings")} type="button">
            <Settings size={16} />
            Settings
          </button>
        </div>
      </nav>

      <Drawer open={isColumnsDrawerOpen} onClose={() => setColumnsDrawerOpen(false)} title="Column Preferences" widthClass="w-[460px]">
        <div className="space-y-4">
          <p className="text-sm text-[var(--text-secondary)]">
            Toggle visibility and drag to reorder columns. New fields from imported Excel files will appear here automatically.
          </p>
          <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 px-3 py-2 text-xs text-[var(--text-secondary)]">
            Active profile: <span className="font-semibold text-[var(--text-primary)]">{activeAccountName}</span> ({activeUserScope})
          </div>

          <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 p-3">
            <div className="text-xs font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">Add Dynamic Column</div>
            <div className="mt-2 flex items-center gap-2">
              <input
                className="form-input"
                placeholder="New column title..."
                value={newColumnTitle}
                onChange={(event) => setNewColumnTitle(event.target.value)}
              />
              <button
                className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold disabled:opacity-50"
                onClick={() => void handleAddColumn()}
                type="button"
                disabled={isMutatingColumns}
              >
                Add
              </button>
            </div>
          </div>

          <input
            className="form-input"
            placeholder="Search columns..."
            value={columnSearchTerm}
            onChange={(event) => setColumnSearchTerm(event.target.value)}
          />

          <div className="text-xs text-[var(--text-secondary)]">
            Visible columns: {effectiveColumnPreferences.order.length - effectiveColumnPreferences.hidden.length}/{effectiveColumnPreferences.order.length}
          </div>

          <div className="space-y-2">
            {filteredColumnKeys.map((key) => {
              const column = configurableColumnMap.get(key)
              if (!column) {
                return null
              }

              const visible = !effectiveColumnPreferences.hidden.includes(key)

              return (
                <div
                  key={key}
                  className="flex items-center gap-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/35 px-3 py-2"
                  draggable
                  onDragStart={() => setDraggingColumnKey(key)}
                  onDragOver={(event) => {
                    event.preventDefault()
                  }}
                  onDrop={() => {
                    if (draggingColumnKey) {
                      reorderColumns(draggingColumnKey, key)
                    }
                    setDraggingColumnKey(null)
                  }}
                  onDragEnd={() => setDraggingColumnKey(null)}
                >
                  <GripVertical size={14} className="text-[var(--text-secondary)]" />
                  <input type="checkbox" checked={visible} onChange={() => toggleColumnVisibility(key)} />
                  <div className="flex-1">
                    <div className="text-sm font-medium">{column.label}</div>
                    <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">{column.source}</div>
                  </div>
                  <button
                    className="rounded-[6px] border border-[var(--border)] px-2 py-1 text-xs"
                    onClick={() => void handleRenameColumn(column)}
                    type="button"
                    disabled={isMutatingColumns}
                  >
                    Rename
                  </button>
                  {column.source === "dynamic" && (
                    <button
                      className="rounded-[6px] border border-[var(--border)] px-2 py-1 text-xs text-[var(--error)]"
                      onClick={() => void handleDeleteColumn(column)}
                      type="button"
                      disabled={isMutatingColumns}
                    >
                      Delete
                    </button>
                  )}
                </div>
              )
            })}
            {filteredColumnKeys.length === 0 && (
              <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/30 px-3 py-2 text-sm text-[var(--text-secondary)]">
                No columns match your search.
              </div>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-2 border-t border-[var(--border)] pt-3">
            <button
              className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
              onClick={handleSaveColumnProfile}
              type="button"
              disabled={!hasUnsavedColumnProfileChanges}
            >
              Save for {activeAccountName}
            </button>
            <button className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium" onClick={resetColumnPreferences} type="button">
              Reset Default
            </button>
            <div className="ml-auto text-xs text-[var(--text-secondary)]">
              {hasUnsavedColumnProfileChanges ? "Unsaved changes" : "All changes saved"}
            </div>
          </div>
        </div>
      </Drawer>

      <Drawer open={isImportDrawerOpen} onClose={() => setImportDrawerOpen(false)} title="Import from Excel" widthClass="w-[560px]">
        <div className="space-y-5">
          <div className="rounded-[8px] border border-dashed border-[var(--border)] bg-[var(--surface-hover)]/35 p-4">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
              <Upload size={16} className="text-[var(--primary)]" />
              Step 1: Select Excel files
            </div>
            <div className="mt-2 text-xs text-[var(--text-secondary)]">
              You can select one or multiple files. The app will detect Staff ID and merge data by employee.
            </div>
            <div className="mt-3 space-y-2 text-xs text-[var(--text-secondary)]">
              {importSelectedFiles.length === 0 ? (
                <div className="rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-2">No files selected.</div>
              ) : (
                importSelectedFiles.map((file) => (
                  <div key={file} className="rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-2 py-2 break-all">
                    {file}
                  </div>
                ))
              )}
            </div>
          </div>

          {importColumnOptions.length > 0 && (
            <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-4">
              <div className="text-sm font-semibold text-[var(--text-primary)]">Step 2: Choose columns to import</div>
              <div className="mt-2 flex items-center gap-2">
                <button
                  className="rounded-[8px] border border-[var(--border)] px-2.5 py-1.5 text-xs font-medium"
                  onClick={selectAllOptionalImportColumns}
                  type="button"
                >
                  Select All Optional
                </button>
                <button
                  className="rounded-[8px] border border-[var(--border)] px-2.5 py-1.5 text-xs font-medium"
                  onClick={clearOptionalImportColumns}
                  type="button"
                >
                  Clear Optional
                </button>
                <div className="ml-auto text-xs text-[var(--text-secondary)]">
                  Selected: {effectiveImportColumnKeySet.size}/{importColumnOptions.length}
                </div>
              </div>

              <div className="mt-3 max-h-64 space-y-2 overflow-auto pr-1">
                {importColumnOptions.map((column) => {
                  const selected = effectiveImportColumnKeySet.has(column.key)
                  const disabled = column.required
                  return (
                    <label key={column.key} className="flex items-center gap-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2">
                      <input
                        type="checkbox"
                        checked={selected}
                        disabled={disabled}
                        onChange={() => toggleImportColumn(column)}
                      />
                      <div className="flex-1">
                        <div className="text-sm text-[var(--text-primary)]">{column.label}</div>
                        <div className="text-[11px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">{column.source}</div>
                      </div>
                      {column.required && (
                        <span className="rounded-[999px] border border-[var(--border)] px-2 py-0.5 text-[10px] uppercase tracking-[0.06em] text-[var(--text-secondary)]">
                          required
                        </span>
                      )}
                    </label>
                  )
                })}
              </div>
            </div>
          )}

          <div className="flex flex-wrap items-center gap-2 border-t border-[var(--border)] pt-4">
            <button className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium" onClick={() => void handlePickImportFiles()} type="button" disabled={isImporting}>
              {isImporting ? (
                <span className="inline-flex items-center gap-2">
                  <LoaderCircle className="animate-spin" size={14} />
                  Preparing
                </span>
              ) : (
                "Choose File(s)"
              )}
            </button>
            <button
              className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c] disabled:opacity-50"
              onClick={() => void handleImportSelectedColumns()}
              type="button"
              disabled={isImporting || importSelectedFiles.length === 0 || importColumnOptions.length === 0}
            >
              {isImporting ? "Importing..." : "Import to App"}
            </button>
            <button className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium" onClick={() => setImportDrawerOpen(false)} type="button">Close</button>
          </div>

          {importReport && (
            <div className="space-y-3 rounded-[8px] border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
              <div className="font-semibold">Import completed</div>
              <div className="grid grid-cols-2 gap-2 text-xs">
                <div>Main file: {importReport.sourceFile}</div>
                <div>Files: {importReport.sourceFiles.length}</div>
                <div>Sheet: {importReport.sheetName}</div>
                <div>Processed sheets: {importReport.processedSheets.join(", ") || "-"}</div>
                <div>Header row: {importReport.headerRow}</div>
                <div>Total rows: {importReport.totalRows}</div>
                <div>Inserted: {importReport.inserted}</div>
                <div>Updated: {importReport.updated}</div>
                <div>Skipped: {importReport.skipped}</div>
                <div>Failed: {importReport.failed}</div>
              </div>
              {importReport.errors.length > 0 && (
                <div className="max-h-44 overflow-auto rounded-[6px] border border-emerald-500/30 bg-black/20 p-2 text-xs">
                  {importReport.errors.slice(0, 12).map((item, index) => (
                    <div key={`${item.row}-${index}`}>
                      Row {item.row}{item.employeeId ? ` (${item.employeeId})` : ""}: {item.reason}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </Drawer>

      <Drawer open={isEmployeeDrawerOpen} onClose={() => setEmployeeDrawerOpen(false)} title={editingEmployee ? "Edit Employee" : "Add Employee"} widthClass="w-[500px]">
        <EmployeeForm
          employee={editingEmployee}
          teamOptions={teamOptions}
          onCancel={() => setEmployeeDrawerOpen(false)}
          onSave={() => {
            setEmployeeDrawerOpen(false)
            triggerReload()
          }}
          onDelete={() => {
            setEmployeeDrawerOpen(false)
            triggerReload()
          }}
        />
      </Drawer>
    </div>
  )
}

function EmployeeForm({
  employee,
  teamOptions,
  onSave,
  onCancel,
  onDelete,
}: {
  employee: EmployeeRecord | null
  teamOptions: string[]
  onSave: () => void
  onCancel: () => void
  onDelete: () => void
}) {
  const [employeeId, setEmployeeId] = useState("")
  const [fullName, setFullName] = useState("")
  const [nickName, setNickName] = useState("")
  const [teamName, setTeamName] = useState("")
  const [project, setProject] = useState("")
  const [jobTitle, setJobTitle] = useState("")
  const [email, setEmail] = useState("")
  const [cellphone, setCellphone] = useState("")
  const [dateOfBirth, setDateOfBirth] = useState("")
  const [gender, setGender] = useState("")
  const [aswStartDate, setAswStartDate] = useState("")
  const [clientStartDate, setClientStartDate] = useState("")
  const [contractEndDate, setContractEndDate] = useState("")
  const [clientYearOfServices, setClientYearOfServices] = useState("")
  const [computerName, setComputerName] = useState("")
  const [notes, setNotes] = useState("")
  const [isSaving, setSaving] = useState(false)
  const [submitError, setSubmitError] = useState<string | null>(null)

  useEffect(() => {
    setEmployeeId(employee?.employeeId ?? "")
    setFullName(employee?.fullName ?? "")
    setNickName(employee?.nickName ?? "")
    setTeamName(employee?.teamName ?? "")
    setProject(employee?.project ?? "")
    setJobTitle(employee?.jobTitle ?? "")
    setEmail(employee?.email ?? "")
    setCellphone(employee?.cellphone ?? "")
    setDateOfBirth(employee?.dateOfBirth ?? "")
    setGender(employee?.gender ?? "")
    setAswStartDate(employee?.aswStartDate ?? "")
    setClientStartDate(employee?.clientStartDate ?? "")
    setContractEndDate(employee?.contractEndDate ?? "")
    setClientYearOfServices(employee?.clientYearOfServices ?? "")
    setComputerName(employee?.computerName ?? "")
    setNotes(employee?.notes ?? "")
    setSubmitError(null)
  }, [employee])

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    try {
      setSaving(true)
      setSubmitError(null)

      const payload: EmployeePayload = {
        employeeId,
        fullName,
        nickName,
        teamName,
        project,
        jobTitle,
        email,
        cellphone,
        dateOfBirth,
        gender,
        aswStartDate,
        clientStartDate,
        contractEndDate,
        clientYearOfServices,
        computerName,
        notes,
      }

      if (employee) {
        await staffApi.updateEmployee(employee.id, payload)
      } else {
        await staffApi.createEmployee(payload)
      }

      onSave()
    } catch (error) {
      setSubmitError(getErrorMessage(error))
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async () => {
    if (!employee) return

    const accepted = window.confirm(`Delete employee '${employee.fullName}'?`)
    if (!accepted) {
      return
    }

    try {
      setSaving(true)
      setSubmitError(null)
      await staffApi.deleteEmployee(employee.id)
      onDelete()
    } catch (error) {
      setSubmitError(getErrorMessage(error))
    } finally {
      setSaving(false)
    }
  }

  return (
    <form className="space-y-3" onSubmit={handleSubmit}>
      {submitError && <div className="rounded-[8px] border border-red-500/50 bg-red-500/10 px-3 py-2 text-xs text-red-300">{submitError}</div>}

      <Field label="EE. ID" required>
        <input className="form-input uppercase" value={employeeId} onChange={(event) => setEmployeeId(event.target.value.toUpperCase())} required />
      </Field>

      <Field label="Vietnamese Name" required>
        <input className="form-input" value={fullName} onChange={(event) => setFullName(event.target.value)} required />
      </Field>

      <Field label="Nick Name">
        <input className="form-input" value={nickName} onChange={(event) => setNickName(event.target.value)} />
      </Field>

      <Field label="CLIENT (PMD)">
        <input className="form-input" value={teamName} onChange={(event) => setTeamName(event.target.value)} list="team-options" />
        <datalist id="team-options">
          {teamOptions.map((name) => (
            <option key={name} value={name} />
          ))}
        </datalist>
      </Field>

      <Field label="PROJECT">
        <input className="form-input" value={project} onChange={(event) => setProject(event.target.value)} />
      </Field>

      <Field label="Current Job Title">
        <input className="form-input" value={jobTitle} onChange={(event) => setJobTitle(event.target.value)} />
      </Field>

      <Field label="Working Email">
        <input className="form-input" value={email} onChange={(event) => setEmail(event.target.value)} type="email" />
      </Field>

      <Field label="Cellphone">
        <input className="form-input" value={cellphone} onChange={(event) => setCellphone(event.target.value)} />
      </Field>

      <Field label="D.O.B">
        <input className="form-input" value={dateOfBirth} onChange={(event) => setDateOfBirth(event.target.value)} type="date" />
      </Field>

      <Field label="Gender">
        <input className="form-input" value={gender} onChange={(event) => setGender(event.target.value)} />
      </Field>

      <Field label="ASW Start date">
        <input className="form-input" value={aswStartDate} onChange={(event) => setAswStartDate(event.target.value)} type="date" />
      </Field>

      <Field label="Client Start Date">
        <input className="form-input" value={clientStartDate} onChange={(event) => setClientStartDate(event.target.value)} type="date" />
      </Field>

      <Field label="Contract End date">
        <input className="form-input" value={contractEndDate} onChange={(event) => setContractEndDate(event.target.value)} placeholder="Date or text" />
      </Field>

      <Field label="Client Year of Services">
        <input className="form-input" value={clientYearOfServices} onChange={(event) => setClientYearOfServices(event.target.value)} />
      </Field>

      <Field label="Computer Name">
        <input className="form-input" value={computerName} onChange={(event) => setComputerName(event.target.value)} />
      </Field>

      <Field label="Notes">
        <textarea className="form-input min-h-20 resize-y" value={notes} onChange={(event) => setNotes(event.target.value)} rows={4} />
      </Field>

      <div className="flex flex-wrap gap-2 border-t border-[var(--border)] pt-4">
        <button type="submit" className="rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-[#00131c]" disabled={isSaving}>
          {isSaving ? "Saving..." : "Save"}
        </button>
        <button type="button" className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium" onClick={onCancel}>
          Cancel
        </button>
        {employee && (
          <button type="button" className="inline-flex items-center gap-2 rounded-[8px] border border-red-500/50 px-3 py-2 text-sm font-medium text-[var(--error)]" onClick={handleDelete}>
            <Trash2 size={15} />
            Delete
          </button>
        )}
      </div>
    </form>
  )
}

function Field({
  label,
  required,
  children,
}: {
  label: string
  required?: boolean
  children: ReactNode
}) {
  return (
    <label className="block space-y-1.5">
      <span className="text-sm text-[var(--text-secondary)]">
        {label}
        {required ? " (required)" : ""}
      </span>
      {children}
    </label>
  )
}

export default App
