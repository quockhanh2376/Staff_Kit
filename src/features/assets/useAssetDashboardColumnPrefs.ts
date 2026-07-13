import { useEffect, useMemo, useRef, useState } from "react"
import type { MouseEvent as ReactMouseEvent } from "react"

type ColumnDefinition<Key extends string> = {
  key: Key
  label: string
  defaultWidth: number
  minWidth: number
}

type WidthMap<Key extends string> = Partial<Record<Key, number>>
type LabelOverrideMap<Key extends string> = Partial<Record<Key, string>>

type DropTarget<Key extends string> = {
  key: Key
  position: "before" | "after"
}

type UseAssetDashboardColumnPrefsOptions<Key extends string> = {
  activeUserScope: string
  storagePrefix: string
  columnMap: Record<Key, ColumnDefinition<Key>>
  defaultOrder: Key[]
  defaultVisibleKeys: Key[]
}

const MAX_COLUMN_WIDTH = 480

export function useAssetDashboardColumnPrefs<Key extends string>({
  activeUserScope,
  storagePrefix,
  columnMap,
  defaultOrder,
  defaultVisibleKeys,
}: UseAssetDashboardColumnPrefsOptions<Key>) {
  const storageKeys = useMemo(
    () => ({
      order: `${storagePrefix}:${activeUserScope}:order`,
      hidden: `${storagePrefix}:${activeUserScope}:hidden`,
      widths: `${storagePrefix}:${activeUserScope}:widths`,
      labels: `${storagePrefix}:${activeUserScope}:labels`,
    }),
    [activeUserScope, storagePrefix],
  )

  const [columnOrder, setColumnOrder] = useState<Key[]>(() =>
    readStoredColumnOrder(storageKeys.order, defaultOrder),
  )
  const [hiddenColumns, setHiddenColumns] = useState<Key[]>(() =>
    readStoredHiddenColumns(storageKeys.hidden, defaultOrder, defaultVisibleKeys),
  )
  const [columnWidths, setColumnWidths] = useState<WidthMap<Key>>(() =>
    readStoredColumnWidths(storageKeys.widths),
  )
  const [labelOverrides, setLabelOverrides] = useState<LabelOverrideMap<Key>>(() =>
    readStoredLabelOverrides(storageKeys.labels),
  )
  const [draggingColumnKey, setDraggingColumnKey] = useState<Key | null>(null)
  const [drawerDraggingColumnKey, setDrawerDraggingColumnKey] = useState<Key | null>(null)
  const [drawerDropTarget, setDrawerDropTarget] = useState<DropTarget<Key> | null>(null)
  const [isColumnsDrawerOpen, setColumnsDrawerOpen] = useState(false)
  const [columnSearchTerm, setColumnSearchTerm] = useState("")
  const [undoResetSnapshot, setUndoResetSnapshot] = useState<{
    order: Key[]
    hidden: Key[]
    widths: WidthMap<Key>
    labels: LabelOverrideMap<Key>
  } | null>(null)
  const [resizeState, setResizeState] = useState<{
    key: Key
    startX: number
    startWidth: number
  } | null>(null)
  const latestColumnWidthsRef = useRef<WidthMap<Key>>(columnWidths)

  useEffect(() => {
    if (typeof window === "undefined") {
      return
    }
    window.localStorage.setItem(storageKeys.order, JSON.stringify(columnOrder))
  }, [columnOrder, storageKeys.order])

  useEffect(() => {
    if (typeof window === "undefined") {
      return
    }
    window.localStorage.setItem(storageKeys.hidden, JSON.stringify(hiddenColumns))
  }, [hiddenColumns, storageKeys.hidden])

  useEffect(() => {
    latestColumnWidthsRef.current = columnWidths
  }, [columnWidths])

  useEffect(() => {
    if (typeof window === "undefined") {
      return
    }
    window.localStorage.setItem(storageKeys.labels, JSON.stringify(labelOverrides))
  }, [labelOverrides, storageKeys.labels])

  useEffect(() => {
    if (!resizeState) {
      return
    }

    const definition = columnMap[resizeState.key]

    const handleMouseMove = (event: MouseEvent) => {
      const delta = event.clientX - resizeState.startX
      const nextWidth = Math.max(
        definition.minWidth,
        Math.min(MAX_COLUMN_WIDTH, Math.round(resizeState.startWidth + delta)),
      )
      setColumnWidths((current) => {
        const nextWidths = {
          ...current,
          [resizeState.key]: nextWidth,
        }
        latestColumnWidthsRef.current = nextWidths
        return nextWidths
      })
    }

    const handleMouseUp = () => {
      if (typeof window !== "undefined") {
        window.localStorage.setItem(
          storageKeys.widths,
          JSON.stringify(latestColumnWidthsRef.current),
        )
      }
      setResizeState(null)
    }

    window.addEventListener("mousemove", handleMouseMove)
    window.addEventListener("mouseup", handleMouseUp)
    document.body.classList.add("column-resize-active")

    return () => {
      window.removeEventListener("mousemove", handleMouseMove)
      window.removeEventListener("mouseup", handleMouseUp)
      document.body.classList.remove("column-resize-active")
    }
  }, [columnMap, resizeState, storageKeys.widths])

  const orderedColumnKeys = useMemo(
    () => reconcileColumnOrder(columnOrder, defaultOrder),
    [columnOrder, defaultOrder],
  )

  const columnLabels = useMemo(() => {
    const labels = {} as Record<Key, string>
    for (const key of defaultOrder) {
      labels[key] = resolveColumnDefinition(columnMap[key], labelOverrides[key]).label
    }
    return labels
  }, [columnMap, defaultOrder, labelOverrides])

  const orderedColumns = useMemo(
    () =>
      orderedColumnKeys
        .filter((key) => !hiddenColumns.includes(key))
        .map((key) => ({ ...columnMap[key], label: columnLabels[key] })),
    [columnLabels, columnMap, hiddenColumns, orderedColumnKeys],
  )

  const filteredColumnKeys = useMemo(() => {
    const keyword = columnSearchTerm.trim().toLowerCase()
    if (!keyword) {
      return orderedColumnKeys
    }

    return orderedColumnKeys.filter((key) => {
      const column = { ...columnMap[key], label: columnLabels[key] }
      return `${column.label} ${column.key}`.toLowerCase().includes(keyword)
    })
  }, [columnLabels, columnMap, columnSearchTerm, orderedColumnKeys])

  const effectiveWidths = useMemo(() => {
    const widths = {} as Record<Key, number>
    for (const key of defaultOrder) {
      const definition = columnMap[key]
      widths[key] = Math.max(
        definition.minWidth,
        Math.min(MAX_COLUMN_WIDTH, Math.round(columnWidths[key] ?? definition.defaultWidth)),
      )
    }
    return widths
  }, [columnMap, columnWidths, defaultOrder])

  const handleHeaderDragStart = (key: Key) => {
    setDraggingColumnKey(key)
  }

  const handleHeaderDrop = (targetKey: Key) => {
    if (!draggingColumnKey || draggingColumnKey === targetKey) {
      setDraggingColumnKey(null)
      return
    }

    setColumnOrder((current) => {
      const base = reconcileColumnOrder(current, defaultOrder)
      const filtered = base.filter((key) => key !== draggingColumnKey)
      const targetIndex = filtered.indexOf(targetKey)
      if (targetIndex === -1) {
        return base
      }
      filtered.splice(targetIndex, 0, draggingColumnKey)
      return filtered
    })
    setDraggingColumnKey(null)
  }

  const toggleColumnVisibility = (key: Key) => {
    setHiddenColumns((current) => {
      const nextHidden = reconcileHiddenColumns(current, defaultOrder)
      const isHidden = nextHidden.includes(key)
      if (isHidden) {
        return nextHidden.filter((item) => item !== key)
      }

      const visibleCount = orderedColumnKeys.filter((item) => !nextHidden.includes(item)).length
      if (visibleCount <= 1) {
        return nextHidden
      }

      return [...nextHidden, key]
    })
  }

  const startDrawerColumnDrag = (key: Key) => {
    setDrawerDraggingColumnKey(key)
    setDrawerDropTarget(null)
  }

  useEffect(() => {
    if (!drawerDraggingColumnKey) {
      return
    }

    document.body.classList.add("column-reorder-active")

    const handlePointerMove = (event: PointerEvent) => {
      const hovered = document.elementFromPoint(event.clientX, event.clientY)
      const row = hovered instanceof Element
        ? hovered.closest<HTMLElement>("[data-asset-dashboard-column-key]")
        : null

      if (!row) {
        setDrawerDropTarget(null)
        return
      }

      const rawKey = row.dataset.assetDashboardColumnKey
      if (!rawKey || rawKey === drawerDraggingColumnKey) {
        setDrawerDropTarget(null)
        return
      }

      const targetKey = rawKey as Key
      const bounds = row.getBoundingClientRect()
      const midpoint = bounds.top + bounds.height / 2
      const position = event.clientY >= midpoint ? "after" : "before"

      setDrawerDropTarget((current) => {
        if (current?.key === targetKey && current.position === position) {
          return current
        }
        return { key: targetKey, position }
      })
    }

    const finishDrag = () => {
      if (drawerDropTarget) {
        setColumnOrder((current) => {
          const nextOrder = reconcileColumnOrder(current, defaultOrder).filter(
            (key) => key !== drawerDraggingColumnKey,
          )
          const targetIndex = nextOrder.indexOf(drawerDropTarget.key)
          if (targetIndex === -1) {
            return current
          }
          const insertionIndex =
            drawerDropTarget.position === "after" ? targetIndex + 1 : targetIndex
          nextOrder.splice(insertionIndex, 0, drawerDraggingColumnKey)
          return nextOrder
        })
      }
      setDrawerDraggingColumnKey(null)
      setDrawerDropTarget(null)
    }

    window.addEventListener("pointermove", handlePointerMove)
    window.addEventListener("pointerup", finishDrag)
    window.addEventListener("pointercancel", finishDrag)

    return () => {
      document.body.classList.remove("column-reorder-active")
      window.removeEventListener("pointermove", handlePointerMove)
      window.removeEventListener("pointerup", finishDrag)
      window.removeEventListener("pointercancel", finishDrag)
    }
  }, [defaultOrder, drawerDraggingColumnKey, drawerDropTarget])

  const resetColumnPreferences = () => {
    setUndoResetSnapshot({
      order: columnOrder,
      hidden: hiddenColumns,
      widths: columnWidths,
      labels: labelOverrides,
    })
    setColumnOrder(defaultOrder.slice())
    setHiddenColumns(buildDefaultHiddenColumns(defaultOrder, defaultVisibleKeys))
    setColumnWidths({})
    setLabelOverrides({})
  }

  const undoResetColumnPreferences = () => {
    if (!undoResetSnapshot) {
      return
    }

    setColumnOrder(undoResetSnapshot.order)
    setHiddenColumns(undoResetSnapshot.hidden)
    setColumnWidths(undoResetSnapshot.widths)
    setLabelOverrides(undoResetSnapshot.labels)
    setUndoResetSnapshot(null)
  }

  const renameColumnLabel = (key: Key, label: string) => {
    const trimmed = label.trim()
    setLabelOverrides((current) => {
      const next = { ...current }
      if (!trimmed || trimmed === columnMap[key].label) {
        delete next[key]
      } else {
        next[key] = trimmed
      }
      return next
    })
  }

  const beginColumnResize = (key: Key, event: ReactMouseEvent<HTMLElement>) => {
    event.preventDefault()
    event.stopPropagation()
    setResizeState({
      key,
      startX: event.clientX,
      startWidth: effectiveWidths[key],
    })
  }

  return {
    orderedColumns,
    orderedColumnKeys,
    columnLabels,
    hiddenColumns,
    filteredColumnKeys,
    effectiveWidths,
    draggingColumnKey,
    drawerDraggingColumnKey,
    drawerDropTarget,
    isColumnsDrawerOpen,
    columnSearchTerm,
    undoResetSnapshot,
    handleHeaderDragStart,
    handleHeaderDrop,
    beginColumnResize,
    toggleColumnVisibility,
    startDrawerColumnDrag,
    resetColumnPreferences,
    undoResetColumnPreferences,
    renameColumnLabel,
    setDraggingColumnKey,
    setColumnsDrawerOpen,
    setColumnSearchTerm,
  }
}

