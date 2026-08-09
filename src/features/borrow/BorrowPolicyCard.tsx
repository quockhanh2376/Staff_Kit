import { LoaderCircle, Save } from "lucide-react"

type BorrowPolicyCardProps = {
  english: string
  vietnamese: string
  savedEnglish: string
  savedVietnamese: string
  isLoading: boolean
  isSaving: boolean
  message: string
  onEnglishChange: (value: string) => void
  onVietnameseChange: (value: string) => void
  onSave: () => void
}

export function BorrowPolicyCard({
  english,
  vietnamese,
  savedEnglish,
  savedVietnamese,
  isLoading,
  isSaving,
  message,
  onEnglishChange,
  onVietnameseChange,
  onSave,
}: BorrowPolicyCardProps) {
  const isDirty = !savedEnglish.trim() && !savedVietnamese.trim()
    ? true
    : english !== savedEnglish || vietnamese !== savedVietnamese

  return (
    <div data-testid="handle-with-care-card" className="min-w-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)]/25 p-3">
      <div className="flex items-center justify-between gap-2">
        <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]">
          Handle with Care
        </div>
        <button
          aria-label="Save Handle with Care policy"
          className="inline-flex h-8 w-8 items-center justify-center rounded-[8px] border border-emerald-500/60 bg-emerald-500 text-[#03130d] transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={isLoading || isSaving || !isDirty}
          onClick={onSave}
          title="Save Handle with Care policy"
          type="button"
        >
          {isSaving ? <LoaderCircle aria-hidden="true" className="animate-spin" size={14} /> : <Save aria-hidden="true" size={14} />}
        </button>
      </div>

      {isLoading ? (
        <div className="mt-3 flex items-center gap-2 text-xs text-[var(--text-secondary)]">
          <LoaderCircle className="animate-spin" size={13} />
          Loading policy...
        </div>
      ) : (
        <div className="mt-3 space-y-3">
          <div>
            <label className="mb-1 block text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]" htmlFor="borrow-policy-english">
              English
            </label>
            <textarea
              id="borrow-policy-english"
              aria-label="English Handle with Care policy"
              className="form-input min-h-[96px] resize-y text-sm"
              spellCheck={false}
              value={english}
              onChange={(event) => onEnglishChange(event.target.value)}
              placeholder="Enter the English policy..."
            />
          </div>
          <div>
            <label className="mb-1 block text-[11px] font-semibold uppercase tracking-[0.06em] text-[var(--text-secondary)]" htmlFor="borrow-policy-vietnamese">
              Vietnamese
            </label>
            <textarea
              id="borrow-policy-vietnamese"
              aria-label="Vietnamese Handle with Care policy"
              className="form-input min-h-[96px] resize-y text-sm"
              spellCheck={false}
              value={vietnamese}
              onChange={(event) => onVietnameseChange(event.target.value)}
              placeholder="Nhập chính sách tiếng Việt..."
            />
          </div>
        </div>
      )}
      {message && <div className="mt-2 text-xs text-[var(--text-secondary)]" role="status">{message}</div>}
    </div>
  )
}
