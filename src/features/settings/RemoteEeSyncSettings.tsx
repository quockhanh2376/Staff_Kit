import { useEffect, useState } from "react"
import { Check, ChevronDown, ChevronUp, Database, LoaderCircle, Trash2 } from "lucide-react"
import { staffApi } from "../../services/staff-api"
import type { MssqlConnectionTestResult, RemoteEeSyncSettings as SettingsDto } from "../../types/staff"
import { useIdleCollapse } from "./useIdleCollapse"

const inputClass = "w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-1.5 text-xs text-[var(--text-primary)] outline-none focus:border-[var(--primary)]/60"

export function RemoteEeSyncSettings({
    isSuperAdmin = false,
    initialExpanded = false,
}: {
    isSuperAdmin?: boolean
    initialExpanded?: boolean
}) {
    void isSuperAdmin
    const card = useIdleCollapse(60000, initialExpanded)
    const [settings, setSettings] = useState<SettingsDto | null>(null)
    const [password, setPassword] = useState("")
    const [result, setResult] = useState<MssqlConnectionTestResult | null>(null)
    const [busy, setBusy] = useState<"load" | "save" | "test" | "remove" | null>("load")
    const [message, setMessage] = useState<string | null>(null)

    useEffect(() => {
        let cancelled = false
        staffApi.getRemoteEeSyncSettings()
            .then((saved) => { if (!cancelled) setSettings(saved) })
            .catch(() => { if (!cancelled) setMessage("Could not load Config Sync settings.") })
            .finally(() => { if (!cancelled) setBusy(null) })
        return () => { cancelled = true }
    }, [])

    const update = <K extends keyof SettingsDto>(key: K, value: SettingsDto[K]) => {
        setSettings((current) => current ? { ...current, [key]: value } : current)
        setResult(null)
        setMessage(null)
    }

    const handleSave = async () => {
        if (!settings) return
        setBusy("save")
        setMessage(null)
        try {
            const server = settings.server.trim()
            const username = settings.username.trim()
            const database = settings.database.trim() || "AssetManagement"
            const port = settings.port || 1433
            const wasAccountRotation = username !== settings.approvedUsername

            await staffApi.saveRemoteEeSyncSettings({
                server,
                database,
                port,
                authType: "sql_server",
                username,
                approvedUsername: username,
            }, password)

            setPassword("")
            setSettings({
                ...settings,
                server,
                database,
                port,
                username,
                approvedUsername: username,
                hasSavedPassword: wasAccountRotation ? Boolean(password) : (password ? true : settings.hasSavedPassword),
            })
            setMessage(
                wasAccountRotation
                    ? "Approved SQL login updated and settings saved. Run Test Connection to verify read-only access."
                    : "Settings saved. Any entered password is stored securely in Windows Credential Manager."
            )
        } catch (error) {
            setMessage(
                typeof error === "string"
                    ? error
                    : "Could not save settings or store the password securely. No password was saved in app settings."
            )
        } finally {
            setBusy(null)
        }
    }

    const handleTest = async () => {
        setBusy("test")
        setResult(null)
        setMessage(null)
        try { setResult(await staffApi.testRemoteEeSyncConnection()) }
        catch { setResult({ status: "FAIL", reason: "Connection test could not complete; no database changes were made.", rowCount: null }) }
        finally { setBusy(null) }
    }

    const handleRemovePassword = async () => {
        setBusy("remove")
        setMessage(null)
        try {
            await staffApi.removeRemoteEeSyncPassword()
            setSettings((current) => current ? { ...current, hasSavedPassword: false } : current)
            setMessage("Saved password removed from Windows Credential Manager.")
        } catch { setMessage("Could not remove the saved password securely.") }
        finally { setBusy(null) }
    }

    return (
        <section
            aria-label="Config Sync"
            {...card.bindActivityHandlers}
            className="self-start overflow-hidden rounded-[16px] border border-[var(--border)] bg-[var(--surface)] text-sm text-[var(--text-secondary)] shadow-[0_18px_42px_rgba(0,0,0,0.22)]"
        >
            <div
                className={`cursor-pointer transition-colors hover:bg-[var(--surface-hover)] min-h-[68px] bg-[var(--surface-hover)] px-4 py-4 ${card.isExpanded ? "border-b border-[var(--border)]" : ""}`}
                onClick={() => card.setExpanded((current) => !current)}
                role="button"
                tabIndex={0}
                aria-expanded={card.isExpanded}
                onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault()
                        card.setExpanded((current) => !current)
                    }
                }}
            >
                <div className="flex items-center justify-between gap-3">
                    <div className="flex min-h-[36px] min-w-0 items-center gap-2 text-xl font-semibold text-[var(--text-primary)]">
                        <Database size={20} className="shrink-0 text-emerald-400" />
                        <div>Config Sync</div>
                        <span
                            aria-label={card.isExpanded ? "Collapse Config Sync" : "Expand Config Sync"}
                            className="ml-1 inline-flex items-center justify-center rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)] p-1 text-[var(--text-secondary)]"
                        >
                            {card.isExpanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                        </span>
                    </div>
                </div>
            </div>

            <div
                className={`origin-top overflow-hidden transition-[max-height,opacity] duration-300 ease-out ${card.isExpanded ? "max-h-[1400px] opacity-100" : "max-h-0 opacity-0"}`}
            >
                <div className="space-y-4 p-4">
                    {!settings ? (
                        <div className="p-2 text-xs text-[var(--text-secondary)]">
                            {busy === "load" ? "Loading Config Sync settings…" : message}
                        </div>
                    ) : (
                        <>
                            {settings.environmentOverride && (
                                <p className="rounded border border-amber-500/30 bg-amber-500/10 p-2 text-xs text-amber-200">
                                    A STAFFKIT_MSSQL_* environment override is active. Test Connection and EE List Update will use that complete environment profile.
                                </p>
                            )}

                            <div className="flex flex-wrap items-end gap-3">
                                <label className="min-w-[150px] flex-1 text-xs text-[var(--text-secondary)]">
                                    Server
                                    <input
                                        aria-label="Server"
                                        className={inputClass}
                                        value={settings.server}
                                        onChange={(event) => update("server", event.target.value)}
                                        placeholder="e.g. 10.184.0.19"
                                    />
                                </label>
                                <label className="min-w-[130px] flex-1 text-xs text-[var(--text-secondary)]">
                                    SQL Username
                                    <input
                                        aria-label="SQL Username"
                                        className={inputClass}
                                        value={settings.username}
                                        onChange={(event) => update("username", event.target.value)}
                                        placeholder="e.g. user_reader"
                                    />
                                </label>
                                <label className="min-w-[150px] flex-1 text-xs text-[var(--text-secondary)]">
                                    Password
                                    <input
                                        aria-label="Password"
                                        className={inputClass}
                                        type="password"
                                        autoComplete="new-password"
                                        placeholder={settings.hasSavedPassword ? "Saved securely in Windows Credential Manager" : "Enter SQL password"}
                                        value={password}
                                        onChange={(event) => { setPassword(event.target.value); setResult(null) }}
                                    />
                                </label>
                                <div className="flex items-center gap-2 pb-0.5">
                                    <button
                                        type="button"
                                        aria-label="Save Settings"
                                        className="inline-flex items-center gap-1.5 rounded-[8px] bg-[var(--primary)] px-3 py-1.5 text-xs font-semibold text-white hover:opacity-90 disabled:opacity-50"
                                        disabled={busy !== null}
                                        onClick={() => void handleSave()}
                                    >
                                        {busy === "save" ? <LoaderCircle size={13} className="animate-spin" /> : <Check size={14} className="stroke-[3]" />} Save
                                    </button>
                                    <button
                                        type="button"
                                        className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-medium hover:bg-[var(--surface-hover)] disabled:opacity-50"
                                        disabled={busy !== null}
                                        onClick={() => void handleTest()}
                                    >
                                        {busy === "test" ? "Testing…" : "Test Connection"}
                                    </button>
                                </div>
                            </div>

                            {settings.hasSavedPassword && (
                                <div className="flex items-center justify-between text-xs text-[var(--text-secondary)]">
                                    <span>Password saved securely in Windows Credential Manager.</span>
                                    <button
                                        type="button"
                                        className="inline-flex items-center gap-1 text-red-300 hover:text-red-200 disabled:opacity-50"
                                        disabled={busy !== null}
                                        onClick={() => void handleRemovePassword()}
                                    >
                                        <Trash2 size={12} /> Remove password
                                    </button>
                                </div>
                            )}

                            <details className="text-xs text-[var(--text-secondary)]">
                                <summary className="cursor-pointer font-medium hover:text-[var(--text-primary)]">
                                    Advanced Settings
                                </summary>
                                <div className="mt-2 grid max-w-md gap-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface)]/50 p-3 sm:grid-cols-2">
                                    <label className="text-xs text-[var(--text-secondary)]">
                                        Database
                                        <input
                                            aria-label="Database"
                                            className={inputClass}
                                            value={settings.database}
                                            placeholder="AssetManagement"
                                            onChange={(event) => update("database", event.target.value)}
                                        />
                                    </label>
                                    <label className="text-xs text-[var(--text-secondary)]">
                                        Port
                                        <input
                                            aria-label="Port"
                                            className={inputClass}
                                            type="number"
                                            min={1}
                                            max={65535}
                                            value={settings.port || 1433}
                                            placeholder="1433"
                                            onChange={(event) => update("port", Number(event.target.value) || 1433)}
                                        />
                                    </label>
                                </div>
                            </details>

                            {message && <p role="status" className="text-xs text-[var(--text-secondary)]">{message}</p>}
                            {result && (
                                <div
                                    role="status"
                                    className={`rounded border p-3 text-xs ${
                                        result.status === "PASS"
                                            ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-200"
                                            : "border-red-500/40 bg-red-500/10 text-red-200"
                                    }`}
                                >
                                    <strong>{result.status}</strong>: {result.reason}
                                    {result.rowCount !== null && <span className="ml-2">{result.rowCount} source rows</span>}
                                </div>
                            )}
                        </>
                    )}
                </div>
            </div>
        </section>
    )
}

export { RemoteEeSyncSettings as ConfigSyncSettings }
