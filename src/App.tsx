import { useCallback, useEffect, useMemo, useState } from "react"
import { ClipboardList, LoaderCircle, LogOut, Moon, Settings, Sun, Users } from "lucide-react"
import { staffApi } from "./services/staff-api"
import type { DatabaseStatus } from "./types/staff"
import type { AppView, Theme } from "./types/app"
import { getGroupCount, normalizeUserScope, buildScopedStorageKey } from "./lib/utils"
import { getUserErrorMessage } from "./lib/errorHandling"
import { useScopedStorageKeys } from "./lib/useScopedStorageKeys"
import {
  THEME_KEY,
  STAFF_GROUP_BUTTONS,
  DEFAULT_ACCOUNT_NAME,
} from "./lib/constants"

// Feature hooks
import { useAuthState } from "./features/auth/useAuthState"
import { useColumnState } from "./features/columns/useColumnState"
import { useEmployeeState } from "./features/employees/useEmployeeState"
import { useTableEdit } from "./features/employees/useTableEdit"
import { useImportState } from "./features/import/useImportState"
import { useTeamState } from "./features/teams/useTeamState"
import { useBorrowState } from "./features/borrow/useBorrowState"
import { useAssetDirectImportState } from "./features/assets/useAssetDirectImportState"
import { useAssetDashboardState } from "./features/assets/useAssetDashboardState"
import { useSettingsState } from "./features/settings/useSettingsState"

// Feature views
import { LoginPage } from "./features/auth/LoginPage"
import { EmployeeView } from "./features/employees/EmployeeView"
import { ColumnsDrawer } from "./features/columns/ColumnsDrawer"
import { ImportDrawer } from "./features/import/ImportDrawer"
import { TeamView } from "./features/teams/TeamView"
import { BorrowAdminView } from "./features/borrow/BorrowAdminView"
import { AssetImportWizard } from "./features/assets/AssetImportWizard"
import { SettingsView } from "./features/settings/SettingsView"

