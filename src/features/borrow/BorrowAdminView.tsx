import { useEffect, useRef } from "react"
import type { AuthState } from "../auth/useAuthState"
import type { SettingsState } from "../settings/useSettingsState"
import { BorrowLanQrCard } from "./BorrowLanQrCard"
import type { BorrowState } from "./useBorrowState"
import { buildBorrowReviewHeaderDescription, buildBorrowReviewHeading } from "./borrowReviewCopy"
import { BorrowPendingQueue } from "./BorrowPendingQueue"

type BorrowAdminViewProps = {
  auth: AuthState
  borrow: BorrowState
  settings: SettingsState
}

const ACTIVE_QUEUE_INTERVAL_MS = 3000
const QUIET_QUEUE_INTERVAL_MS = 10000
const QUIET_QUEUE_THRESHOLD_MS = 120000

export function BorrowAdminView({ auth, borrow, settings }: BorrowAdminViewProps) {
  const refreshQueueRef = useRef(borrow.refreshQueue)

  useEffect(() => {
    refreshQueueRef.current = borrow.refreshQueue
  }, [borrow.refreshQueue])

  useEffect(() => {
    if (!auth.isAdminAccount) return

    let disposed = false
    let timer: number | undefined
    let quietSince = Date.now()

    const scheduleNext = (delay: number) => {
      timer = window.setTimeout(async () => {
        const changed = await refreshQueueRef.current(undefined, { silent: true })
        if (disposed) return
        if (changed) quietSince = Date.now()
        const quiet = Date.now() - quietSince >= QUIET_QUEUE_THRESHOLD_MS
        scheduleNext(quiet ? QUIET_QUEUE_INTERVAL_MS : ACTIVE_QUEUE_INTERVAL_MS)
      }, delay)
    }

    const fetchImmediately = async () => {
      const changed = await refreshQueueRef.current()
      if (disposed) return
      if (changed) quietSince = Date.now()
      scheduleNext(ACTIVE_QUEUE_INTERVAL_MS)
    }

    void fetchImmediately()

    return () => {
      disposed = true
      if (timer !== undefined) window.clearTimeout(timer)
    }
  }, [auth.isAdminAccount])

  if (!auth.isAdminAccount) {
    return (
      <section className="px-4 py-7 md:px-8">
        <h2 className="text-[30px] font-bold">{buildBorrowReviewHeading()}</h2>
        <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--surface)] p-4 text-sm text-[var(--text-secondary)]">
          Admin access is required to review pending borrow and return requests.
        </div>
      </section>
    )
  }

  return (
    <section className="px-4 py-7 md:px-8">
      <div>
        <div>
          <h2 className="text-[30px] font-bold">{buildBorrowReviewHeading()}</h2>
          <p className="mt-1 text-sm text-[var(--text-secondary)]">
            {buildBorrowReviewHeaderDescription()}
          </p>
        </div>
      </div>

      <BorrowLanQrCard
        settings={settings}
        isAdmin={auth.isAdminAccount}
        isQueueRefreshing={borrow.isLoadingQueue}
        onRefreshQueue={() => void borrow.refreshQueue()}
        onStartLan={() => void settings.ensureBorrowLanReady()}
        onStopLan={() => void settings.handleStopBorrowLanServer()}
      />

      <div className="mt-4">
        <BorrowPendingQueue borrow={borrow} />
      </div>
    </section>
  )
}
