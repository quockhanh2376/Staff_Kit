import { useEffect, useState } from "react"
import { Database, LoaderCircle, Save, Trash2 } from "lucide-react"
import { staffApi } from "../../services/staff-api"
import type { MssqlAuthType, MssqlConnectionTestResult, RemoteEeSyncSettings as SettingsDto } from "../../types/staff"

const inputClass = "w-full rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--primary)]/60"

export function RemoteEeSyncSettings() {
    const [settings, setSettings] = useState<SettingsDto | null>(null)
    const [password, setPassword] = useState("")
    const [result, setResult] = useState<MssqlConnectionTestResult | null>(null)
    const [busy, setBusy] = useState<"load" | "save" | "test" | "remove" | null>("load")
    const [message, setMessage] = useState<string | null>(null)

    useEffect(() => {
        let cancelled = false
        staffApi.getRemoteEeSyncSettings()
            .then((saved) => { if (!cancelled) setSettings(saved) })
            .catch(() => { if (!cancelled) setMessage("Could not load Remote EE Sync settings.") })
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
            await staffApi.saveRemoteEeSyncSettings({
                server: settings.server,
                database: settings.database,
                port: settings.port,
                authType: settings.authType,
                username: settings.username,
            }, password)
            setPassword("")
            setSettings({ ...settings, hasSavedPassword: settings.authType === "sql_server" && (password ? true : settings.hasSavedPassword) })
            setMessage("Settings saved. Any entered password is stored securely in Windows Credential Manager.")
        } catch {
            setMessage("Could not save settings or store the password securely. No password was saved in app settings.")
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

    if (!settings) return <div className="p-3 text-sm text-[var(--text-secondary)]">{busy === "load" ? "Loading Remote EE Sync settings…" : message}</div>

    return (
        <section aria-label="Remote EE Sync" className="rounded-[12px] border border-[var(--border)] bg-[var(--surface-hover)]/30 p-4">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--text-primary)]">
                <Database size={16} className="text-[var(--primary)]" /> Remote EE Sync
            </div>
            <p className="mt-1 text-xs text-[var(--text-secondary)]">Uses the approved Query B and blocks sync unless source permissions are verified read-only.</p>
            <p className="mt-1 text-xs text-[var(--text-secondary)]">Save settings before testing. Test Connection uses the saved profile, unless an environment override is active.</p>
            {settings.environmentOverride && <p className="mt-2 rounded border border-amber-500/30 bg-amber-500/10 p-2 text-xs text-amber-200">A STAFFKIT_MSSQL_* environment override is active. Test Connection and EE List Update will use that complete environment profile.</p>}
            <div className="mt-3 grid gap-3 sm:grid-cols-2">
                <label className="text-xs text-[var(--text-secondary)]">Server<input aria-label="Server" className={inputClass} value={settings.server} onChange={(event) => update("server", event.target.value)} /></label>
                <label className="text-xs text-[var(--text-secondary)]">Database<input aria-label="Database" className={inputClass} value={settings.database} onChange={(event) => update("database", event.target.value)} /></label>
                <label className="text-xs text-[var(--text-secondary)]">Port<input aria-label="Port" className={inputClass} type="number" min={1} max={65535} value={settings.port} onChange={(event) => update("port", Number(event.target.value))} /></label>
                <label className="text-xs text-[var(--text-secondary)]">Authentication type<select aria-label="Authentication type" className={inputClass} value={settings.authType} onChange={(event) => update("authType", event.target.value as MssqlAuthType)}><option value="sql_server">SQL Server</option><option value="windows_integrated">Windows Integrated</option></select></label>
                {settings.authType === "sql_server" && <>
                    <label className="text-xs text-[var(--text-secondary)]">Username<input aria-label="Username" className={inputClass} autoComplete="off" value={settings.username} onChange={(event) => update("username", event.target.value)} /></label>
                    <label className="text-xs text-[var(--text-secondary)]">Password<input aria-label="Password" className={inputClass} type="password" autoComplete="new-password" placeholder={settings.hasSavedPassword ? "Saved securely in Windows Credential Manager" : "Enter SQL password"} value={password} onChange={(event) => { setPassword(event.target.value); setResult(null) }} /></label>
                </>}
            </div>
            {settings.hasSavedPassword && <div className="mt-2 flex items-center justify-between text-xs text-[var(--text-secondary)]"><span>Password saved securely in Windows Credential Manager.</span><button type="button" className="inline-flex items-center gap-1 text-red-300" disabled={busy !== null} onClick={() => void handleRemovePassword()}><Trash2 size={13} /> Remove password</button></div>}
            <div className="mt-3 flex flex-wrap items-center gap-2">
                <button type="button" className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-3 py-2 text-sm font-semibold text-white disabled:opacity-50" disabled={busy !== null} onClick={() => void handleSave()}>{busy === "save" ? <LoaderCircle size={14} className="animate-spin" /> : <Save size={14} />}Save Settings</button>
                <button type="button" className="rounded-[8px] border border-[var(--border)] px-3 py-2 text-sm font-medium disabled:opacity-50" disabled={busy !== null} onClick={() => void handleTest()}>{busy === "test" ? "Testing…" : "Test Connection"}</button>
            </div>
            {message && <p role="status" className="mt-2 text-xs text-[var(--text-secondary)]">{message}</p>}
            {result && <div role="status" className={`mt-3 rounded border p-3 text-xs ${result.status === "PASS" ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-200" : "border-red-500/40 bg-red-500/10 text-red-200"}`}><strong>{result.status}</strong>: {result.reason}{result.rowCount !== null && <span className="ml-2">{result.rowCount} source rows</span>}</div>}
        </section>
    )
}
