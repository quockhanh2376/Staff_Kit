import { RefreshCw } from "lucide-react"
import type { AuthState } from "../auth/useAuthState"
import type { BorrowState } from "./useBorrowState"
import { buildBorrowReviewHeaderDescription, buildBorrowReviewHeading } from "./borrowReviewCopy"
import { BorrowPendingQueue } from "./BorrowPendingQueue"
import { BorrowRequestDetail } from "./BorrowRequestDetail"

type BorrowAdminViewProps = {
  auth: AuthState
  borrow: BorrowState
}

export function BorrowAdminView({ auth, borrow }: BorrowAdminViewProps) {
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
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-[30px] font-bold">{buildBorrowReviewHeading()}</h2>
          <p className="mt-1 text-sm text-[var(--text-secondary)]">
            {buildBorrowReviewHeaderDescription()}
          </p>
        </div>
        <button
          className="inline-flex items-center gap-2 rounded-[8px] border border-[var(--border)] px-3 py-2 text-xs font-semibold text-[var(--text-primary)] transition hover:bg-[var(--surface-hover)] disabled:opacity-50"
          onClick={() => void borrow.refreshQueue(borrow.selectedRequestId)}
          type="button"
          disabled={borrow.isLoadingQueue}
        >
          <RefreshCw className={borrow.isLoadingQueue ? "animate-spin" : ""} size={14} />
          Refresh Queue
        </button>
      </div>

      <div className="mt-4">
        <BorrowPendingQueue borrow={borrow} />
      </div>

      <BorrowRequestDetail borrow={borrow} />
    </section>
  )
}
