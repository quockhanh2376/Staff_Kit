import { CheckCircle2, LoaderCircle, RefreshCw, Smartphone, XCircle } from "lucide-react"
import { QRCodeSVG } from "qrcode.react"
import type { SettingsState } from "../settings/useSettingsState"

type BorrowLanQrCardProps = {
  settings: SettingsState
  isAdmin: boolean
}

export function BorrowLanQrCard({ settings, isAdmin }: BorrowLanQrCardProps) {
  const borrowUrlPreview = settings.borrowLanUrlPreview
  const borrowUrl = borrowUrlPreview.startsWith("http://") ? borrowUrlPreview : ""

  return (
    <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
      <div className="text-sm font-semibold text-[var(--text-primary)]">LAN QR Flow</div>
      <div className="mt-4 flex flex-col gap-4 lg:flex-row lg:items-start">
        <div className="order-2 space-y-4 lg:order-1 lg:flex-1">
          <p className="text-xs leading-5 text-[var(--text-secondary)]">
            Keep Staff Kit open. The local LAN server is available while the app is running. If you
            changed the port, restart the app before testing the new QR URL.
          </p>

          {settings.lanServerAlive === null && (
            <div className="flex items-center gap-2 text-xs text-[var(--text-secondary)]">
              <LoaderCircle className="animate-spin" size={13} />
              Checking LAN server…
            </div>
          )}
          {settings.lanServerAlive === true && (
            <div className="flex items-center gap-2 text-xs text-emerald-400">
              <CheckCircle2 size={13} />
              LAN server active on port {settings.borrowLanSettings?.port}.
            </div>
          )}
          {settings.lanServerAlive === false && (
            <div className="space-y-1 rounded-[8px] border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-300">
              <div className="flex items-center gap-2 font-semibold">
                <XCircle size={13} />
                LAN server not responding on port {settings.borrowLanSettings?.port ?? "?"}.
              </div>
              <div>
                Port may be in use by a previous session, or Windows Firewall is blocking inbound
                connections. Restart the app — if it still fails, add a Firewall exception:{" "}
                <span className="font-medium text-amber-200">
                  Windows Security → Firewall → Allow an app through firewall → Staff Kit
                </span>
              </div>
            </div>
          )}

          <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
            <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
              Borrow URL
            </div>
            <div className="mt-2 break-all rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-sm text-[var(--text-primary)]">
              {borrowUrlPreview}
            </div>
            <div className="mt-3 flex items-start gap-2 rounded-[8px] border border-[var(--primary)]/30 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
              <Smartphone size={14} className="mt-0.5 shrink-0 text-[var(--primary)]" />
              <span>
                Phone and ST machine must stay on the same Wi-Fi/LAN. The employee form is
                intentionally narrow and public inside the local network only.
              </span>
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
            </div>
            {settings.borrowLanMessage && (
              <div className="mt-2 rounded-[8px] border border-[var(--primary)]/35 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
                {settings.borrowLanMessage}
              </div>
            )}
          </div>
        </div>

        <div className="order-1 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3 lg:order-2 lg:w-[280px] lg:shrink-0 lg:self-start">
          <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
            QR Preview
          </div>
          <div className="mt-3 flex items-center justify-center rounded-[10px] border border-[var(--border)] bg-white p-3">
            {borrowUrl ? (
              <QRCodeSVG value={borrowUrl} size={180} bgColor="#ffffff" fgColor="#111827" includeMargin />
            ) : (
              <div className="px-3 py-12 text-center text-xs text-slate-500">
                Borrow URL is empty.
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
