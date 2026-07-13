import { useCallback, useEffect, useState } from "react"
import { Database, Eye, EyeOff, LoaderCircle } from "lucide-react"
import { staffApi } from "../../services/staff-api"
import type { MssqlImportPreview, MssqlImportReport } from "../../types/staff"
import type { StaffGroupKey } from "../../types/app"
import { getUserErrorMessage } from "../../lib/errorHandling"

type MssqlImportPanelProps = {
    onClose: () => void
    triggerReload: () => void
    setGlobalError: (msg: string | null) => void
}

export function MssqlImportPanel({ onClose, triggerReload, setGlobalError }: MssqlImportPanelProps) {
    const [host, setHost] = useState("")
    const [port, setPort] = useState("1433")
    const [user, setUser] = useState("")
    const [password, setPassword] = useState("")
    const [showPassword, setShowPassword] = useState(false)
    const [query, setQuery] = useState("")
    const [staffGroup, setStaffGroup] = useState<StaffGroupKey>("employee_list")
    const [isTesting, setIsTesting] = useState(false)
    const [connectionOk, setConnectionOk] = useState(false)
    const [connectionError, setConnectionError] = useState<string | null>(null)
    const [preview, setPreview] = useState<MssqlImportPreview | null>(null)
    const [isPreviewing, setIsPreviewing] = useState(false)
    const [isImporting, setIsImporting] = useState(false)
    const [report, setReport] = useState<MssqlImportReport | null>(null)

    useEffect(() => {
        let cancelled = false
        staffApi.getMssqlConnectionDefaults()
            .then((defaults) => {
                if (cancelled) return
                setHost(defaults.host)
                setPort(String(defaults.port))
                setUser(defaults.user)
                setPassword(defaults.password)
            })
            .catch((err) => {
                if (!cancelled) {
                    setConnectionError(getUserErrorMessage(err))
                }
            })

        return () => {
            cancelled = true
        }
    }, [])

    const handleTestConnection = useCallback(async () => {
        if (!host.trim() || !port.trim() || !user.trim() || !password.trim()) return
        setIsTesting(true)
        setConnectionError(null)
        setConnectionOk(false)
        try {
            const ok = await staffApi.testMssqlConnection(host.trim(), Number(port.trim()), user.trim(), password)
            setConnectionOk(ok)
        } catch (err) {
            setConnectionError(getUserErrorMessage(err))
        } finally {
            setIsTesting(false)
        }
    }, [host, port, user, password])

    const handlePreview = useCallback(async () => {
        if (!host.trim() || !port.trim() || !user.trim() || !password.trim()) return
        setIsPreviewing(true)
        setPreview(null)
        setReport(null)
        try {
            const result = await staffApi.previewMssqlStaff(
                host.trim(), Number(port.trim()), user.trim(), password,
                query.trim() || undefined,
            )
            setPreview(result)
            setConnectionOk(true)
        } catch (err) {
            setGlobalError(getUserErrorMessage(err))
        } finally {
            setIsPreviewing(false)
        }
    }, [host, port, user, password, query, setGlobalError])

    const handleImport = useCallback(async () => {
        if (!preview) return
        setIsImporting(true)
        try {
            const result = await staffApi.importMssqlStaff(
                host.trim(), Number(port.trim()), user.trim(), password,
                query.trim() || undefined,
                staffGroup,
            )
            setReport(result)
            setPreview(null)
            triggerReload()
        } catch (err) {
            setGlobalError(getUserErrorMessage(err))
        } finally {
            setIsImporting(false)
        }
    }, [host, port, user, password, query, staffGroup, preview, triggerReload, setGlobalError])

    const isFormValid = host.trim() && port.trim() && user.trim() && password.trim()

    const staffGroupLabel = (() => {
        switch (staffGroup) {
            case "employee_list": return "Employee list"
            case "onboarding": return "Onboarding"
            case "offboarding": return "Offboarding"
            case "internal_movement": return "Internal Movement"
        }
    })()

    const inputClass = "w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--primary)]/60"

    return (
        <div className="space-y-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                <Database size={16} className="text-[var(--primary)]" />
                MSSQL Connection
            </div>

            <div className="grid grid-cols-2 gap-3">
                <div>
                    <label className="text-xs font-medium text-[var(--text-secondary)]">Host</label>
                    <input
                        className={inputClass}
                        placeholder="MSSQL host"
                        value={host}
                        onChange={(e) => {
                            setHost(e.target.value)
                            setConnectionOk(false)
                            setConnectionError(null)
                        }}
                    />
                </div>
                <div>
                    <label className="text-xs font-medium text-[var(--text-secondary)]">Port</label>
                    <input
                        className={inputClass}
                        placeholder="1433"
                        value={port}
                        onChange={(e) => {
                            setPort(e.target.value)
                            setConnectionOk(false)
                            setConnectionError(null)
                        }}
                    />
                </div>
                <div>
                    <label className="text-xs font-medium text-[var(--text-secondary)]">Username</label>
                    <input
                        className={inputClass}
                        placeholder="Username"
                        value={user}
                        onChange={(e) => {
                            setUser(e.target.value)
                            setConnectionOk(false)
                            setConnectionError(null)
                        }}
                    />
                </div>
                <div>
                    <label className="text-xs font-medium text-[var(--text-secondary)]">Password</label>
                    <div className="relative">
                        <input
                            className={inputClass}
                            type={showPassword ? "text" : "password"}
                            placeholder="password"
                            value={password}
                            onChange={(e) => {
                                setPassword(e.target.value)
                                setConnectionOk(false)
                                setConnectionError(null)
                            }}
                        />
                        <button
                            className="absolute right-2 top-1/2 -translate-y-1/2 text-[var(--text-secondary)]"
                            onClick={() => setShowPassword(!showPassword)}
                            type="button"
                            tabIndex={-1}
                        >
                            {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                        </button>
                    </div>
                </div>
            </div>

            <div className="flex items-center gap-2">
                <button
                    className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium disabled:opacity-50"
                    onClick={handleTestConnection}
                    disabled={!isFormValid || isTesting}
                    type="button"
                >
                    {isTesting ? (
                        <span className="inline-flex items-center gap-2">
                            <LoaderCircle className="animate-spin" size={14} />
                            Testing...
                        </span>
                    ) : (
                        "Test Connection"
                    )}
                </button>
                {connectionOk && (
                    <span className="text-sm text-green-500">Connected OK</span>
                )}
                {connectionError && (
                    <span className="text-sm text-red-400">{connectionError}</span>
                )}
            </div>

            <div>
                <label className="text-xs font-medium text-[var(--text-secondary)]">
                    SQL Query <span className="text-[var(--text-secondary)]/50">(optional — defaults to staff list)</span>
                </label>
                <textarea
                    className="mt-1 w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--primary)]/60"
                    rows={4}
                    placeholder="Custom SQL query — must return Code, Name, NickName, WorkEmail columns"
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                />
            </div>

            <div>
                <label className="text-xs font-medium text-[var(--text-secondary)]">Target Staff Group</label>
                <select
                    className="mt-1 w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--primary)]/60"
                    value={staffGroup}
                    onChange={(e) => setStaffGroup(e.target.value as StaffGroupKey)}
                >
                    <option value="employee_list">Employee list</option>
                    <option value="onboarding">Onboarding</option>
                    <option value="offboarding">Offboarding</option>
                    <option value="internal_movement">Internal Movement</option>
                </select>
            </div>

            <div className="flex gap-2">
                <button
                    className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium disabled:opacity-50"
                    onClick={handlePreview}
                    disabled={!isFormValid || !connectionOk || isPreviewing || isImporting}
                    type="button"
                >
                    {isPreviewing ? (
                        <span className="inline-flex items-center gap-2">
                            <LoaderCircle className="animate-spin" size={14} />
                            Fetching...
                        </span>
                    ) : (
                        "Preview Data"
                    )}
                </button>
                {preview && !isImporting && (
                    <button
                        className="rounded-[8px] bg-green-600 px-3 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:opacity-50"
                        onClick={handleImport}
                        disabled={isImporting}
                        type="button"
                    >
                        {isImporting ? "Importing..." : `Import ${preview.totalRows} employees to ${staffGroupLabel}`}
                    </button>
                )}
                <button
                    className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium"
                    onClick={onClose}
                    type="button"
                >
                    Close
                </button>
            </div>

            {isImporting && (
                <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <LoaderCircle className="animate-spin" size={14} />
                    Importing staff data...
                </div>
            )}

            {preview && !isImporting && (
                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-4">
                    <div className="text-sm font-semibold text-[var(--text-primary)]">
                        Preview: {preview.totalRows} records
                    </div>
                    <div className="mt-3 max-h-64 space-y-2 overflow-auto pr-1">
                        {preview.records.slice(0, 50).map((record, index) => (
                            <div key={index} className="flex items-center gap-3 rounded-[6px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2">
                                <span className="min-w-[80px] text-sm font-medium text-[var(--text-primary)]">{record.code}</span>
                                <span className="flex-1 text-sm text-[var(--text-primary)]">{record.name}</span>
                                {record.nickName && (
                                    <span className="text-xs text-[var(--text-secondary)]">{record.nickName}</span>
                                )}
                                {record.workEmail && (
                                    <span className="text-xs text-[var(--text-secondary)]">{record.workEmail}</span>
                                )}
                                {record.azureAdAccount && (
                                    <span className="text-xs text-[var(--text-secondary)]">{record.azureAdAccount}</span>
                                )}
                            </div>
                        ))}
                        {preview.totalRows > 50 && (
                            <div className="text-xs text-[var(--text-secondary)]">
                                ... and {preview.totalRows - 50} more
                            </div>
                        )}
                    </div>
                </div>
            )}

            {report && (
                <div className="space-y-3 rounded-[8px] border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
                    <div className="font-semibold">Import completed</div>
                    <div className="grid grid-cols-2 gap-2 text-xs">
                        <div>Total rows: {report.totalRows}</div>
                        <div>Inserted: {report.imported}</div>
                        <div>Updated: {report.updated}</div>
                        <div>Failed: {report.failed}</div>
                    </div>
                    {report.errors.length > 0 && (
                        <div className="max-h-44 overflow-auto rounded-[6px] border border-emerald-500/30 bg-black/20 p-2 text-xs">
                            {report.errors.slice(0, 12).map((item, index) => (
                                <div key={index}>
                                    {item.code}: {item.action}{item.error ? ` — ${item.error}` : ""}
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            )}
        </div>
    )
}
