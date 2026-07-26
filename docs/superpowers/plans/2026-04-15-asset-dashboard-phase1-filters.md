# Asset Dashboard Phase 1 Filters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Phase 1 serialized-asset filtering to `AssetDashboard` with a search bar, category filter, and clear-filters action, delivered in strict test-first order.

**Architecture:** Keep filtering as client-side view logic on top of the existing `assetDashboard.serializedRows` payload. Put matching logic in a small pure helper module, keep filter UI state in `AssetDashboard.tsx`, and continue using `useSerializedAssetGridState` only for sorting, resize, and column reordering after rows have already been filtered.

**Tech Stack:** React 19, TypeScript, lucide-react, Tailwind utility classes, existing `AssetDashboard` + `useSerializedAssetGridState`, Node assert-based script tests, `npm run check:quality`.

---

## File Map

- Verify/Modify: `src/features/assets/serializedAssetFilters.ts`
  - Confirm the helper matches the approved plan, then extend it for normalized serialized-asset search/category filtering as needed.
- Modify: `src/features/assets/AssetDashboard.tsx`
  - Add serialized-tab filter state, render search/category/clear controls, compute filtered rows before table sorting, and show filtered empty state.
- Verify/Modify: `scripts/serialized-asset-filters.test.ts`
  - Confirm the behavior-level Node test coverage, then extend it for serialized search and category filtering as needed.
- Verify/Modify: `scripts/asset-dashboard-phase1-ui.test.ts`
  - Confirm the source-guard coverage, then extend it for serialized-tab filter UI wiring in `AssetDashboard.tsx` as needed.
- Verify: `scripts/serialized-asset-grid.test.ts`
  - Ensure the existing serialized grid behavior still passes after filter wiring.
- Verify: `scripts/asset-dashboard-formatting.test.ts`
  - Ensure existing asset dashboard formatting helpers still pass.
- Verify: `docs/superpowers/specs/2026-04-15-asset-dashboard-phase1-filters-design.md`
  - Keep implementation aligned to the approved design.

This plan captures the original TDD execution order from the pre-implementation baseline used for Phase 1. In the current repo state, `src/features/assets/serializedAssetFilters.ts`, `scripts/serialized-asset-filters.test.ts`, and `scripts/asset-dashboard-phase1-ui.test.ts` already exist, so any `Create`-style steps below should be interpreted as `Verify/Modify` when replaying the plan against a later checkout.

## Branch And Workspace

- Worktree: `.worktrees/asset-dashboard-phase1-filters`
- Branch: `phase1-asset-dashboard-filters`
- Base: `origin/main`

All commands below should run from the worktree root unless a later task explicitly says otherwise.

### Task 1: Add Search Filter With Test-First Coverage

**Files:**
- Verify/Modify: `src/features/assets/serializedAssetFilters.ts`
- Verify/Modify: `scripts/serialized-asset-filters.test.ts`
- Verify/Modify: `scripts/asset-dashboard-phase1-ui.test.ts`
- Modify: `src/features/assets/AssetDashboard.tsx`

- [ ] **Step 1: Write the failing search-helper test**

Ensure `scripts/serialized-asset-filters.test.ts` matches this baseline red-state content:

