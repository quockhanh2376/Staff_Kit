import { CheckCircle2, Crosshair, LoaderCircle, RefreshCw, Save, Sparkles, XCircle } from "lucide-react"
import { useRef, useState } from "react"
import { QRCodeSVG } from "qrcode.react"
import type { SettingsState } from "../settings/useSettingsState"
import { BorrowPolicyCard } from "./BorrowPolicyCard"

type BorrowLanQrCardProps = {
  settings: SettingsState
  isAdmin: boolean
  isQueueRefreshing?: boolean
  onRefreshQueue?: () => void
  onStartLan?: () => void
  onStopLan?: () => void
  policyEnglish?: string
  policyVietnamese?: string
  savedPolicyEnglish?: string
  savedPolicyVietnamese?: string
  isLoadingPolicy?: boolean
  isSavingPolicy?: boolean
  policyMessage?: string
  onPolicyEnglishChange?: (value: string) => void
  onPolicyVietnameseChange?: (value: string) => void
  onSavePolicy?: () => void
}

export function BorrowLanQrCard({
  settings,
  isAdmin,
  isQueueRefreshing = false,
  onRefreshQueue,
  onStartLan,
  onStopLan,
  policyEnglish = "",
  policyVietnamese = "",
  savedPolicyEnglish = "",
  savedPolicyVietnamese = "",
  isLoadingPolicy = false,
  isSavingPolicy = false,
  policyMessage = "",
  onPolicyEnglishChange,
  onPolicyVietnameseChange,
  onSavePolicy,
}: BorrowLanQrCardProps) {
  const [isTogglePending, setTogglePending] = useState(false)
  const togglePendingRef = useRef(false)
  if (!isAdmin) return null

  const isReady =
    (settings.lanServerStatus?.running ?? settings.lanServerAlive === true) &&
    (settings.lanServerStatus?.tokenReady ?? settings.lanTokenReady) &&
    !settings.borrowLanRestartRequired &&
    Boolean(settings.borrowLanQrUrl)
  const host = settings.borrowLanSettings?.host || settings.lanServerStatus?.bindHost || "configured LAN host"
  const port = settings.borrowLanSettings?.port || settings.lanServerStatus?.port || "?"
  const isLanRunning = settings.lanServerStatus?.running ?? settings.lanServerAlive === true
  const isLanStarting = settings.lanAutoStartState === "starting" || (settings.isManagingLanToken && !isLanRunning) || (isTogglePending && !isLanRunning)
  const isLanStopping = (settings.isManagingLanToken && isLanRunning) || (isTogglePending && isLanRunning)
  const isLanTransitioning = isLanStarting || isLanStopping

  const handleLanCardToggle = async () => {
    if (isLanTransitioning || isTogglePending || togglePendingRef.current) return
    const action = isLanRunning ? onStopLan : onStartLan
    if (!action) return
    togglePendingRef.current = true
    setTogglePending(true)
    try {
      await action()
    } finally {
      togglePendingRef.current = false
      setTogglePending(false)
    }
  }

  return (
    <div className="mt-4 max-w-6xl rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4">
      <div data-testid="lan-top-cards" className="grid items-start gap-3 xl:grid-cols-[minmax(255px,275px)_minmax(300px,330px)_minmax(460px,1fr)]">
        <div data-testid="qr-code-card" className="relative order-first flex aspect-square min-w-0 flex-col rounded-[24px] border border-white/8 bg-[#101722] p-3 shadow-[0_18px_50px_rgba(2,8,23,0.46)]">
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_18%_18%,rgba(74,222,128,0.28),transparent_26%),radial-gradient(circle_at_82%_18%,rgba(250,204,21,0.26),transparent_28%),radial-gradient(circle_at_20%_82%,rgba(236,72,153,0.24),transparent_28%),radial-gradient(circle_at_80%_82%,rgba(96,165,250,0.24),transparent_28%)]" />
          <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(180deg,rgba(255,255,255,0.05),transparent_22%,transparent_78%,rgba(255,255,255,0.03))]" />
          <div className="relative flex items-center justify-between gap-2">
            <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-white/70">QR Code</div>
            <div className="flex items-center gap-1.5">
              {onRefreshQueue && (
                <button
                  aria-label="Refresh Queue"
                  className="inline-flex h-7 w-7 items-center justify-center rounded-[7px] border border-white/20 text-white/80 transition hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-50"
                  disabled={isQueueRefreshing}
                  onClick={(event) => {
                    event.stopPropagation()
                    onRefreshQueue()
                  }}
                  title="Refresh Queue"
                  type="button"
                >
                  <RefreshCw aria-hidden="true" className={isQueueRefreshing ? "animate-spin" : ""} size={13} />
                </button>
              )}
              <Sparkles size={13} className="text-white/40" />
            </div>
          </div>

          <div
            aria-disabled={isLanTransitioning ? "true" : undefined}
            aria-label={isLanStarting ? "Starting LAN" : isLanStopping ? "Stopping LAN" : isLanRunning ? "Stop LAN" : "Start Borrow/Return"}
            className={`relative mt-3 flex min-h-0 flex-1 items-center justify-center rounded-[28px] p-[3px] transition ${isReady ? "cursor-pointer bg-[linear-gradient(135deg,#46ff80_0%,#80ffea_16%,#60a5fa_34%,#c084fc_52%,#f472b6_70%,#facc15_88%,#46ff80_100%)] shadow-[0_0_22px_rgba(96,165,250,0.26),0_0_34px_rgba(250,204,21,0.12)]" : "cursor-pointer bg-slate-500/40 grayscale opacity-60"}`}
            onClick={() => void handleLanCardToggle()}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault()
                void handleLanCardToggle()
              }
            }}
            role="button"
            tabIndex={isLanTransitioning ? -1 : 0}
            title={isLanStarting ? "Starting LAN" : isLanStopping ? "Stopping LAN" : isLanRunning ? "Stop LAN" : "Start Borrow/Return"}
          >
            <div className="flex aspect-square max-h-full max-w-full items-center justify-center rounded-[26px] bg-[#152032] p-[3px]">
              <div className="flex aspect-square max-h-full max-w-full items-center justify-center rounded-[24px] bg-[linear-gradient(135deg,#5fd0ff_0%,#c084fc_34%,#f472b6_66%,#5df980_100%)] p-[3px] shadow-[0_0_16px_rgba(192,132,252,0.24)]">
                <div data-testid="qr-code-surface" className="flex aspect-square h-full max-h-full w-auto max-w-full items-center justify-center rounded-[22px] bg-white p-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.65)]">
                  {isReady ? (
                    <div className="flex aspect-square w-full items-center justify-center">
                      <QRCodeSVG className="h-full w-full" value={settings.borrowLanQrUrl} size={220} bgColor="#ffffff" fgColor="#111827" />
                    </div>
                  ) : (
                    <div className="px-3 py-12 text-center text-xs text-slate-500">
                      {isLanStarting
                        ? "Starting…"
                        : isLanStopping
                          ? "Stopping…"
                          : settings.borrowLanRestartRequired
                            ? "Restart Borrow/Return to refresh QR"
                            : "Start Borrow/Return"}
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div data-testid="lan-address-card" className="min-w-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
          <div data-testid="lan-address-actions" className="flex justify-end">
            <button
              aria-label="Detect LAN IP"
              className="ml-auto inline-flex h-8 w-8 items-center justify-center rounded-[8px] border border-[var(--border)] text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:cursor-not-allowed disabled:opacity-50"
              onClick={() => settings.handleRefreshBorrowLanHost()}
              disabled={settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
              title="Detect LAN IP"
              type="button"
            >
              <Crosshair aria-hidden="true" className={settings.isDetectingBorrowLanHost ? "animate-pulse" : ""} size={14} />
            </button>
          </div>

          {isLanStarting && (
            <div className="mt-2 flex items-center gap-2 text-xs text-[var(--text-secondary)]">
              <LoaderCircle className="animate-spin" size={13} />
              Starting…
            </div>
          )}
          {isLanStopping && (
            <div className="mt-2 flex items-center gap-2 text-xs text-[var(--text-secondary)]">
              <LoaderCircle className="animate-spin" size={13} />
              Stopping…
            </div>
          )}
          {isReady && (
            <div className="mt-2 flex items-center gap-2 text-xs text-emerald-400">
              <CheckCircle2 size={13} />
              Ready at {host}:{port}
            </div>
          )}
          {settings.borrowLanRestartRequired && !isLanStarting && !isLanStopping && (
            <div className="mt-2 space-y-2 text-xs text-amber-300">
              <div className="flex items-start gap-2">
                <XCircle className="mt-0.5 shrink-0" size={13} />
                <span>LAN settings saved. Restart Borrow / Return to apply.</span>
              </div>
              <button
                aria-label="Restart Borrow / Return"
                className="rounded-[8px] border border-amber-400/60 px-3 py-1.5 font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:cursor-not-allowed disabled:opacity-50"
                disabled={settings.isManagingLanToken}
                onClick={() => void settings.handleRestartBorrowLanServer()}
                type="button"
              >
                Restart Borrow / Return
              </button>
            </div>
          )}
          {!isLanStarting && !isLanStopping && !isReady && !settings.borrowLanRestartRequired && (
            <div className="mt-2 flex items-center gap-2 text-xs text-[var(--text-secondary)]">
              LAN server inactive.
            </div>
          )}
          {settings.lanAutoStartState === "error" && (
            <div className="mt-2 space-y-2 text-xs text-rose-300">
              <div className="flex items-start gap-2">
                <XCircle className="mt-0.5 shrink-0" size={13} />
                <span>{settings.lanAutoStartError || "LAN server could not start."}</span>
              </div>
              <button
                className="rounded-[8px] border border-[var(--border)] px-3 py-1.5 font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)]"
                onClick={() => void onStartLan?.()}
                type="button"
              >
                Retry
              </button>
            </div>
          )}

          <div data-testid="lan-config-row" className="mt-2 grid items-end gap-2 sm:grid-cols-[minmax(0,1fr)_110px_auto]">
            <div>
              <label className="mb-1 block text-[9px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]" htmlFor="borrow-lan-host">
                LAN Host / IP
              </label>
              <input
                id="borrow-lan-host"
                className="w-full rounded-[8px] border border-[var(--border)] bg-[var(--bg)] px-2.5 py-1.5 text-[11px] text-[var(--text-primary)] outline-none focus:border-emerald-500/50"
                value={settings.borrowLanHostInput}
                onChange={(event) => settings.handleBorrowLanHostInputChange(event.target.value)}
                disabled={settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
                placeholder="192.168.1.25"
                title="Use the same Wi-Fi/LAN as the Staff Kit machine when scanning this QR."
              />
            </div>
            <div>
              <label className="mb-1 block text-[9px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]" htmlFor="borrow-lan-port">
                Port
              </label>
              <input
                id="borrow-lan-port"
                className="w-full rounded-[8px] border border-[var(--border)] bg-[var(--bg)] px-2.5 py-1.5 text-[11px] text-[var(--text-primary)] outline-none focus:border-emerald-500/50"
                inputMode="numeric"
                value={settings.borrowLanPortInput}
                onChange={(event) => settings.setBorrowLanPortInput(event.target.value)}
                disabled={settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
                placeholder="8787"
              />
            </div>
            <div className="flex items-end justify-end">
              <button
                aria-label="Save LAN settings"
                className="inline-flex h-8 w-8 items-center justify-center rounded-[8px] border border-emerald-500/60 bg-emerald-500 text-[#03130d] transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-50"
                onClick={() => void settings.handleSaveBorrowLanSettings()}
                disabled={settings.isSavingBorrowLanSettings || settings.isDetectingBorrowLanHost}
                title="Save LAN settings"
                type="button"
              >
                <Save aria-hidden="true" size={14} />
              </button>
            </div>
          </div>
          {settings.borrowLanDetectionNote && (
            <div className="mt-2 text-[11px] text-[var(--primary)]">{settings.borrowLanDetectionNote}</div>
          )}
          {settings.borrowLanMessage && (
            <div className="mt-2 text-[11px] text-[var(--text-secondary)]">{settings.borrowLanMessage}</div>
          )}
        </div>

        <BorrowPolicyCard
          english={policyEnglish}
          vietnamese={policyVietnamese}
          savedEnglish={savedPolicyEnglish}
          savedVietnamese={savedPolicyVietnamese}
          isLoading={isLoadingPolicy}
          isSaving={isSavingPolicy}
          message={policyMessage}
          onEnglishChange={onPolicyEnglishChange ?? (() => undefined)}
          onVietnameseChange={onPolicyVietnameseChange ?? (() => undefined)}
          onSave={onSavePolicy ?? (() => undefined)}
        />
      </div>
    </div>
  )
}