function resolveColumnDefinition<Key extends string>(
  column: ColumnDefinition<Key>,
  labelOverride: string | undefined,
): ColumnDefinition<Key> {
  const label = labelOverride?.trim()
  return label ? { ...column, label } : column
}

function reconcileColumnOrder<Key extends string>(order: Key[], defaultOrder: Key[]): Key[] {
  const unique = order.filter(
    (key, index, values) => defaultOrder.includes(key) && values.indexOf(key) === index,
  )

  for (const key of defaultOrder) {
    if (!unique.includes(key)) {
      unique.push(key)
    }
  }

  return unique
}

function reconcileHiddenColumns<Key extends string>(hiddenColumns: Key[], defaultOrder: Key[]): Key[] {
  return hiddenColumns.filter(
    (key, index, values) => defaultOrder.includes(key) && values.indexOf(key) === index,
  )
}

function buildDefaultHiddenColumns<Key extends string>(defaultOrder: Key[], defaultVisibleKeys: Key[]): Key[] {
  return defaultOrder.filter((key) => !defaultVisibleKeys.includes(key))
}

function readStoredColumnOrder<Key extends string>(storageKey: string, defaultOrder: Key[]): Key[] {
  if (typeof window === "undefined") {
    return defaultOrder.slice()
  }

  const savedOrder = window.localStorage.getItem(storageKey)
  if (!savedOrder) {
    return defaultOrder.slice()
  }

  try {
    return reconcileColumnOrder(JSON.parse(savedOrder) as Key[], defaultOrder)
  } catch {
    return defaultOrder.slice()
  }
}

