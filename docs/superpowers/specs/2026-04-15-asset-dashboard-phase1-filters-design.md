# Asset Dashboard Phase 1 Filters Design

## Goal

Implement Phase 1 of the planning docs by adding serialized-asset filtering to the existing in-app `AssetDashboard` in three ordered slices:

1. search bar
2. category filter
3. clear filters

Each slice must be delivered with test-first coverage before moving to the next slice.

## Current Baseline

The current asset UI already lives in `src/features/assets/AssetDashboard.tsx`.

Relevant existing structure:

- `AssetDashboard.tsx` renders the dashboard shell, actions, summary cards, tabs, and serialized/quantity/category bodies
- `useAssetDashboardState.ts` loads `serializedRows`, `quantityRows`, and `categoryDetails`
- `useSerializedAssetGridState.ts` owns serialized-table sorting, column resize, and drag-reorder behavior
- `serializedAssetGridConfig.ts` already contains the serialized column definitions plus helper display resolvers like `resolveSerializedAssetComputerName` and `resolveSerializedAssetName`

The repo does not currently use a generic asset list abstraction for this screen. Phase 1 should therefore extend the existing dashboard directly instead of introducing a new framework.

## Approved Scope

Phase 1 is limited to the `Serialized` tab only.

It must:

- add a search input above the serialized table
- add a category filter beside the search input
- add a `Clear Filters` action that resets both controls
- keep all existing serialized table behaviors intact after filtering

It must not:

- change backend APIs or database queries
- change `Quantity` tab behavior
- change `Categories` tab behavior
- change import flows or category management flows
- introduce a generic master-data abstraction in this slice

## Chosen Approach

### Recommended approach

Implement the feature as `UI state + pure filter helper`.

That means:

- keep filter state in `AssetDashboard.tsx` / serialized-tab UI
- move row-filtering logic into a small pure helper module
- keep `useSerializedAssetGridState` focused on post-filter table behavior only
- pass filtered rows into the existing serialized grid state hook

This is the smallest correct change because it keeps search/filter behavior easy to test without pushing view-only concerns into backend or table-state code.

### Rejected alternatives

#### 1. Inline all filter logic directly inside `AssetDashboard.tsx`

Rejected because the feature is explicitly being delivered in three test-first slices. A pure helper gives clean behavior-level tests for search and category logic without relying on brittle UI-only assertions.

#### 2. Push filtering into the backend or IPC layer

Rejected because the planning docs for Phase 1 are view-layer only, the serialized dataset is already in memory, and backend changes would expand scope unnecessarily.

#### 3. Create a generic asset list or master-data config layer first

Rejected because the current codebase does not use that abstraction for the asset screen, and introducing it here would turn a focused feature into a refactor project.

## UX Design

The controls sit at the top of the `Serialized` tab body, above the table shell.

### Search

- single text input
- placeholder: `Search computer, asset code, holder, model...`
- applies immediately to serialized rows in memory

The search must match against user-visible asset identity fields, not only raw database fields.

### Category filter

- dropdown beside the search input
- default option: `All Categories`
- option list comes from the current dashboard data, using `categoryDetails` and/or distinct row categories as needed
- no hard-coded category enum for this slice

### Clear Filters

- explicit button beside the other controls
- resets `searchTerm` to empty string
- resets `categoryFilter` to the default all-categories state

## Data And Matching Rules

Filtering runs only against `assetDashboard.serializedRows` already loaded in the client.

The search slice must match across these effective values:

- `assetCode`
- resolved serialized computer name via `resolveSerializedAssetComputerName(assetCode, computerName)`
- resolved serialized asset name via `resolveSerializedAssetName(assetCode, displayName, displayNameShort)`
- `model`
- `serialNumber`
- holder identity assembled from `holderFullName` and `holderEmployeeId`

Matching rules:

- case-insensitive
- trimmed query
- blank query means no search filter
- category filter is applied in addition to search, not instead of it

Category matching rules:

- default state means all rows
- row category display value is `categoryName ?? categoryCode ?? ""`
- exact category selection is sufficient for this slice; no fuzzy category search is needed

## Table Behavior Contract

Filtering must happen before table sorting and rendering.

Required behavior after the slice lands:

- sort still works on the filtered row set
- drag-reorder still works
- width persistence still works
- loading and empty states still render correctly
- empty filtered result should use an explicit empty-state treatment rather than breaking the table shell

`useSerializedAssetGridState` should continue receiving the rows it needs to sort and render, without becoming responsible for owning the filter UI state.

## File-Level Design

### `src/features/assets/AssetDashboard.tsx`

- add serialized-tab filter state
- render search input, category dropdown, and clear button
- compute category options from current dashboard data
- call the pure filter helper before passing rows into `useSerializedAssetGridState`

### `src/features/assets/serializedAssetFilters.ts`

- new pure helper module for filtering serialized rows
- owns normalization and match logic only
- no React state and no DOM code

### `scripts/serialized-asset-filters.test.ts`

- new Node assert-based script test for the helper module
- covers the ordered Phase 1 slices in the same file as the feature grows:
  - search behavior first
  - category behavior second
  - combined/reset expectations third

### Optional UI guard script

If the clear-filter reset wiring is not well covered by helper tests alone, add one focused source-level guard script to assert that the button resets both UI states in `AssetDashboard.tsx`.

This should stay minimal and only be added if needed.

## TDD Execution Order

The implementation must follow this exact order:

### Slice 1: Search

- write the failing helper test for search matches
- verify failure
- implement the minimal search filter logic
- verify pass

### Slice 2: Category filter

- extend tests for category-only and combined filtering
- verify failure
- implement the minimal category filtering logic and UI state
- verify pass

### Slice 3: Clear Filters

- write the failing test covering reset behavior
- verify failure
- implement the minimal reset wiring
- verify pass

Only after all three slices are green should the final quality verification run.

## Verification

Minimum verification for this slice:

- `node --experimental-strip-types scripts/serialized-asset-filters.test.ts`
- any added focused UI/source guard script
- `npm run check:quality`

## Branch And Workspace Strategy

Implementation should happen in a dedicated git worktree and branch, not on the dirty local `main` workspace.

Approved isolation strategy:

- worktree path: `E:\Staff_Kit\.worktrees\asset-dashboard-phase1-filters`
- branch: `phase1-asset-dashboard-filters`
- base: `origin/main`

This keeps Phase 1 isolated from unrelated local changes while still starting from the latest remote baseline.
