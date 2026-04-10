import { useEffect, useMemo, useRef, useState } from "react"
import type { MouseEvent as ReactMouseEvent } from "react"
import type { AssetDashboardSerializedRecord } from "../../types/staff"
import {
  buildSerializedAssetGridStorageKeys,
  cycleSerializedAssetSort,
  DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER,
  SERIALIZED_ASSET_COLUMN_MAP,
  sortSerializedAssetRows,
  type SerializedAssetColumnKey,
  type SerializedAssetGridSort,
} from "./serializedAssetGridConfig"

type UseSerializedAssetGridStateOptions = {
  activeUserScope: string
  rows: AssetDashboardSerializedRecord[]
}

type WidthMap = Partial<Record<SerializedAssetColumnKey, number>>

const MAX_COLUMN_WIDTH = 480

export function useSerializedAssetGridState({
  activeUserScope,
  rows,
}: UseSerializedAssetGridStateOptions) {
  const storageKeys = useMemo(
    () => buildSerializedAssetGridStorageKeys(activeUserScope),
    [activeUserScope],
  )

  const [columnOrder, setColumnOrder] = useState<SerializedAssetColumnKey[]>(() =>
    readStoredColumnOrder(storageKeys.order),
  )
  const [columnWidths, setColumnWidths] = useState<WidthMap>(() =>
    readStoredColumnWidths(storageKeys.widths),
  )
  const [sort, setSort] = useState<SerializedAssetGridSort>({
    key: null,
    direction: null,
  })
  const [draggingColumnKey, setDraggingColumnKey] =
    useState<SerializedAssetColumnKey | null>(null)
  const [resizeState, setResizeState] = useState<{
    key: SerializedAssetColumnKey
    startX: number
    startWidth: number
  } | null>(null)
  const latestColumnWidthsRef = useRef<WidthMap>(columnWidths)

  useEffect(() => {
    if (typeof window === "undefined") {
      return
    }
    window.localStorage.setItem(storageKeys.order, JSON.stringify(columnOrder))
  }, [columnOrder, storageKeys.order])

  useEffect(() => {
    latestColumnWidthsRef.current = columnWidths
  }, [columnWidths])

  useEffect(() => {
    if (!resizeState) {
      return
    }

    const definition = SERIALIZED_ASSET_COLUMN_MAP[resizeState.key]

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
  }, [resizeState, storageKeys.widths])

  const orderedColumns = useMemo(
    () => reconcileColumnOrder(columnOrder).map((key) => SERIALIZED_ASSET_COLUMN_MAP[key]),
    [columnOrder],
  )

  const effectiveWidths = useMemo(() => {
    const widths: Record<SerializedAssetColumnKey, number> = {} as Record<
      SerializedAssetColumnKey,
      number
    >
    for (const key of DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER) {
      const definition = SERIALIZED_ASSET_COLUMN_MAP[key]
      widths[key] = Math.max(
        definition.minWidth,
        Math.min(MAX_COLUMN_WIDTH, Math.round(columnWidths[key] ?? definition.defaultWidth)),
      )
    }
    return widths
  }, [columnWidths])

  const sortedRows = useMemo(() => sortSerializedAssetRows(rows, sort), [rows, sort])

  const toggleSort = (key: SerializedAssetColumnKey) => {
    setSort((current) => cycleSerializedAssetSort(current, key))
  }

  const handleHeaderDragStart = (key: SerializedAssetColumnKey) => {
    setDraggingColumnKey(key)
  }

  const handleHeaderDrop = (targetKey: SerializedAssetColumnKey) => {
    if (!draggingColumnKey || draggingColumnKey === targetKey) {
      setDraggingColumnKey(null)
      return
    }

    setColumnOrder((current) => {
      const base = reconcileColumnOrder(current)
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

  const beginColumnResize = (
    key: SerializedAssetColumnKey,
    event: ReactMouseEvent<HTMLSpanElement>,
  ) => {
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
    sortedRows,
    effectiveWidths,
    sort,
    draggingColumnKey,
    toggleSort,
    handleHeaderDragStart,
    handleHeaderDrop,
    beginColumnResize,
    setDraggingColumnKey,
  }
}

function reconcileColumnOrder(
  order: SerializedAssetColumnKey[],
): SerializedAssetColumnKey[] {
  const unique = order.filter(
    (key, index, values) =>
      DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER.includes(key) &&
      values.indexOf(key) === index,
  )

  for (const key of DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER) {
    if (!unique.includes(key)) {
      unique.push(key)
    }
  }

  return unique
}

function readStoredColumnOrder(storageKey: string): SerializedAssetColumnKey[] {
  if (typeof window === "undefined") {
    return DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER.slice()
  }

  const savedOrder = window.localStorage.getItem(storageKey)
  if (!savedOrder) {
    return DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER.slice()
  }

  try {
    return reconcileColumnOrder(JSON.parse(savedOrder) as SerializedAssetColumnKey[])
  } catch {
    return DEFAULT_SERIALIZED_ASSET_COLUMN_ORDER.slice()
  }
}

function readStoredColumnWidths(storageKey: string): WidthMap {
  if (typeof window === "undefined") {
    return {}
  }

  const savedWidths = window.localStorage.getItem(storageKey)
  if (!savedWidths) {
    return {}
  }

  try {
    return JSON.parse(savedWidths) as WidthMap
  } catch {
    return {}
  }
}