```ts
import assert from "node:assert/strict"

import {
  ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  filterSerializedAssetRows,
} from "../src/features/assets/serializedAssetFilters.ts"
import type { AssetDashboardSerializedRecord } from "../src/types/staff.ts"

const rows: AssetDashboardSerializedRecord[] = [
  {
    assetId: 1,
    assetCode: "VNLAP235",
    categoryCode: "laptop",
    categoryName: "Laptop",
    computerName: "ASWVNLAP235",
    displayName: "Dell Latitude 3520",
    displayNameShort: null,
    model: "Dell Latitude 3520",
    serialNumber: "SN-235",
    adapterNumber: null,
    usageLocation: null,
    notes: null,
    status: "assigned",
    holderEmployeeId: "ASWVN1302",
    holderFullName: "Le The Hung",
  },
  {
    assetId: 2,
    assetCode: "VNMON709",
    categoryCode: "monitor",
    categoryName: "Monitor",
    computerName: null,
    displayName: "Mon709",
    displayNameShort: "Mon709",
    model: "LG 27",
    serialNumber: null,
    adapterNumber: null,
    usageLocation: "office",
    notes: "Window desk",
    status: "in_stock",
    holderEmployeeId: null,
    holderFullName: null,
  },
]

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "vnlap235",
    categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  }).map((row) => row.assetCode),
  ["VNLAP235"],
)

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "  aswvnlap235  ",
    categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  }).map((row) => row.assetCode),
  ["VNLAP235"],
)

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "hung",
    categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  }).map((row) => row.assetCode),
  ["VNLAP235"],
)

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "lg 27",
    categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  }).map((row) => row.assetCode),
  ["VNMON709"],
)

console.log("serialized-asset-filters tests passed")
```

- [ ] **Step 2: Run the helper test to verify it fails**

Run:

```powershell
node --experimental-strip-types scripts/serialized-asset-filters.test.ts
```

Expected: On the baseline assumed by this plan, FAIL with a module-resolution error because `src/features/assets/serializedAssetFilters.ts` has not been implemented yet. If that module already exists in the checkout you are using, this exact failure will not reproduce; treat that as a baseline mismatch and continue using the repo's current state.

- [ ] **Step 3: Write the failing search UI guard**

Ensure `scripts/asset-dashboard-phase1-ui.test.ts` matches this baseline red-state content:

```ts
import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const assetDashboardSource = readFileSync("src/features/assets/AssetDashboard.tsx", "utf8")

assert.match(
  assetDashboardSource,
  /placeholder="Search computer, asset code, holder, model\.\.\."/,
)
assert.match(assetDashboardSource, /filterSerializedAssetRows\(/)
assert.match(assetDashboardSource, /rows:\s*filteredRows/)
assert.match(assetDashboardSource, /No serialized assets match the current filters\./)

console.log("asset-dashboard-phase1-ui tests passed")
```

- [ ] **Step 4: Run the UI guard to verify it fails**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-phase1-ui.test.ts
```

Expected: On the baseline assumed by this plan, FAIL because the search input, filtered row wiring, and filtered empty-state copy have not been implemented yet. If the current checkout already contains those changes, this exact failure will not reproduce; treat that as a baseline mismatch and continue from the repo's current state.

- [ ] **Step 5: Write the minimal serialized search helper**

Ensure `src/features/assets/serializedAssetFilters.ts` matches this baseline helper implementation:

```ts
import type { AssetDashboardSerializedRecord } from "../../types/staff"
import {
  resolveSerializedAssetComputerName,
  resolveSerializedAssetName,
} from "./serializedAssetGridConfig"

export const ALL_SERIALIZED_ASSET_CATEGORY_FILTER = "__all__"

export type SerializedAssetFilterState = {
  searchTerm: string
  categoryFilter: string
}

export function filterSerializedAssetRows(
  rows: AssetDashboardSerializedRecord[],
  filters: SerializedAssetFilterState,
): AssetDashboardSerializedRecord[] {
  const normalizedSearch = normalizeSerializedAssetFilterText(filters.searchTerm)
  if (!normalizedSearch) {
    return rows
  }

  return rows.filter((row) => buildSerializedAssetSearchText(row).includes(normalizedSearch))
}

export function normalizeSerializedAssetFilterText(
  value: string | null | undefined,
): string {
  return value?.trim().toLowerCase() ?? ""
}

