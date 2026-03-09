import { useCallback, useEffect, useMemo, useState } from "react"
import type { MouseEvent as ReactMouseEvent } from "react"
import type { EmployeeColumnDefinition } from "../../types/staff"
import type {
    UiColumnDefinition,
    ColumnPreferences,
    ColumnWidthMap,
    ActiveResizeState,
} from "../../types/app"
import { staffApi } from "../../services/staff-api"
import { getErrorMessage } from "../../lib/utils"
import {
    readColumnPreferences,
    readColumnLabelOverrides,
    readColumnWidths,
    serializeColumnPreferences,
    serializeStringMap,
    serializeWidthMap,
} from "../../lib/storage"
import {
    DEFAULT_SYSTEM_COLUMNS,
    DEFAULT_VISIBLE_COLUMN_KEYS,
    DEFAULT_COLUMN_WIDTHS,
    MIN_COLUMN_WIDTH,
    MAX_COLUMN_WIDTH,
    COLUMN_PREFS_VERSION,
} from "../../lib/constants"

// ── Column preference helpers ─────────────────────────────────────────────────

function reconcileColumnPreferences(
    preferences: ColumnPreferences,
    configurableKeys: string[],
): ColumnPreferences {
    const keySet = new Set(configurableKeys)
    const existingOrder = preferences.order.filter((key) => keySet.has(key))
    const missing = configurableKeys.filter((key) => !existingOrder.includes(key))
    return {
        order: [...existingOrder, ...missing],
        hidden: preferences.hidden.filter((key) => keySet.has(key)),
    }
}

function buildDefaultColumnPreferences(configurableKeys: string[]): ColumnPreferences {
    const visibleSet = new Set(DEFAULT_VISIBLE_COLUMN_KEYS)
    const prioritized = DEFAULT_VISIBLE_COLUMN_KEYS.filter((key) => configurableKeys.includes(key))
    const remaining = configurableKeys.filter((key) => !prioritized.includes(key))
    const order = [...prioritized, ...remaining]
    return {
        order,
        hidden: order.filter((key) => !visibleSet.has(key)),
    }
}

type UseColumnStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    reloadToken: number
    scopedColumnPrefsKey: string
    scopedColumnPrefsVersionKey: string
    scopedColumnLabelOverridesKey: string
    scopedColumnWidthsKey: string
    activeAccountName: string
    setGlobalError: (msg: string | null) => void
}

export type ColumnState = ReturnType<typeof useColumnState>