function App() {
  // ── App-level state ──────────────────────────────────────────────────────────
  const [theme, setThemeState] = useState<Theme>("dark")
  const [activeView, setActiveView] = useState<AppView>("employees")
  const [dbStatus, setDbStatus] = useState<DatabaseStatus | null>(null)
  const [dbReady, setDbReady] = useState(false)
  const [isBootstrapping, setBootstrapping] = useState(true)
  const [globalError, setGlobalError] = useState<string | null>(null)
  const [reloadToken, setReloadToken] = useState(0)
  const [sidebarNote, setSidebarNote] = useState("")
  const [noteSaved, setNoteSaved] = useState(false)

  const triggerReload = useCallback(() => setReloadToken((v) => v + 1), [])

  // Auto-clear globalError after 10s
  useEffect(() => {
    if (!globalError) return
    const timer = setTimeout(() => setGlobalError(null), 10000)
    return () => clearTimeout(timer)
  }, [globalError])

  // ── Theme ────────────────────────────────────────────────────────────────────
  // Kept here as a placeholder — actual setTheme defined after activeUserScope below
  const setTheme = useCallback((nextTheme: Theme, key?: string) => {
    setThemeState(nextTheme)
    localStorage.setItem(key ?? "staffkit-theme", nextTheme)
    document.documentElement.classList.toggle("dark", nextTheme === "dark")
  }, [])

  // Initial theme load (before login, use a generic key; swapped after login via effect below)
  useEffect(() => {
    const saved = (localStorage.getItem("staffkit-theme") as Theme | null) ?? "dark"
    setTheme(saved)
  }, [setTheme])

  // ── Auth ─────────────────────────────────────────────────────────────────────
  const auth = useAuthState({
    dbReady,
    reloadToken,
    onLoginSuccess: triggerReload,
    onLogout: () => {
      emp.resetEmployeeStateOnLogout()
      col.resetColumnStateOnLogout()
      edit.resetTableEditStateOnLogout()
    },
  })

  const { isAuthenticated, activeAccount, activeAccountName, canAccessSettings } = auth

  // ── Scoped storage keys (per user profile) ───────────────────────────────────
  const activeUserScope = useMemo(
    () => activeAccount?.accountKey ?? normalizeUserScope(DEFAULT_ACCOUNT_NAME),
    [activeAccount],
  )

  // Restore per-user theme preference on login / account switch
  const scopedThemeKey = buildScopedStorageKey(THEME_KEY, activeUserScope)
  useEffect(() => {
    const saved = (localStorage.getItem(scopedThemeKey) as Theme | null) ?? "dark"
    setTheme(saved, scopedThemeKey)
  }, [scopedThemeKey]) // eslint-disable-line react-hooks/exhaustive-deps

  // ── Employee + Column state ─────────────────────────────────────────────────

  const employeeState = useEmployeeState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
  })

  const emp = employeeState

  const selectedGroupLabel = useMemo(
    () => STAFF_GROUP_BUTTONS.find((item) => item.key === emp.staffGroupFilter)?.label ?? "Employee list",
    [emp.staffGroupFilter],
  )

  const scopedKeys = useScopedStorageKeys(activeAccount?.accountKey, emp.staffGroupFilter)

  const col = useColumnState({
    dbReady,
    isAuthenticated,
    reloadToken,
    ...scopedKeys,
    activeAccountName,
    setGlobalError,
  })

  // ── Employee state ───────────────────────────────────────────────────────────

  // ── Table edit state ─────────────────────────────────────────────────────────
  const edit = useTableEdit({
    employees: emp.employees,
    staffGroupFilter: emp.staffGroupFilter,
    canEditEmployeeTable: auth.canEditEmployeeTable,
    canEditEmployeeComputerName: auth.canEditEmployeeComputerName,
    triggerReload,
    setGlobalError,
  })

  // ── Import state ─────────────────────────────────────────────────────────────
  const imp = useImportState({
    staffGroupFilter: emp.staffGroupFilter,
    setGlobalError,
    triggerReload,
  })

  // ── Team state ───────────────────────────────────────────────────────────────
  const teamState = useTeamState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
    triggerReload,
  })

  // ── Settings state ───────────────────────────────────────────────────────────
  const settings = useSettingsState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
    triggerReload,
  })

  const borrow = useBorrowState({
    dbReady,
    isAuthenticated,
    isAdminAccount: auth.isAdminAccount,
    reloadToken,
    setGlobalError,
    triggerReload,
  })

  const assetImport = useAssetDirectImportState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
    triggerReload,
  })

  const assetDashboard = useAssetDashboardState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
  })

  // ── Bootstrap database ───────────────────────────────────────────────────────
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
        if (!disposed) setGlobalError(getUserErrorMessage(error))
      } finally {
        if (!disposed) setBootstrapping(false)
      }
    })()

    return () => { disposed = true }
  }, [])

  // ── Auto-reset to employees view when losing settings access ────────────────
  useEffect(() => {
    if ((activeView === "settings" && !canAccessSettings) || (activeView === "borrow" && !auth.isAdminAccount)) {
      setActiveView("employees")
    }
  }, [activeView, canAccessSettings, auth.isAdminAccount])

  // ── Reset column prefs on login ──────────────────────────────────────────────
  useEffect(() => {
    if (!isAuthenticated) return
    col.resetColumnPrefsOnAuth()
    // Load this user's private note
    const noteKey = buildScopedStorageKey("sidebar-note", activeUserScope)
    setSidebarNote(localStorage.getItem(noteKey) ?? "")
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAuthenticated, activeUserScope, scopedKeys.scopedColumnPrefsKey, scopedKeys.scopedColumnLabelOverridesKey, scopedKeys.scopedColumnWidthsKey])

  // ── Derived employee labels ───────────────────────────────────────────────────
  const selectedGroupTotal = useMemo(
    () => getGroupCount(emp.employeeGroupCounts, emp.staffGroupFilter),
    [emp.employeeGroupCounts, emp.staffGroupFilter],
  )

  // ── Login page ───────────────────────────────────────────────────────────────
  if (!isAuthenticated) {
    return (
      <LoginPage
        theme={theme}
        setTheme={setTheme}
        auth={auth}
        isBootstrapping={isBootstrapping}
        globalError={globalError}
        setGlobalError={setGlobalError}
      />
    )
  }

  // ── Main shell ───────────────────────────────────────────────────────────────
  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--text-primary)]">
      {/* Header */}
      <header className="sticky top-0 z-40 h-16 border-b border-[var(--border)] bg-[var(--bg)]/95 px-5 backdrop-blur">
        <div className="flex h-full w-full items-center gap-4">
          <div className="flex min-w-[220px] items-center gap-3">
            <div className="flex h-9 w-9 items-center justify-center rounded-[8px] bg-[var(--primary)]/15 text-[var(--primary)]">
              <Users size={18} />
            </div>
            <span className="text-[30px] font-bold leading-none">Staff Kit</span>
          </div>

          <div className="hidden flex-1 lg:block" />

          <div className="ml-auto flex items-center gap-4">
            {/* Theme toggle */}
            <div className="flex items-center rounded-[8px] border border-[var(--border)] p-1">
              <button
                className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${theme === "dark" ? "bg-[var(--primary)]/20 text-[var(--primary)]" : "text-[var(--text-secondary)]"}`}
                onClick={() => setTheme("dark")}
                type="button"
              >
                <span className="inline-flex items-center gap-1"><Moon size={14} />Dark</span>
              </button>
              <button
                className={`rounded-[6px] px-3 py-1 text-xs font-medium transition ${theme === "light" ? "bg-[var(--primary)]/20 text-[var(--primary)]" : "text-[var(--text-secondary)]"}`}
                onClick={() => setTheme("light")}
                type="button"
              >
                <span className="inline-flex items-center gap-1"><Sun size={14} />Light</span>
              </button>
            </div>

            {/* Active account */}
            <div className="hidden items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs text-[var(--text-secondary)] md:flex">
              <span className="font-semibold text-[var(--text-primary)]">{activeAccountName}</span>
              <span className="rounded-[999px] border border-[var(--border)] px-2 py-[1px] uppercase tracking-[0.06em]">
                {activeAccount?.role ?? "user"}
              </span>
            </div>

            {/* Logout */}
            <button
              className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-medium text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
              onClick={auth.handleLogout}
              type="button"
            >
              <LogOut size={14} />
              Logout
            </button>

            {/* Settings icon */}
            {canAccessSettings && (
              <button
                className="hidden rounded-[8px] border border-[var(--border)] p-2 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] md:block"
                type="button"
                onClick={() => setActiveView("settings")}
              >
                <Settings size={18} />
              </button>
            )}
          </div>
        </div>
      </header>

      <div className="flex w-full">
        {/* Sidebar */}
        <aside className="hidden h-[calc(100vh-4rem)] w-64 shrink-0 flex-col border-r border-[var(--border)] px-5 py-5 lg:flex">
          <nav className="space-y-2">
            <button
              className={`nav-button ${activeView === "employees" ? "nav-button-active" : ""}`}
              onClick={() => setActiveView("employees")}
              type="button"
            >
              <Users size={18} /> Employees
            </button>
            {auth.isAdminAccount && (
              <button
                className={`nav-button ${activeView === "borrow" ? "nav-button-active" : ""}`}
                onClick={() => setActiveView("borrow")}
                type="button"
              >
                <ClipboardList size={18} />
                <span>Borrow <span style={{ opacity: 0.55, fontWeight: 400 }}>/</span> Return</span>
              </button>
            )}
            <button
              className={`nav-button ${activeView === "teams" ? "nav-button-active" : ""}`}
              onClick={() => setActiveView("teams")}
              type="button"
            >
              <Users size={18} /> Teams
            </button>
            {canAccessSettings && (
              <button
                className={`nav-button ${activeView === "settings" ? "nav-button-active" : ""}`}
                onClick={() => setActiveView("settings")}
                type="button"
              >
                <Settings size={18} /> Settings
              </button>
            )}
          </nav>

          {/* Staff group filter capsules */}
          <div className="mt-8 flex flex-col items-center gap-3">
            {STAFF_GROUP_BUTTONS.map((button) => {
              const count = getGroupCount(emp.employeeGroupCounts, button.key)
              const active = emp.staffGroupFilter === button.key
              return (
                <button
                  key={button.key}
                  className={`group-capsule-button ${active ? "group-capsule-button-active" : ""}`}
                  onClick={() => {
                    emp.setStaffGroupFilter(button.key)
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

          {/* Note box — auto-saves to localStorage */}
          <div className="mt-auto flex flex-col gap-1">
            <div className="flex items-center justify-between">
              <span className="text-[10px] font-semibold uppercase tracking-[0.08em] text-[var(--text-secondary)]">Note</span>
              {noteSaved && (
                <span className="text-[9px] text-[var(--primary)] opacity-70">saved ✓</span>
              )}
            </div>
            <textarea
              className="h-72 w-full resize-none rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-xs text-[var(--text-primary)] placeholder-[var(--text-secondary)]/50 outline-none transition focus:border-[var(--primary)]/60 focus:ring-1 focus:ring-[var(--primary)]/30"
              spellCheck={false}
              value={sidebarNote}
              placeholder="Write a note..."
              onChange={(e) => {
                const val = e.target.value
                setSidebarNote(val)
                setNoteSaved(false)
                const noteKey = buildScopedStorageKey("sidebar-note", activeUserScope)
                // debounce via inline timeout ref approach
                clearTimeout((window as unknown as Record<string, number>).__noteTimer)
                  ; (window as unknown as Record<string, number>).__noteTimer = window.setTimeout(() => {
                    localStorage.setItem(noteKey, val)
                    setNoteSaved(true)
                    setTimeout(() => setNoteSaved(false), 2000)
                  }, 600)
              }}
            />
          </div>
        </aside>

        {/* Main content */}
        <main className="h-[calc(100vh-4rem)] flex-1 overflow-y-auto pb-24">
          {globalError && (
            <div
              className="mx-4 mt-4 cursor-pointer rounded-[10px] border border-red-500/50 bg-red-500/10 px-4 py-3 text-sm text-red-300 md:mx-8"
              onClick={() => setGlobalError(null)}
              title="Click to dismiss"
            >
              {globalError}
              <span className="ml-2 opacity-50 text-xs">(auto-dismiss in 10s · click to close)</span>
            </div>
          )}

          {isBootstrapping && (
            <div className="mx-4 mt-4 flex items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-4 py-3 text-sm text-[var(--text-secondary)] md:mx-8">
              <LoaderCircle className="animate-spin" size={16} />
              Initializing local database...
            </div>
          )}

          {activeView === "employees" && (
            <EmployeeView
              employeeState={emp}
              tableEdit={edit}
              columnState={col}
              canEditEmployeeTable={auth.canEditEmployeeTable}
              canEditEmployeeComputerName={auth.canEditEmployeeComputerName}
              isAdminAccount={auth.isAdminAccount}
              setGlobalError={setGlobalError}
              selectedGroupLabel={selectedGroupLabel}
              selectedGroupTotal={selectedGroupTotal}
            />
          )}

          {activeView === "teams" && <TeamView teamState={teamState} />}

          {activeView === "borrow" && (
            <BorrowAdminView
              auth={auth}
              borrow={borrow}
              settings={settings}
            />
          )}

          {activeView === "settings" && (
            <SettingsView
              auth={auth}
              activeUserScope={activeUserScope}
              settings={settings}
              assetImport={assetImport}
              assetDashboard={assetDashboard}
              importState={imp}
              employeeGroupCounts={emp.employeeGroupCounts}
              dbStatus={dbStatus}
              setGlobalError={setGlobalError}
              triggerReload={triggerReload}
            />
          )}
        </main>
      </div>

      {/* Mobile bottom nav */}
      <nav className="fixed inset-x-0 bottom-0 z-30 border-t border-[var(--border)] bg-[var(--surface)] px-2 py-2 lg:hidden">
        <div className={`grid gap-2 ${auth.isAdminAccount ? "grid-cols-4" : "grid-cols-2"}`}>
          <button
            className={`mobile-nav ${activeView === "employees" ? "mobile-nav-active" : ""}`}
            onClick={() => setActiveView("employees")}
            type="button"
          >
            <Users size={16} /> Employees
          </button>
          {auth.isAdminAccount && (
            <button
              className={`mobile-nav ${activeView === "borrow" ? "mobile-nav-active" : ""}`}
              onClick={() => setActiveView("borrow")}
              type="button"
            >
              <ClipboardList size={16} />
              <span>Borrow <span style={{ opacity: 0.55, fontWeight: 400 }}>/</span> Return</span>
            </button>
          )}
          <button
            className={`mobile-nav ${activeView === "teams" ? "mobile-nav-active" : ""}`}
            onClick={() => setActiveView("teams")}
            type="button"
          >
              <Users size={16} /> Teams
          </button>
          {canAccessSettings && (
            <button
              className={`mobile-nav ${activeView === "settings" ? "mobile-nav-active" : ""}`}
              onClick={() => setActiveView("settings")}
              type="button"
            >
              <Settings size={16} /> Settings
            </button>
          )}
        </div>
      </nav>

      {/* Drawers */}
      <ColumnsDrawer
        columnState={col}
        activeAccountName={activeAccountName}
        activeUserScope={activeUserScope}
        activeTableLabel={selectedGroupLabel}
        triggerReload={triggerReload}
      />

      <ImportDrawer importState={imp} />
      <AssetImportWizard assetImport={assetImport} />
    </div>
  )
}

export default App