function buildSerializedAssetSearchText(row: AssetDashboardSerializedRecord): string {
  return [
    row.assetCode,
    resolveSerializedAssetComputerName(row.assetCode, row.computerName),
    resolveSerializedAssetName(row.assetCode, row.displayName, row.displayNameShort),
    row.model,
    row.serialNumber,
    row.holderFullName,
    row.holderEmployeeId,
  ]
    .map((value) => normalizeSerializedAssetFilterText(value))
    .filter(Boolean)
    .join(" ")
}
```

- [ ] **Step 6: Wire search filtering into `AssetDashboard.tsx`**

Update the imports at the top of `src/features/assets/AssetDashboard.tsx` like this:

```ts
import {
  ArrowUpDown,
  Boxes,
  FileSpreadsheet,
  GripVertical,
  LoaderCircle,
  PencilLine,
  PlusCircle,
  RefreshCw,
  Search,
  Trash2,
} from "lucide-react"
```

Add this import block near the existing serialized-grid imports:

```ts
import {
  ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  filterSerializedAssetRows,
} from "./serializedAssetFilters"
```

Add this state near the other top-level `useState` calls in `AssetDashboard`:

```ts
const [serializedSearchTerm, setSerializedSearchTerm] = useState("")
```

Change the serialized-tab render call to pass search state:

```tsx
{activeTab === "serialized" ? (
  <SerializedDashboardTable
    activeUserScope={activeUserScope}
    assetDashboard={assetDashboard}
    searchTerm={serializedSearchTerm}
    onSearchTermChange={setSerializedSearchTerm}
  />
) : activeTab === "quantity" ? (
```

Change the `SerializedDashboardTable` signature to accept those props:

```ts
function SerializedDashboardTable({
  activeUserScope,
  assetDashboard,
  searchTerm,
  onSearchTermChange,
}: {
  activeUserScope: string
  assetDashboard: AssetDashboardState
  searchTerm: string
  onSearchTermChange: (value: string) => void
}) {
```

Inside `SerializedDashboardTable`, add `filteredRows` before the `useSerializedAssetGridState` call:

```ts
const filteredRows = useMemo(
  () =>
    filterSerializedAssetRows(assetDashboard.serializedRows, {
      searchTerm,
      categoryFilter: ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
    }),
  [assetDashboard.serializedRows, searchTerm],
)
```

Change the grid hook input from `assetDashboard.serializedRows` to `filteredRows`:

```ts
  } = useSerializedAssetGridState({
    activeUserScope,
    rows: filteredRows,
  })
```

Insert the search bar and filtered empty state immediately before the table shell return:

```tsx
  if (filteredRows.length === 0) {
    return (
      <div className="space-y-3">
        <label className="relative block">
          <Search
            size={15}
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[#8f98a8]"
          />
          <input
            className={`${dashboardInputClass} pl-9`}
            onChange={(event) => onSearchTermChange(event.target.value)}
            placeholder="Search computer, asset code, holder, model..."
            type="text"
            value={searchTerm}
          />
        </label>
        <div className="rounded-[12px] border border-dashed border-[#31394a] bg-[#0b0f15] px-4 py-8 text-center text-sm text-[#8f98a8]">
          No serialized assets match the current filters.
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <label className="relative block">
        <Search
          size={15}
          className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[#8f98a8]"
        />
        <input
          className={`${dashboardInputClass} pl-9`}
          onChange={(event) => onSearchTermChange(event.target.value)}
          placeholder="Search computer, asset code, holder, model..."
          type="text"
          value={searchTerm}
        />
      </label>
      <div className={dashboardTableShellClass}>
```

Keep the rest of the table body unchanged inside that new wrapper.

- [ ] **Step 7: Run the helper test to verify it passes**

Run:

```powershell
node --experimental-strip-types scripts/serialized-asset-filters.test.ts
```

Expected: PASS with `serialized-asset-filters tests passed`.

- [ ] **Step 8: Run the UI guard to verify it passes**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-phase1-ui.test.ts
```

Expected: PASS with `asset-dashboard-phase1-ui tests passed`.

- [ ] **Step 9: Commit the search slice**

Run:

```powershell
git add src/features/assets/serializedAssetFilters.ts src/features/assets/AssetDashboard.tsx scripts/serialized-asset-filters.test.ts scripts/asset-dashboard-phase1-ui.test.ts
git commit -m "feat: add serialized asset search filter"
```

### Task 2: Add Category Filter With Test-First Coverage

**Files:**
- Modify: `src/features/assets/serializedAssetFilters.ts`
- Modify: `src/features/assets/AssetDashboard.tsx`
- Modify: `scripts/serialized-asset-filters.test.ts`
- Modify: `scripts/asset-dashboard-phase1-ui.test.ts`

- [ ] **Step 1: Extend the helper test with category assertions**

Append these assertions before the final `console.log(...)` in `scripts/serialized-asset-filters.test.ts`:

```ts
assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "",
    categoryFilter: "laptop",
  }).map((row) => row.assetCode),
  ["VNLAP235"],
)

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "",
    categoryFilter: "monitor",
  }).map((row) => row.assetCode),
  ["VNMON709"],
)

assert.deepEqual(
  filterSerializedAssetRows(rows, {
    searchTerm: "hung",
    categoryFilter: "monitor",
  }).map((row) => row.assetCode),
  [],
)
```

- [ ] **Step 2: Run the helper test to verify it fails**

Run:

```powershell
node --experimental-strip-types scripts/serialized-asset-filters.test.ts
```

Expected: On the baseline assumed by this task, FAIL because the current helper still ignores any category filter other than search behavior. If the checkout already contains later category-filter work, this exact failure will not reproduce; treat that as a baseline mismatch and continue from the repo's current state.

- [ ] **Step 3: Extend the UI guard with category-filter assertions**

Append these assertions before the final `console.log(...)` in `scripts/asset-dashboard-phase1-ui.test.ts`:

```ts
assert.match(assetDashboardSource, /All Categories/)
assert.match(assetDashboardSource, /value=\{categoryFilter\}/)
assert.match(assetDashboardSource, /onCategoryFilterChange\(event\.target\.value\)/)
assert.match(assetDashboardSource, /categoryOptions\.map\(\(option\) => \(/)
```

- [ ] **Step 4: Run the UI guard to verify it fails**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-phase1-ui.test.ts
```

Expected: On the baseline assumed by this task, FAIL because the category dropdown and its prop wiring have not been implemented yet. If the checkout already contains those changes, this exact failure will not reproduce; treat that as a baseline mismatch and continue from the repo's current state.

- [ ] **Step 5: Implement category filtering in the helper and dashboard UI**

Replace `filterSerializedAssetRows(...)` in `src/features/assets/serializedAssetFilters.ts` with this version:

```ts
export function filterSerializedAssetRows(
  rows: AssetDashboardSerializedRecord[],
  filters: SerializedAssetFilterState,
): AssetDashboardSerializedRecord[] {
  const normalizedSearch = normalizeSerializedAssetFilterText(filters.searchTerm)
  const normalizedCategory = normalizeSerializedAssetFilterText(filters.categoryFilter)

  return rows.filter((row) => {
    if (
      normalizedCategory &&
      normalizedCategory !== ALL_SERIALIZED_ASSET_CATEGORY_FILTER
    ) {
      const rowCategory = normalizeSerializedAssetFilterText(row.categoryCode)
      if (rowCategory !== normalizedCategory) {
        return false
      }
    }

    if (!normalizedSearch) {
      return true
    }

    return buildSerializedAssetSearchText(row).includes(normalizedSearch)
  })
}
```

Update the helper import block in `AssetDashboard.tsx` like this:

```ts
import {
  ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
  filterSerializedAssetRows,
  normalizeSerializedAssetFilterText,
} from "./serializedAssetFilters"
```

Add category state in `AssetDashboard` next to the search state:

```ts
const [serializedCategoryFilter, setSerializedCategoryFilter] = useState(
  ALL_SERIALIZED_ASSET_CATEGORY_FILTER,
)
```

Pass that state into `SerializedDashboardTable`:

```tsx
<SerializedDashboardTable
  activeUserScope={activeUserScope}
  assetDashboard={assetDashboard}
  searchTerm={serializedSearchTerm}
  onSearchTermChange={setSerializedSearchTerm}
  categoryFilter={serializedCategoryFilter}
  onCategoryFilterChange={setSerializedCategoryFilter}
/>
```

Update the `SerializedDashboardTable` signature to accept the new props:

```ts
function SerializedDashboardTable({
  activeUserScope,
  assetDashboard,
  searchTerm,
  onSearchTermChange,
  categoryFilter,
  onCategoryFilterChange,
}: {
  activeUserScope: string
  assetDashboard: AssetDashboardState
  searchTerm: string
  onSearchTermChange: (value: string) => void
  categoryFilter: string
  onCategoryFilterChange: (value: string) => void
}) {
```

Update `filteredRows` so it uses the real category state:

```ts
const filteredRows = useMemo(
  () =>
    filterSerializedAssetRows(assetDashboard.serializedRows, {
      searchTerm,
      categoryFilter,
    }),
  [assetDashboard.serializedRows, categoryFilter, searchTerm],
)
```

Add category options above the filtered-row computation:

```ts
const categoryOptions = useMemo(() => {
  const options = new Map<string, string>([
    [ALL_SERIALIZED_ASSET_CATEGORY_FILTER, "All Categories"],
  ])

  for (const detail of assetDashboard.categoryDetails) {
    if (detail.trackingMode !== "serialized") {
      continue
    }

    const label = detail.categoryName.trim() || detail.categoryCode.trim()
    const value = normalizeSerializedAssetFilterText(detail.categoryCode)
    if (value && !options.has(value)) {
      options.set(value, label)
    }
  }

  for (const row of assetDashboard.serializedRows) {
    const label = (row.categoryName ?? row.categoryCode ?? "").trim()
    const value = normalizeSerializedAssetFilterText(row.categoryCode)
    if (value && !options.has(value)) {
      options.set(value, label)
    }
  }

  return Array.from(options, ([value, label]) => ({ value, label }))
}, [assetDashboard.categoryDetails, assetDashboard.serializedRows])
```

Replace the single-control search block in both the filtered-empty branch and the main return branch with this shared two-control row:

```tsx
<div className="flex flex-col gap-3 md:flex-row md:items-center">
  <label className="relative flex-1">
    <Search
      size={15}
      className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[#8f98a8]"
    />
    <input
      className={`${dashboardInputClass} pl-9`}
      onChange={(event) => onSearchTermChange(event.target.value)}
      placeholder="Search computer, asset code, holder, model..."
      type="text"
      value={searchTerm}
    />
  </label>
  <select
    className={`${dashboardInputClass} md:w-[220px]`}
    onChange={(event) => onCategoryFilterChange(event.target.value)}
    value={categoryFilter}
  >
    {categoryOptions.map((option) => (
      <option key={option.value} value={option.value}>
        {option.label}
      </option>
    ))}
  </select>
</div>
```

- [ ] **Step 6: Run the helper test to verify it passes**

Run:

```powershell
node --experimental-strip-types scripts/serialized-asset-filters.test.ts
```

Expected: PASS with `serialized-asset-filters tests passed`.

- [ ] **Step 7: Run the UI guard to verify it passes**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-phase1-ui.test.ts
```

Expected: PASS with `asset-dashboard-phase1-ui tests passed`.

- [ ] **Step 8: Commit the category-filter slice**

Run:

```powershell
git add src/features/assets/serializedAssetFilters.ts src/features/assets/AssetDashboard.tsx scripts/serialized-asset-filters.test.ts scripts/asset-dashboard-phase1-ui.test.ts
git commit -m "feat: add serialized asset category filter"
```

### Task 3: Add Clear Filters With Test-First Coverage

**Files:**
- Modify: `src/features/assets/AssetDashboard.tsx`
- Modify: `scripts/asset-dashboard-phase1-ui.test.ts`

- [ ] **Step 1: Extend the UI guard with clear-filter assertions**

Append these assertions before the final `console.log(...)` in `scripts/asset-dashboard-phase1-ui.test.ts`:

```ts
assert.match(assetDashboardSource, />\s*Clear Filters\s*</)
assert.match(assetDashboardSource, /setSerializedSearchTerm\(""\)/)
assert.match(
  assetDashboardSource,
  /setSerializedCategoryFilter\(ALL_SERIALIZED_ASSET_CATEGORY_FILTER\)/,
)
assert.match(assetDashboardSource, /onClearFilters=\{clearSerializedFilters\}/)
```

- [ ] **Step 2: Run the UI guard to verify it fails**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-phase1-ui.test.ts
```

Expected: On the baseline assumed by this task, FAIL because the clear-filters button and reset handler have not been implemented yet. If the checkout already contains those changes, this exact failure will not reproduce; treat that as a baseline mismatch and continue from the repo's current state.

- [ ] **Step 3: Implement the clear-filters reset wiring**

In `AssetDashboard`, add this handler near the other local action helpers:

```ts
const clearSerializedFilters = () => {
  setSerializedSearchTerm("")
  setSerializedCategoryFilter(ALL_SERIALIZED_ASSET_CATEGORY_FILTER)
}
```

Pass the handler into `SerializedDashboardTable`:

```tsx
<SerializedDashboardTable
  activeUserScope={activeUserScope}
  assetDashboard={assetDashboard}
  searchTerm={serializedSearchTerm}
  onSearchTermChange={setSerializedSearchTerm}
  categoryFilter={serializedCategoryFilter}
  onCategoryFilterChange={setSerializedCategoryFilter}
  onClearFilters={clearSerializedFilters}
/>
```

Update the `SerializedDashboardTable` signature to accept the callback:

```ts
function SerializedDashboardTable({
  activeUserScope,
  assetDashboard,
  searchTerm,
  onSearchTermChange,
  categoryFilter,
  onCategoryFilterChange,
  onClearFilters,
}: {
  activeUserScope: string
  assetDashboard: AssetDashboardState
  searchTerm: string
  onSearchTermChange: (value: string) => void
  categoryFilter: string
  onCategoryFilterChange: (value: string) => void
  onClearFilters: () => void
}) {
```

Add the clear button to the filter-control row in both the filtered-empty branch and the main branch:

```tsx
<button
  className={dashboardSecondaryButtonClass}
  onClick={onClearFilters}
  type="button"
>
  Clear Filters
</button>
```

- [ ] **Step 4: Run the UI guard to verify it passes**

Run:

```powershell
node --experimental-strip-types scripts/asset-dashboard-phase1-ui.test.ts
```

Expected: PASS with `asset-dashboard-phase1-ui tests passed`.

- [ ] **Step 5: Commit the clear-filters slice**

Run:

```powershell
git add src/features/assets/AssetDashboard.tsx scripts/asset-dashboard-phase1-ui.test.ts
git commit -m "feat: add serialized asset clear filters"
```

### Task 4: Final Verification And Publish Branch

**Files:**
- Verify: `src/features/assets/serializedAssetFilters.ts`
- Verify: `src/features/assets/AssetDashboard.tsx`
- Verify: `scripts/serialized-asset-filters.test.ts`
- Verify: `scripts/asset-dashboard-phase1-ui.test.ts`

- [ ] **Step 1: Run the focused Phase 1 test scripts**

Run:

```powershell
node --experimental-strip-types scripts/serialized-asset-filters.test.ts
node --experimental-strip-types scripts/asset-dashboard-phase1-ui.test.ts
node --experimental-strip-types scripts/serialized-asset-grid.test.ts
node --experimental-strip-types scripts/asset-dashboard-formatting.test.ts
```

Expected: PASS for all four scripts.

- [ ] **Step 2: Run the full quality check**

Run:

```powershell
npm run check:quality
```

Expected: PASS, including lint, typecheck, build, and tauri cargo check.

- [ ] **Step 3: Push the Phase 1 branch**

Run:

```powershell
git push -u origin phase1-asset-dashboard-filters
```

Expected: remote branch created or updated successfully.