export function useColumnState({
    dbReady,
    isAuthenticated,
    reloadToken,
    scopedColumnPrefsKey,
    scopedColumnPrefsVersionKey,
    scopedColumnLabelOverridesKey,
    scopedColumnWidthsKey,
    setGlobalError,
}: UseColumnStateOptions) {
    const [columnDefinitions, setColumnDefinitions] = useState<EmployeeColumnDefinition[]>([])
    const [isLoadingColumns, setLoadingColumns] = useState(false)
    const [isMutatingColumns, setMutatingColumns] = useState(false)

    const [columnPreferences, setColumnPreferences] = useState<ColumnPreferences>(() =>
        readColumnPreferences(scopedColumnPrefsKey),
    )
    const [columnLabelOverrides, setColumnLabelOverrides] = useState<Record<string, string>>(
        () => readColumnLabelOverrides(scopedColumnLabelOverridesKey),
    )
    const [columnWidths, setColumnWidths] = useState<ColumnWidthMap>(() =>
        readColumnWidths(scopedColumnWidthsKey),
    )
    const [savedColumnPreferences, setSavedColumnPreferences] = useState<ColumnPreferences>(() =>
        readColumnPreferences(scopedColumnPrefsKey),
    )
    const [savedColumnLabelOverrides, setSavedColumnLabelOverrides] = useState<Record<string, string>>(
        () => readColumnLabelOverrides(scopedColumnLabelOverridesKey),
    )
    const [savedColumnWidths, setSavedColumnWidths] = useState<ColumnWidthMap>(() =>
        readColumnWidths(scopedColumnWidthsKey),
    )
    const [undoColumnResetSnapshot, setUndoColumnResetSnapshot] = useState<{
        preferences: ColumnPreferences
        widths: ColumnWidthMap
    } | null>(null)
    const [columnSearchTerm, setColumnSearchTerm] = useState("")
    const [newColumnTitle, setNewColumnTitle] = useState("")
    const [isColumnsDrawerOpen, setColumnsDrawerOpen] = useState(false)
    const [draggingColumnKey, setDraggingColumnKey] = useState<string | null>(null)
    const [activeResize, setActiveResize] = useState<ActiveResizeState | null>(null)

    // Load columns when authenticated
    useEffect(() => {
        if (!dbReady || !isAuthenticated) return

        let disposed = false

        void (async () => {
            try {
                setLoadingColumns(true)
                const columns = await staffApi.listEmployeeColumns()
                if (!disposed) {
                    setColumnDefinitions(columns)
                }
            } catch (error) {
                if (!disposed) {
                    setGlobalError(getErrorMessage(error))
                }
            } finally {
                if (!disposed) {
                    setLoadingColumns(false)
                }
            }
        })()

        return () => {
            disposed = true
        }
    }, [dbReady, isAuthenticated, reloadToken, setGlobalError])

    // Reset column prefs on login
    const resetColumnPrefsOnAuth = useCallback(() => {
        const nextPrefs = readColumnPreferences(scopedColumnPrefsKey)
        const nextLabels = readColumnLabelOverrides(scopedColumnLabelOverridesKey)
        const nextWidths = readColumnWidths(scopedColumnWidthsKey)
        setColumnPreferences(nextPrefs)
        setColumnLabelOverrides(nextLabels)
        setColumnWidths(nextWidths)
        setSavedColumnPreferences(nextPrefs)
        setSavedColumnLabelOverrides(nextLabels)
        setSavedColumnWidths(nextWidths)
        setColumnSearchTerm("")
    }, [scopedColumnLabelOverridesKey, scopedColumnPrefsKey, scopedColumnWidthsKey])

    // Column resize mouse tracking
    useEffect(() => {
        if (!activeResize) return

        document.body.classList.add("column-resize-active")

        const handlePointerMove = (event: MouseEvent) => {
            const deltaX = event.clientX - activeResize.startX
            const nextWidth = Math.max(
                MIN_COLUMN_WIDTH,
                Math.min(MAX_COLUMN_WIDTH, activeResize.startWidth + deltaX),
            )
            setColumnWidths((prev) => ({
                ...prev,
                [activeResize.key]: Math.round(nextWidth),
            }))
        }

        const handlePointerUp = () => {
            setActiveResize(null)
        }

        window.addEventListener("mousemove", handlePointerMove)
        window.addEventListener("mouseup", handlePointerUp)
        return () => {
            document.body.classList.remove("column-resize-active")
            window.removeEventListener("mousemove", handlePointerMove)
            window.removeEventListener("mouseup", handlePointerUp)
        }
    }, [activeResize])

    // ── Derived ──────────────────────────────────────────────────────────────────

    const uiColumns = useMemo<UiColumnDefinition[]>(() => {
        const dbColumns = columnDefinitions.map((column) => ({
            key: column.key,
            label: columnLabelOverrides[column.key]?.trim() || column.label,
            source: column.source,
        }))
        return [DEFAULT_SYSTEM_COLUMNS[0], ...dbColumns]
    }, [columnDefinitions, columnLabelOverrides])

    const configurableColumns = uiColumns

    const configurableColumnMap = useMemo(
        () => new Map(configurableColumns.map((column) => [column.key, column])),
        [configurableColumns],
    )

    const configurableKeys = useMemo(
        () => configurableColumns.map((column) => column.key),
        [configurableColumns],
    )

    const effectiveColumnPreferences = useMemo(
        () => reconcileColumnPreferences(columnPreferences, configurableKeys),
        [columnPreferences, configurableKeys],
    )

    const visibleColumnKeys = useMemo(
        () =>
            effectiveColumnPreferences.order.filter(
                (key) => !effectiveColumnPreferences.hidden.includes(key),
            ),
        [effectiveColumnPreferences.hidden, effectiveColumnPreferences.order],
    )

    const visibleColumns = useMemo(
        () =>
            visibleColumnKeys
                .map((key) => uiColumns.find((column) => column.key === key))
                .filter(Boolean) as UiColumnDefinition[],
        [uiColumns, visibleColumnKeys],
    )

    const resolvedColumnWidths = useMemo<ColumnWidthMap>(() => {
        const next: ColumnWidthMap = {}
        for (const column of uiColumns) {
            const savedWidth = columnWidths[column.key]
            const fallback = DEFAULT_COLUMN_WIDTHS[column.key] ?? 170
            const width =
                typeof savedWidth === "number" && !Number.isNaN(savedWidth) ? savedWidth : fallback
            next[column.key] = Math.max(MIN_COLUMN_WIDTH, Math.min(MAX_COLUMN_WIDTH, Math.round(width)))
        }
        return next
    }, [columnWidths, uiColumns])

    const filteredColumnKeys = useMemo(() => {
        const keyword = columnSearchTerm.trim().toLowerCase()
        if (!keyword) {
            return effectiveColumnPreferences.order
        }
        return effectiveColumnPreferences.order.filter((key) => {
            const column = configurableColumnMap.get(key)
            if (!column) return false
            return `${column.label} ${column.key}`.toLowerCase().includes(keyword)
        })
    }, [columnSearchTerm, configurableColumnMap, effectiveColumnPreferences.order])

    const hasUnsavedColumnProfileChanges = useMemo(
        () =>
            serializeColumnPreferences(columnPreferences) !==
            serializeColumnPreferences(savedColumnPreferences) ||
            serializeStringMap(columnLabelOverrides) !== serializeStringMap(savedColumnLabelOverrides) ||
            serializeWidthMap(columnWidths) !== serializeWidthMap(savedColumnWidths),
        [
            columnLabelOverrides,
            columnPreferences,
            columnWidths,
            savedColumnLabelOverrides,
            savedColumnPreferences,
            savedColumnWidths,
        ],
    )

    // Version-based prefs migration — runs AFTER configurableKeys is defined
    useEffect(() => {
        if (!isAuthenticated) return
        if (columnDefinitions.length === 0) return
        if (configurableKeys.length === 0) return

        const currentVersion = localStorage.getItem(scopedColumnPrefsVersionKey)
        if (currentVersion !== COLUMN_PREFS_VERSION) {
            const defaults = buildDefaultColumnPreferences(configurableKeys)
            localStorage.setItem(scopedColumnPrefsKey, JSON.stringify(defaults))
            localStorage.setItem(scopedColumnLabelOverridesKey, JSON.stringify({}))
            localStorage.setItem(scopedColumnWidthsKey, JSON.stringify({}))
            setColumnPreferences(defaults)
            setColumnLabelOverrides({})
            setColumnWidths({})
            setSavedColumnPreferences(defaults)
            setSavedColumnLabelOverrides({})
            setSavedColumnWidths({})
            localStorage.setItem(scopedColumnPrefsVersionKey, COLUMN_PREFS_VERSION)
            return
        }

        const reconciled = reconcileColumnPreferences(columnPreferences, configurableKeys)
        const shouldUpdateOrder = reconciled.order.join("|") !== columnPreferences.order.join("|")
        const shouldUpdateHidden = reconciled.hidden.join("|") !== columnPreferences.hidden.join("|")
        if (shouldUpdateOrder || shouldUpdateHidden) {
            setColumnPreferences(reconciled)
        }
    }, [
        columnDefinitions.length,
        isAuthenticated,
        columnPreferences,
        configurableKeys,
        scopedColumnLabelOverridesKey,
        scopedColumnPrefsKey,
        scopedColumnPrefsVersionKey,
        scopedColumnWidthsKey,
    ])

    // ── Handlers ─────────────────────────────────────────────────────────────────

    const startColumnResize = (event: ReactMouseEvent<HTMLButtonElement>, key: string) => {
        event.preventDefault()
        event.stopPropagation()
        setActiveResize({
            key,
            startX: event.clientX,
            startWidth: resolvedColumnWidths[key] ?? DEFAULT_COLUMN_WIDTHS[key] ?? 170,
        })
    }

    const toggleColumnVisibility = (key: string) => {
        setColumnPreferences((prev) => {
            const reconciled = reconcileColumnPreferences(prev, configurableKeys)
            const isHidden = reconciled.hidden.includes(key)
            return {
                ...reconciled,
                hidden: isHidden
                    ? reconciled.hidden.filter((item) => item !== key)
                    : [...reconciled.hidden, key],
            }
        })
    }

    const reorderColumns = (sourceKey: string, targetKey: string) => {
        if (sourceKey === targetKey) return
        setColumnPreferences((prev) => {
            const reconciled = reconcileColumnPreferences(prev, configurableKeys)
            const nextOrder = [...reconciled.order]
            const from = nextOrder.indexOf(sourceKey)
            const to = nextOrder.indexOf(targetKey)
            if (from === -1 || to === -1) return reconciled
            const [moved] = nextOrder.splice(from, 1)
            nextOrder.splice(to, 0, moved)
            return { ...reconciled, order: nextOrder }
        })
    }

    const moveColumnByOffset = (key: string, offset: -1 | 1) => {
        setColumnPreferences((prev) => {
            const reconciled = reconcileColumnPreferences(prev, configurableKeys)
            const nextOrder = [...reconciled.order]
            const from = nextOrder.indexOf(key)
            if (from === -1) return reconciled
            const to = Math.max(0, Math.min(nextOrder.length - 1, from + offset))
            if (to === from) return reconciled
            const [moved] = nextOrder.splice(from, 1)
            nextOrder.splice(to, 0, moved)
            return { ...reconciled, order: nextOrder }
        })
    }

    const handleSaveColumnProfile = () => {
        localStorage.setItem(scopedColumnPrefsKey, JSON.stringify(columnPreferences))
        localStorage.setItem(scopedColumnLabelOverridesKey, JSON.stringify(columnLabelOverrides))
        localStorage.setItem(scopedColumnWidthsKey, JSON.stringify(columnWidths))
        localStorage.setItem(scopedColumnPrefsVersionKey, COLUMN_PREFS_VERSION)
        setSavedColumnPreferences(columnPreferences)
        setSavedColumnLabelOverrides(columnLabelOverrides)
        setSavedColumnWidths(columnWidths)
        setUndoColumnResetSnapshot(null)
    }

    const resetColumnPreferences = () => {
        const firstConfirm = window.confirm("Reset column view to default for this profile?")
        if (!firstConfirm) return
        const secondConfirm = window.confirm(
            "Final confirmation: this will discard your current column arrangement/visibility unless you click Undo Reset. Continue?",
        )
        if (!secondConfirm) return
        setUndoColumnResetSnapshot({ preferences: columnPreferences, widths: columnWidths })
        setColumnPreferences(buildDefaultColumnPreferences(configurableKeys))
        setColumnWidths({})
    }

    const undoResetColumnPreferences = () => {
        if (!undoColumnResetSnapshot) return
        setColumnPreferences(undoColumnResetSnapshot.preferences)
        setColumnWidths(undoColumnResetSnapshot.widths)
        setUndoColumnResetSnapshot(null)
    }

    const handleRenameColumn = async (column: UiColumnDefinition, triggerReload: () => void) => {
        const nextTitle = window.prompt("Column title", column.label)
        if (nextTitle === null) return
        const normalized = nextTitle.trim()

        if (normalized.length === 0) {
            setColumnLabelOverrides((prev) => {
                const next = { ...prev }
                delete next[column.key]
                return next
            })
            return
        }

        if (column.source === "dynamic") {
            try {
                setMutatingColumns(true)
                await staffApi.upsertEmployeeColumn({ key: column.key, label: normalized })
                setColumnLabelOverrides((prev) => {
                    const next = { ...prev }
                    delete next[column.key]
                    return next
                })
                triggerReload()
            } catch (error) {
                setGlobalError(getErrorMessage(error))
            } finally {
                setMutatingColumns(false)
            }
            return
        }

        setColumnLabelOverrides((prev) => ({ ...prev, [column.key]: normalized }))
    }

    const handleAddColumn = async (triggerReload: () => void) => {
        const title = newColumnTitle.trim()
        if (!title) return
        try {
            setMutatingColumns(true)
            await staffApi.upsertEmployeeColumn({ key: null, label: title })
            setNewColumnTitle("")
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setMutatingColumns(false)
        }
    }

    const handleDeleteColumn = async (column: UiColumnDefinition, triggerReload: () => void) => {
        if (column.source !== "dynamic") return
        const accepted = window.confirm(`Delete column '${column.label}'?`)
        if (!accepted) return
        try {
            setMutatingColumns(true)
            await staffApi.deleteEmployeeColumn(column.key)
            setColumnLabelOverrides((prev) => {
                const next = { ...prev }
                delete next[column.key]
                return next
            })
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setMutatingColumns(false)
        }
    }

    const resetColumnStateOnLogout = useCallback(() => {
        setColumnPreferences({ order: [], hidden: [] })
        setColumnLabelOverrides({})
        setColumnWidths({})
        setSavedColumnPreferences({ order: [], hidden: [] })
        setSavedColumnLabelOverrides({})
        setSavedColumnWidths({})
        setColumnsDrawerOpen(false)
    }, [])

    return {
        // raw state
        columnDefinitions,
        isLoadingColumns,
        isMutatingColumns,
        columnPreferences,
        columnLabelOverrides,
        columnWidths,
        undoColumnResetSnapshot,
        columnSearchTerm,
        setColumnSearchTerm,
        newColumnTitle,
        setNewColumnTitle,
        isColumnsDrawerOpen,
        setColumnsDrawerOpen,
        draggingColumnKey,
        setDraggingColumnKey,
        activeResize,
        // derived
        uiColumns,
        configurableColumns,
        configurableColumnMap,
        configurableKeys,
        effectiveColumnPreferences,
        visibleColumnKeys,
        visibleColumns,
        resolvedColumnWidths,
        filteredColumnKeys,
        hasUnsavedColumnProfileChanges,
        // handlers
        startColumnResize,
        toggleColumnVisibility,
        reorderColumns,
        moveColumnByOffset,
        handleSaveColumnProfile,
        resetColumnPreferences,
        undoResetColumnPreferences,
        handleRenameColumn,
        handleAddColumn,
        handleDeleteColumn,
        resetColumnStateOnLogout,
        resetColumnPrefsOnAuth,
    }
}