function readStoredHiddenColumns<Key extends string>(
  storageKey: string,
  defaultOrder: Key[],
  defaultVisibleKeys: Key[],
): Key[] {
  if (typeof window === "undefined") {
    return buildDefaultHiddenColumns(defaultOrder, defaultVisibleKeys)
  }

  const savedHiddenColumns = window.localStorage.getItem(storageKey)
  if (!savedHiddenColumns) {
    return buildDefaultHiddenColumns(defaultOrder, defaultVisibleKeys)
  }

  try {
    return reconcileHiddenColumns(JSON.parse(savedHiddenColumns) as Key[], defaultOrder)
  } catch {
    return buildDefaultHiddenColumns(defaultOrder, defaultVisibleKeys)
  }
}

function readStoredColumnWidths<Key extends string>(storageKey: string): WidthMap<Key> {
  if (typeof window === "undefined") {
    return {}
  }

  const savedWidths = window.localStorage.getItem(storageKey)
  if (!savedWidths) {
    return {}
  }

  try {
    return JSON.parse(savedWidths) as WidthMap<Key>
  } catch {
    return {}
  }
}

function readStoredLabelOverrides<Key extends string>(storageKey: string): LabelOverrideMap<Key> {
  if (typeof window === "undefined") {
    return {}
  }

  try {
    const raw = window.localStorage.getItem(storageKey)
    if (!raw) {
      return {}
    }
    const parsed = JSON.parse(raw) as Record<string, unknown>
    const labels = {} as LabelOverrideMap<Key>
    for (const [key, value] of Object.entries(parsed)) {
      if (typeof value === "string" && value.trim()) {
        labels[key as Key] = value.trim()
      }
    }
    return labels
  } catch {
    return {}
  }
}
