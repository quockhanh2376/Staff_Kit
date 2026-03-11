import { ArrowDown, ArrowUp, ArrowUpDown, LoaderCircle, MoreHorizontal, Plus } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import type { TeamState } from "./useTeamState"
import type { TeamRecord } from "../../types/staff"

type SortKey = "name" | "memberCount"
type SortDir = "asc" | "desc"

type TeamViewProps = {
    teamState: TeamState
}

function SortIndicator({
    activeKey,
    colKey,
    sortDir,
}: {
    activeKey: SortKey
    colKey: SortKey
    sortDir: SortDir
}) {
    if (activeKey !== colKey) return <ArrowUpDown size={11} className="opacity-40" />
    return sortDir === "asc" ? <ArrowUp size={11} /> : <ArrowDown size={11} />
}

function TeamTableHeader({
    sortKey,
    sortDir,
    onToggleSort,
}: {
    sortKey: SortKey
    sortDir: SortDir
    onToggleSort: (key: SortKey) => void
}) {
    return (
        <div className="team-grid-header grid grid-cols-[2rem_1fr_5rem_auto] items-center gap-1 border-b-2 border-[var(--primary)]/40 bg-[var(--surface-hover)] px-2 py-2.5 font-bold uppercase tracking-[0.08em] text-[var(--primary)]">
            <span className="text-center">#</span>
            <button
                type="button"
                className="team-grid-header-button inline-flex items-center gap-1.5 hover:brightness-125"
                onClick={() => onToggleSort("name")}
            >
                Team Name <SortIndicator activeKey={sortKey} colKey="name" sortDir={sortDir} />
            </button>
            <button
                type="button"
                className="team-grid-header-button inline-flex items-center gap-1.5 hover:brightness-125"
                onClick={() => onToggleSort("memberCount")}
            >
                Members <SortIndicator activeKey={sortKey} colKey="memberCount" sortDir={sortDir} />
            </button>
            <span className="text-center">Action</span>
        </div>
    )
}

function ActionMenu({ t, teamState, flipUp }: { t: TeamRecord; teamState: TeamState; flipUp?: boolean }) {
    const [open, setOpen] = useState(false)
    const ref = useRef<HTMLDivElement>(null)

    useEffect(() => {
        if (!open) return
        const handler = (e: MouseEvent) => {
            if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
        }
        document.addEventListener("mousedown", handler)
        return () => document.removeEventListener("mousedown", handler)
    }, [open])

    const dropdownPos = flipUp
        ? "absolute right-0 bottom-full z-50 mb-1"
        : "absolute right-0 top-full z-50 mt-1"

    return (
        <div ref={ref} className="relative">
            <button
                type="button"
                className="team-grid-action-button icon-button flex items-center gap-1 py-0.5"
                onClick={() => setOpen((v) => !v)}
                title="Actions"
            >
                <MoreHorizontal size={15} />
            </button>
            {open && (
                <div className={`${dropdownPos} min-w-[130px] rounded-[8px] border border-[var(--border)] bg-[var(--surface)] py-1 shadow-lg`}>
                    <button
                        type="button"
                        className="flex w-full items-center px-3 py-1.5 text-left text-sm text-[var(--text-primary)] hover:bg-[var(--surface-hover)]"
                        onClick={() => {
                            void teamState.handleRenameTeam(t)
                            setOpen(false)
                        }}
                    >
                        Rename
                    </button>
                    <button
                        type="button"
                        className="flex w-full items-center px-3 py-1.5 text-left text-sm text-[var(--error)] hover:bg-[var(--surface-hover)]"
                        onClick={() => {
                            void teamState.handleDeleteTeam(t)
                            setOpen(false)
                        }}
                    >
                        Remove
                    </button>
                </div>
            )}
        </div>
    )
}

