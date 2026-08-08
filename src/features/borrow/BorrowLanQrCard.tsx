import { CheckCircle2, LoaderCircle, Smartphone, Sparkles, XCircle } from "lucide-react"
import { QRCodeSVG } from "qrcode.react"
import type { SettingsState } from "../settings/useSettingsState"

type BorrowLanQrCardProps = {
  settings: SettingsState
  isAdmin: boolean
}

export function BorrowLanQrCard({ settings, isAdmin }: BorrowLanQrCardProps) {
  if (!isAdmin) return null

  const isReady =
    settings.lanAutoStartState === "ready" &&
    settings.lanServerStatus?.running === true &&
    Boolean(settings.borrowLanQrUrl)
  const host = settings.borrowLanSettings?.host || settings.lanServerStatus?.bindHost || "configured LAN host"
  const port = settings.borrowLanSettings?.port || settings.lanServerStatus?.port || "?"

  return (
    <div className="mt-4 max-w-6xl rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
      <div className="grid gap-3 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)_300px]">
        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
          <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
            LAN readiness
          </div>

          {(settings.lanAutoStartState === "starting" || settings.lanServerAlive === null || settings.lanAutoStartState === "idle") && (
            <div className="mt-3 flex items-center gap-2 text-xs text-[var(--text-secondary)]">
              <LoaderCircle className="animate-spin" size={13} />
              Starting LAN server…
            </div>
          )}
          {isReady && (
            <div className="mt-3 flex items-center gap-2 text-xs text-emerald-400">
              <CheckCircle2 size={13} />
              Ready at {host}:{port}
            </div>
          )}
          {settings.lanAutoStartState === "error" && (
            <div className="mt-3 space-y-2 text-xs text-rose-300">
              <div className="flex items-start gap-2">
                <XCircle className="mt-0.5 shrink-0" size={13} />
                <span>{settings.lanAutoStartError || "LAN server could not start."}</span>
              </div>
              <button
                className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)]"
                onClick={() => void settings.ensureBorrowLanReady()}
                type="button"
              >
                Retry
              </button>
            </div>
          )}

          <div className="mt-3 flex items-start gap-2 rounded-[8px] border border-[var(--primary)]/30 bg-[var(--primary)]/8 px-3 py-2 text-xs text-[var(--text-primary)]">
            <Smartphone size={14} className="mt-0.5 shrink-0 text-[var(--primary)]" />
            <span>Use the same Wi-Fi/LAN as the Staff Kit machine when scanning this QR.</span>
          </div>
        </div>

        <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
          <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
            Reachable address
          </div>
          <div className="mt-3 rounded-[8px] border border-[var(--border)] bg-[var(--surface)] px-3 py-2 text-sm font-semibold text-[var(--text-primary)]">
            {host}:{port}
          </div>
          <p className="mt-2 text-xs text-[var(--text-secondary)]">
            LAN configuration and lifecycle controls are available in Settings.
          </p>
        </div>

        <div className="relative overflow-hidden rounded-[24px] border border-white/8 bg-[#101722] p-3 shadow-[0_18px_50px_rgba(2,8,23,0.46)] xl:self-start">
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_18%_18%,rgba(74,222,128,0.28),transparent_26%),radial-gradient(circle_at_82%_18%,rgba(250,204,21,0.26),transparent_28%),radial-gradient(circle_at_20%_82%,rgba(236,72,153,0.24),transparent_28%),radial-gradient(circle_at_80%_82%,rgba(96,165,250,0.24),transparent_28%)]" />
          <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(180deg,rgba(255,255,255,0.05),transparent_22%,transparent_78%,rgba(255,255,255,0.03))]" />
          <div className="relative flex items-center justify-between gap-2">
            <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-white/70">QR code</div>
            <Sparkles size={13} className="text-white/40" />
          </div>

          <div className="relative mt-3 rounded-[28px] bg-[linear-gradient(135deg,#46ff80_0%,#80ffea_16%,#60a5fa_34%,#c084fc_52%,#f472b6_70%,#facc15_88%,#46ff80_100%)] p-[3px] shadow-[0_0_22px_rgba(96,165,250,0.26),0_0_34px_rgba(250,204,21,0.12)]">
            <div className="rounded-[26px] bg-[#152032] p-[3px]">
              <div className="rounded-[24px] bg-[linear-gradient(135deg,#5fd0ff_0%,#c084fc_34%,#f472b6_66%,#5df980_100%)] p-[3px] shadow-[0_0_16px_rgba(192,132,252,0.24)]">
                <div className="rounded-[22px] bg-white p-[6px] shadow-[inset_0_1px_0_rgba(255,255,255,0.65)]">
                  {isReady ? (
                    <div className="flex items-center justify-center">
                      <QRCodeSVG value={settings.borrowLanQrUrl} size={220} bgColor="#ffffff" fgColor="#111827" />
                    </div>
                  ) : (
                    <div className="px-3 py-12 text-center text-xs text-slate-500">
                      QR appears when the LAN server is ready.
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
