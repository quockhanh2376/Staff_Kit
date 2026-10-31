import { useEffect, useState, useCallback } from "react"
import {
  FileText,
  Search,
  RefreshCw,
  ChevronLeft,
  ChevronRight,
  Shield,
  Briefcase,
  Eye,
  X,
} from "lucide-react"
import { staffApi } from "../../services/staff-api"
import type { AuditLogRecord } from "../../types/staff"
import type { LocalAccountRole } from "../../types/app"

type AuditLogViewerProps = {
  userRole?: LocalAccountRole | null
  onError?: (msg: string | null) => void
}

export function AuditLogViewer({ userRole, onError }: AuditLogViewerProps) {
  const isSuperAdmin = userRole === "super_admin"
  const [logs, setLogs] = useState<AuditLogRecord[]>([])
  const [totalCount, setTotalCount] = useState(0)
  const [isLoading, setIsLoading] = useState(false)
  const [category, setCategory] = useState<"all" | "security" | "business">(
    isSuperAdmin ? "all" : "business"
  )
  const [searchInput, setSearchInput] = useState("")
  const [searchTerm, setSearchTerm] = useState("")
  const [page, setPage] = useState(0)
  const pageSize = 20

  const [selectedLog, setSelectedLog] = useState<AuditLogRecord | null>(null)

  const fetchLogs = useCallback(async () => {
    setIsLoading(true)
    try {
      const res = await staffApi.queryAuditLogs({
        limit: pageSize,
        offset: page * pageSize,
        eventCategory: isSuperAdmin ? category : "business",
        search: searchTerm.trim() || undefined,
      })
      setLogs(res.logs)
      setTotalCount(res.totalCount)
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      onError?.(`Failed to load audit logs: ${msg}`)
    } finally {
      setIsLoading(false)
    }
  }, [category, isSuperAdmin, onError, page, pageSize, searchTerm])

  useEffect(() => {
    void fetchLogs()
  }, [fetchLogs])

  const totalPages = Math.ceil(totalCount / pageSize)

  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setPage(0)
    setSearchTerm(searchInput)
  }

  const formatEventName = (raw: string) => {
    return raw.replace(/^(security|business)\./, "")
  }

  return (
    <div className="space-y-4">
      {/* Top toolbar */}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        {/* Category selector */}
        {isSuperAdmin ? (
          <div className="inline-flex rounded-[10px] border border-[var(--border)] bg-[var(--bg)] p-1 text-xs">
            <button
              type="button"
              className={`rounded-[7px] px-3 py-1.5 font-medium transition ${
                category === "all"
                  ? "bg-emerald-500 font-semibold text-[#03130d]"
                  : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              }`}
              onClick={() => {
                setCategory("all")
                setPage(0)
              }}
            >
              All Events
            </button>
            <button
              type="button"
              className={`inline-flex items-center gap-1.5 rounded-[7px] px-3 py-1.5 font-medium transition ${
                category === "security"
                  ? "bg-emerald-500 font-semibold text-[#03130d]"
                  : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              }`}
              onClick={() => {
                setCategory("security")
                setPage(0)
              }}
            >
              <Shield size={13} />
              Security
            </button>
            <button
              type="button"
              className={`inline-flex items-center gap-1.5 rounded-[7px] px-3 py-1.5 font-medium transition ${
                category === "business"
                  ? "bg-emerald-500 font-semibold text-[#03130d]"
                  : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              }`}
              onClick={() => {
                setCategory("business")
                setPage(0)
              }}
            >
              <Briefcase size={13} />
              Business
            </button>
          </div>
        ) : (
          <div className="inline-flex items-center gap-1.5 rounded-[999px] border border-emerald-500/30 bg-emerald-500/10 px-3 py-1 text-xs font-semibold text-emerald-300">
            <Briefcase size={13} />
            Business Audit Log (Read-only)
          </div>
        )}

        {/* Search bar & Refresh */}
        <div className="flex items-center gap-2">
          <form onSubmit={handleSearchSubmit} className="relative flex-1 sm:w-64">
            <input
              type="text"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              placeholder="Search audit records..."
              className="w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] pl-8 pr-3 py-1.5 text-xs text-[var(--text-primary)] placeholder:text-[var(--text-secondary)] outline-none transition focus:border-emerald-500/60"
            />
            <Search
              size={13}
              className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-secondary)]"
            />
          </form>

          <button
            type="button"
            onClick={() => void fetchLogs()}
            disabled={isLoading}
            className="inline-flex items-center justify-center rounded-[10px] border border-[var(--border)] bg-[var(--surface-hover)] p-2 text-[var(--text-secondary)] transition hover:text-[var(--text-primary)] disabled:opacity-50"
            title="Refresh logs"
          >
            <RefreshCw size={14} className={isLoading ? "animate-spin" : ""} />
          </button>
        </div>
      </div>

      {/* Audit table */}
      <div className="overflow-hidden rounded-[12px] border border-[var(--border)] bg-[var(--bg)] shadow-sm">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="border-b border-[var(--border)] bg-[var(--surface-hover)] uppercase tracking-wider text-[10px] font-semibold text-[var(--text-secondary)]">
              <tr>
                <th className="py-2.5 px-3">Timestamp</th>
                <th className="py-2.5 px-3">Category</th>
                <th className="py-2.5 px-3">Event</th>
                <th className="py-2.5 px-3">Actor</th>
                <th className="py-2.5 px-3">Target</th>
                <th className="py-2.5 px-3 text-right">Action</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--border)] font-mono text-[11px]">
              {logs.length === 0 ? (
                <tr>
                  <td colSpan={6} className="py-8 text-center text-[var(--text-secondary)] font-sans">
                    {isLoading ? (
                      <span className="inline-flex items-center gap-2">
                        <RefreshCw size={14} className="animate-spin text-emerald-400" />
                        Loading audit logs...
                      </span>
                    ) : (
                      "No audit records match the current filter."
                    )}
                  </td>
                </tr>
              ) : (
                logs.map((log) => {
                  const isSec = log.eventType.startsWith("security.") || !log.eventType.startsWith("business.")
                  return (
                    <tr
                      key={log.id}
                      className="transition-colors hover:bg-[var(--surface-hover)]/60 cursor-pointer"
                      onClick={() => setSelectedLog(log)}
                    >
                      <td className="py-2 px-3 whitespace-nowrap text-[var(--text-secondary)] font-sans">
                        {log.createdAt}
                      </td>
                      <td className="py-2 px-3 whitespace-nowrap">
                        <span
                          className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wider ${
                            isSec
                              ? "border border-amber-500/30 bg-amber-500/10 text-amber-300"
                              : "border border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
                          }`}
                        >
                          {isSec ? <Shield size={10} /> : <Briefcase size={10} />}
                          {isSec ? "Security" : "Business"}
                        </span>
                      </td>
                      <td className="py-2 px-3 font-semibold text-[var(--text-primary)]">
                        {formatEventName(log.eventType)}
                      </td>
                      <td className="py-2 px-3 text-[var(--text-secondary)] font-sans">
                        {log.actorRef || log.actorType}
                      </td>
                      <td className="py-2 px-3 text-[var(--text-secondary)] truncate max-w-[140px]" title={`${log.entityType}:${log.entityId}`}>
                        {log.entityType}:{log.entityId}
                      </td>
                      <td className="py-2 px-3 text-right">
                        <button
                          type="button"
                          className="inline-flex items-center gap-1 rounded-[6px] border border-[var(--border)] px-2 py-0.5 text-[10px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--surface-hover)] transition"
                          onClick={(e) => {
                            e.stopPropagation()
                            setSelectedLog(log)
                          }}
                        >
                          <Eye size={11} />
                          Details
                        </button>
                      </td>
                    </tr>
                  )
                })
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination footer */}
        <div className="flex items-center justify-between border-t border-[var(--border)] bg-[var(--surface-hover)] px-3 py-2 text-xs text-[var(--text-secondary)]">
          <div>
            Showing {logs.length > 0 ? page * pageSize + 1 : 0}–{Math.min((page + 1) * pageSize, totalCount)} of {totalCount} records
          </div>
          <div className="flex items-center gap-1">
            <button
              type="button"
              disabled={page <= 0 || isLoading}
              onClick={() => setPage((p) => Math.max(0, p - 1))}
              className="inline-flex items-center justify-center rounded-[6px] border border-[var(--border)] p-1 transition hover:bg-[var(--border)] disabled:opacity-40"
            >
              <ChevronLeft size={14} />
            </button>
            <span className="px-2 font-mono text-[11px]">
              Page {totalPages > 0 ? page + 1 : 1} of {Math.max(1, totalPages)}
            </span>
            <button
              type="button"
              disabled={page >= totalPages - 1 || isLoading}
              onClick={() => setPage((p) => p + 1)}
              className="inline-flex items-center justify-center rounded-[6px] border border-[var(--border)] p-1 transition hover:bg-[var(--border)] disabled:opacity-40"
            >
              <ChevronRight size={14} />
            </button>
          </div>
        </div>
      </div>

      {/* Detail modal */}
      {selectedLog && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm"
          onClick={() => setSelectedLog(null)}
        >
          <div
            className="w-full max-w-xl rounded-[16px] border border-[var(--border)] bg-[var(--surface)] p-5 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between border-b border-[var(--border)] pb-3">
              <div className="flex items-center gap-2">
                <FileText className="text-emerald-400" size={18} />
                <h3 className="text-sm font-semibold text-[var(--text-primary)]">
                  Audit Entry #{selectedLog.id}
                </h3>
              </div>
              <button
                type="button"
                className="rounded-[6px] p-1 text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                onClick={() => setSelectedLog(null)}
              >
                <X size={16} />
              </button>
            </div>

            <div className="grid grid-cols-2 gap-3 text-xs">
              <div>
                <span className="text-[10px] uppercase font-semibold text-[var(--text-secondary)]">Event Type</span>
                <p className="font-mono text-sm font-semibold text-[var(--text-primary)]">{selectedLog.eventType}</p>
              </div>
              <div>
                <span className="text-[10px] uppercase font-semibold text-[var(--text-secondary)]">Recorded At</span>
                <p className="font-mono text-[var(--text-primary)]">{selectedLog.createdAt}</p>
              </div>
              <div>
                <span className="text-[10px] uppercase font-semibold text-[var(--text-secondary)]">Actor</span>
                <p className="text-[var(--text-primary)]">{selectedLog.actorRef ? `${selectedLog.actorRef} (${selectedLog.actorType})` : selectedLog.actorType}</p>
              </div>
              <div>
                <span className="text-[10px] uppercase font-semibold text-[var(--text-secondary)]">Target Entity</span>
                <p className="font-mono text-[var(--text-primary)]">{selectedLog.entityType}: {selectedLog.entityId}</p>
              </div>
            </div>

            <div>
              <span className="text-[10px] uppercase font-semibold text-[var(--text-secondary)]">Event Payload</span>
              <pre className="mt-1 max-h-56 overflow-y-auto rounded-[8px] border border-[var(--border)] bg-[var(--bg)] p-3 font-mono text-[11px] text-[var(--text-primary)]">
                {selectedLog.payloadJson
                  ? JSON.stringify(JSON.parse(selectedLog.payloadJson), null, 2)
                  : "(empty)"}
              </pre>
            </div>

            <div className="flex justify-end pt-2 border-t border-[var(--border)]">
              <button
                type="button"
                className="rounded-[8px] border border-[var(--border)] bg-[var(--surface-hover)] px-4 py-1.5 text-xs font-semibold text-[var(--text-primary)] hover:brightness-95 transition"
                onClick={() => setSelectedLog(null)}
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