export function TeamView({ teamState }: TeamViewProps) {
    const team = teamState

    const [sortKey, setSortKey] = useState<SortKey>("name")
    const [sortDir, setSortDir] = useState<SortDir>("asc")

    const sortedTeams = useMemo(() => {
        return [...team.teams].sort((a, b) => {
            let cmp = 0
            if (sortKey === "name") cmp = a.name.localeCompare(b.name)
            else if (sortKey === "memberCount") cmp = (a.memberCount ?? 0) - (b.memberCount ?? 0)
            return sortDir === "asc" ? cmp : -cmp
        })
    }, [team.teams, sortKey, sortDir])

    const toggleSort = (key: SortKey) => {
        if (sortKey === key) setSortDir((d) => (d === "asc" ? "desc" : "asc"))
        else {
            setSortKey(key)
            setSortDir("asc")
        }
    }

    const total = sortedTeams.length
    const col1Size = Math.ceil(total / 3)
    const col2Size = Math.ceil((total - col1Size) / 2)
    const col1 = sortedTeams.slice(0, col1Size)
    const col2 = sortedTeams.slice(col1Size, col1Size + col2Size)
    const col3 = sortedTeams.slice(col1Size + col2Size)

    const renderRows = (items: TeamRecord[], startIndex: number) =>
        items.map((t, localIdx) => {
            const globalIndex = startIndex + localIdx
            const flipUp = localIdx >= items.length - 3
            return (
                <div
                    key={t.id}
                    className="team-grid-row grid grid-cols-[2rem_1fr_5rem_auto] items-center gap-1 border-b border-[var(--border)] px-2 py-1 hover:bg-[var(--surface-hover)]"
                >
                    <span className="team-grid-index text-center">{globalIndex + 1}</span>
                    <span className="team-grid-name truncate font-medium" title={t.name}>
                        {t.name}
                    </span>
                    <span className="team-grid-count text-center">{t.memberCount ?? 0}</span>
                    <div className="flex justify-center">
                        <ActionMenu t={t} teamState={team} flipUp={flipUp} />
                    </div>
                </div>
            )
        })

    return (
        <section className="px-4 py-7 md:px-8">
            <div className="mb-4 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
                <div>
                    <h2 className="text-[30px] font-bold">Teams</h2>
                    <p className="mt-1 text-[15px] text-[var(--text-secondary)]">
                        Manage team names and hierarchy used by employee records.
                        <span className="ml-2 rounded-full bg-[var(--primary)]/15 px-2 py-0.5 text-xs text-[var(--primary)]">
                            {team.teams.length} total
                        </span>
                    </p>
                </div>
                <div className="flex items-center gap-2">
                    <input
                        className="form-input w-[200px]"
                        placeholder="New team name"
                        value={team.newTeamName}
                        onChange={(event) => team.setNewTeamName(event.target.value)}
                        onKeyDown={(event) => {
                            if (event.key === "Enter") void team.handleCreateTeam()
                        }}
                    />
                    <button
                        className="inline-flex items-center gap-2 rounded-[8px] bg-[var(--primary)] px-4 py-2.5 font-semibold text-[#00131c] transition hover:brightness-110"
                        onClick={() => void team.handleCreateTeam()}
                        type="button"
                        disabled={team.isSavingTeam}
                    >
                        {team.isSavingTeam ? <LoaderCircle className="animate-spin" size={16} /> : <Plus size={18} />}
                        Add
                    </button>
                </div>
            </div>

            <div className="grid grid-cols-3 gap-x-3">
                <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] overflow-hidden">
                    <TeamTableHeader sortKey={sortKey} sortDir={sortDir} onToggleSort={toggleSort} />
                    <div>{renderRows(col1, 0)}</div>
                </div>

                <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] overflow-hidden">
                    <TeamTableHeader sortKey={sortKey} sortDir={sortDir} onToggleSort={toggleSort} />
                    <div>{renderRows(col2, col1Size)}</div>
                </div>

                <div className="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] overflow-hidden">
                    <TeamTableHeader sortKey={sortKey} sortDir={sortDir} onToggleSort={toggleSort} />
                    <div>{renderRows(col3, col1Size + col2Size)}</div>
                </div>
            </div>

            {team.isLoadingTeams && (
                <div className="mt-3 text-sm text-[var(--text-secondary)]">Refreshing teams...</div>
            )}
        </section>
    )
}
