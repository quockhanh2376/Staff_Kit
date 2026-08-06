import { CheckCircle2, LoaderCircle, RefreshCw, Smartphone, Sparkles, XCircle } from "lucide-react"
import { QRCodeSVG } from "qrcode.react"
import type { SettingsState } from "../settings/useSettingsState"

type BorrowLanQrCardProps = {
  settings: SettingsState
  isAdmin: boolean
}

export function BorrowLanQrCard({ settings, isAdmin }: BorrowLanQrCardProps) {
  if (!isAdmin) return null

  const borrowUrl = settings.borrowLanQrUrl

  return (
    <div className="mt-4 max-w-6xl rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
      <div className="grid gap-3 xl:grid-cols-[minmax(0,0.58fr)_minmax(0,1.32fr)_300px]">
        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
          <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
            LAN Status
          </div>

          {settings.lanServerAlive === null && (
            <div className="mt-3 flex items-center gap-2 text-xs text-[var(--text-secondary)]">
              <LoaderCircle className="animate-spin" size={13} />
              Checking LAN server…
            </div>
          )}
          {settings.lanServerStatus?.running && (
            <div className="mt-3 flex items-center gap-2 text-xs text-emerald-400">
              <CheckCircle2 size={13} />
              LAN server active on port {settings.borrowLanSettings?.port}.
            </div>
          )}
          {settings.lanServerStatus && !settings.lanServerStatus.running && (
            <div className="mt-3 flex items-center gap-2 text-xs text-amber-300">
              <XCircle size={13} />
              LAN server not responding on port {settings.borrowLanSettings?.port ?? "?"}.
            </div>
          )}

          <div className="mt-3 flex items-start gap-2 rounded-[8px] border border-[var(--primary)]/30 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
            <Smartphone size={14} className="mt-0.5 shrink-0 text-[var(--primary)]" />
            <span>Use the same Wi-Fi/LAN as the Staff Kit machine when scanning this QR.</span>
          </div>
        </div>

        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
          <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
            Borrow LAN Settings
          </div>
          <div className="mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr),120px]">
            <input
              className="form-input text-xs"
              value={settings.borrowLanHostInput}
              onChange={(event) => settings.handleBorrowLanHostInputChange(event.target.value)}
              placeholder="192.168.1.25 or OFFICE-PC"
              disabled={!isAdmin || settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
            />
            <input
              className="form-input text-xs"
              value={settings.borrowLanPortInput}
              onChange={(event) => settings.setBorrowLanPortInput(event.target.value)}
              placeholder="8787"
              disabled={!isAdmin || settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
            />
          </div>
          {settings.borrowLanDetectionNote && (
            <div className="mt-2 text-[11px] text-[var(--primary)]">
              {settings.borrowLanDetectionNote}
            </div>
          )}
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
              onClick={() => settings.handleRefreshBorrowLanHost()}
              type="button"
              disabled={!isAdmin || settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
            >
              <RefreshCw
                className={settings.isDetectingBorrowLanHost ? "animate-spin" : undefined}
                size={14}
              />
              {settings.isDetectingBorrowLanHost ? "Detecting..." : "Refresh LAN IP"}
            </button>
            <button
              className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
              onClick={() => void settings.handleSaveBorrowLanSettings()}
              type="button"
              disabled={!isAdmin || settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
            >
              {settings.isSavingBorrowLanSettings ? "Saving..." : "Save Borrow LAN Settings"}
            </button>
            <button
              className="rounded-[8px] border border-[var(--primary)]/45 px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--primary)]/10 disabled:opacity-50"
              onClick={() => void settings.handleIssueBorrowLanToken()}
              type="button"
              disabled={!isAdmin || settings.isManagingLanToken || settings.isSavingBorrowLanSettings || settings.lanServerStatus?.running !== true}
            >
              {settings.isManagingLanToken
                ? "Working..."
                : settings.lanTokenReady
                  ? "Regenerate QR Token"
                : "Generate QR Token"}
            </button>
            <button
              className="rounded-[8px] border border-emerald-400/45 px-3 py-1.5 text-xs font-semibold text-emerald-200 transition hover:bg-emerald-400/10 disabled:opacity-50"
              onClick={() => void settings.handleStartBorrowLanServer()}
              type="button"
              disabled={!isAdmin || settings.isManagingLanToken || settings.lanServerStatus?.running === true}
            >
              Start LAN Server
            </button>
            <button
              className="rounded-[8px] border border-amber-400/45 px-3 py-1.5 text-xs font-semibold text-amber-200 transition hover:bg-amber-400/10 disabled:opacity-50"
              onClick={() => void settings.handleStopBorrowLanServer()}
              type="button"
              disabled={!isAdmin || settings.isManagingLanToken || settings.lanServerStatus?.running !== true}
            >
              Stop LAN Server
            </button>
            <button
              className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
              onClick={() => void settings.refreshBorrowLanStatus()}
              type="button"
              disabled={!isAdmin || settings.isManagingLanToken}
            >
              Refresh Status
            </button>
            {settings.lanTokenReady && (
              <button
                className="rounded-[8px] border border-rose-400/40 px-3 py-1.5 text-xs font-semibold text-rose-200 transition hover:bg-rose-400/10 disabled:opacity-50"
                onClick={() => void settings.handleRevokeBorrowLanToken()}
                type="button"
                disabled={!isAdmin || settings.isManagingLanToken}
              >
                Revoke QR Token
              </button>
            )}
          </div>
          <label className="mt-3 flex items-center gap-2 text-xs text-[var(--text-primary)]">
            <input
              type="checkbox"
              checked={settings.borrowLanSettings?.enabled ?? false}
              onChange={(event) => settings.handleBorrowLanEnabledChange(event.target.checked)}
              disabled={!isAdmin || settings.isSavingBorrowLanSettings || settings.isManagingLanToken}
            />
            Enable LAN server on app startup
          </label>
          {settings.borrowLanMessage && (
            <div className="mt-2 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
              {settings.borrowLanMessage}
            </div>
          )}
        </div>

        <div className="relative overflow-hidden rounded-[24px] border border-white/8 bg-[#101722] p-3 shadow-[0_18px_50px_rgba(2,8,23,0.46)] xl:self-start">
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_18%_18%,rgba(74,222,128,0.28),transparent_26%),radial-gradient(circle_at_82%_18%,rgba(250,204,21,0.26),transparent_28%),radial-gradient(circle_at_20%_82%,rgba(236,72,153,0.24),transparent_28%),radial-gradient(circle_at_80%_82%,rgba(96,165,250,0.24),transparent_28%)]" />
          <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(180deg,rgba(255,255,255,0.05),transparent_22%,transparent_78%,rgba(255,255,255,0.03))]" />
          <div className="pointer-events-none absolute left-5 top-16 h-1.5 w-1.5 rounded-full bg-cyan-300/80 shadow-[0_0_14px_rgba(103,232,249,0.95)]" />
          <div className="pointer-events-none absolute right-6 top-12 h-1 w-1 rounded-full bg-amber-300/85 shadow-[0_0_14px_rgba(252,211,77,0.95)]" />
          <div className="pointer-events-none absolute bottom-14 left-6 h-1 w-1 rounded-full bg-fuchsia-300/80 shadow-[0_0_14px_rgba(240,171,252,0.95)]" />
          <div className="pointer-events-none absolute bottom-10 right-7 h-1.5 w-1.5 rounded-full bg-emerald-300/80 shadow-[0_0_14px_rgba(110,231,183,0.95)]" />
          <div className="pointer-events-none absolute left-10 top-20 h-px w-12 rotate-[8deg] bg-gradient-to-r from-transparent via-cyan-200/80 to-transparent opacity-70" />
          <div className="pointer-events-none absolute bottom-16 right-8 h-px w-10 -rotate-[14deg] bg-gradient-to-r from-transparent via-fuchsia-200/75 to-transparent opacity-70" />

          <div className="relative flex items-center justify-between gap-2">
            <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-white/70">
              QR Preview
            </div>
            <Sparkles size={13} className="text-white/40" />
          </div>

          <div className="relative mt-3 rounded-[28px] bg-[linear-gradient(135deg,#46ff80_0%,#80ffea_16%,#60a5fa_34%,#c084fc_52%,#f472b6_70%,#facc15_88%,#46ff80_100%)] p-[3px] shadow-[0_0_22px_rgba(96,165,250,0.26),0_0_34px_rgba(250,204,21,0.12)]">
            <div className="rounded-[26px] bg-[#152032] p-[3px]">
              <div className="rounded-[24px] bg-[linear-gradient(135deg,#5fd0ff_0%,#c084fc_34%,#f472b6_66%,#5df980_100%)] p-[3px] shadow-[0_0_16px_rgba(192,132,252,0.24)]">
                <div className="rounded-[22px] bg-white p-[6px] shadow-[inset_0_1px_0_rgba(255,255,255,0.65)]">
                  {borrowUrl ? (
                    <div className="flex items-center justify-center">
                      <QRCodeSVG value={borrowUrl} size={220} bgColor="#ffffff" fgColor="#111827" />
                    </div>
                  ) : (
                    <div className="px-3 py-12 text-center text-xs text-slate-500">
                      Generate a QR token to activate the LAN flow.
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>

          <div className="pointer-events-none absolute bottom-3 right-3 text-white/34">
            <Sparkles size={15} />
          </div>
        </div>
      </div>
    </div>
  )
}
