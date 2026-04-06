# Assets CRUD + Preload UI Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first real `/assets` admin workstation with search/filter, full create/edit modal, and CSV/XLSX preload upload with preview and summary.

**Architecture:** Keep `/assets` as a server-rendered workstation shell, but split interactive pieces into focused client components for table controls, asset form modal, and preload modal. Reuse the existing asset API/service layer, extend validation where business rules are missing, and drive the implementation test-first from service parsing and UI smoke components.

**Tech Stack:** Next.js 16 App Router, React 19, TypeScript, Prisma 7, PostgreSQL 16, Zod 4, Vitest 3.

---

## Chunk 1: Asset Contracts and Parsing

### Task 1: Add missing asset form and preload validation rules

**Files:**
- Create: `web/tests/unit/assets.schemas.test.ts`
- Modify: `web/src/lib/assets/assets.schemas.ts`

- [ ] **Step 1: Write the failing schema tests**

```ts
it("rejects retiredAt when status is not RETIRED", () => {
  expect(() => assetCreateSchema.parse({
    name: "Laptop",
    assetType: "Laptop",
    status: "IN_STOCK",
    retiredAt: "2026-03-12",
  })).toThrow();
});

it("rejects disposedAt when status is not DISPOSED", () => {
  expect(() => assetUpdateSchema.parse({
    status: "RETIRED",
    disposedAt: "2026-03-12",
  })).toThrow();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- tests/unit/assets.schemas.test.ts`
Expected: FAIL because `retiredAt` and `disposedAt` are not handled yet.

- [ ] **Step 3: Extend asset schemas with the full form fields**

```ts
retiredAt: z.coerce.date().optional(),
disposedAt: z.coerce.date().optional(),
```

Add `.superRefine(...)` rules so the date fields are only valid for their matching status.

- [ ] **Step 4: Run the schema test again**

Run: `npm run test -- tests/unit/assets.schemas.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add web/tests/unit/assets.schemas.test.ts web/src/lib/assets/assets.schemas.ts
git commit -m "feat: validate full asset form fields"
```

### Task 2: Add CSV/XLSX preload parser and preview summary

**Files:**
- Create: `web/tests/unit/assets.preload-parser.test.ts`
- Create: `web/src/lib/assets/assets-preload-parser.ts`
- Modify: `web/package.json`

- [ ] **Step 1: Write the failing parser tests**

```ts
it("parses a CSV file into preload asset rows", async () => {
  const file = new File(
    ["assetCode,name,assetType,status\nAST-1,Latitude,Laptop,IN_STOCK"],
    "assets.csv",
    { type: "text/csv" },
  );
  const result = await parseAssetPreloadFile(file);
  expect(result.validRows).toHaveLength(1);
});

it("rejects duplicate asset codes within the same upload", async () => {
  // same assetCode twice in one file
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- tests/unit/assets.preload-parser.test.ts`
Expected: FAIL because the parser module does not exist yet.

- [ ] **Step 3: Add minimal parser implementation**

```ts
export async function parseAssetPreloadFile(file: File) {
  // detect csv/xlsx
  // read rows
  // normalize headers
  // validate rows with assetPreloadSchema-compatible shape
}
```

Use one lightweight client-safe parser library for CSV/XLSX handling and return:
- `rows`
- `validRows`
- `invalidRows`
- `summary`

- [ ] **Step 4: Run the parser test again**

Run: `npm run test -- tests/unit/assets.preload-parser.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add web/tests/unit/assets.preload-parser.test.ts web/src/lib/assets/assets-preload-parser.ts web/package.json web/package-lock.json
git commit -m "feat: add asset preload file parser"
```

## Chunk 2: Asset Workstation UI

### Task 3: Add server actions for asset create, update, and preload

**Files:**
- Create: `web/src/app/(protected)/assets/actions.ts`
- Create: `web/tests/unit/assets.actions.test.ts`
- Modify: `web/src/lib/assets/assets.service.ts`
- Modify: `web/src/lib/assets/assets.schemas.ts`

- [ ] **Step 1: Write the failing action tests**

```ts
it("creates an asset from full-form input", async () => {
  // submit form payload and expect asset.create path
});

it("returns validation error for invalid retired/disposed dates", async () => {
  // bad status/date combination
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- tests/unit/assets.actions.test.ts`
Expected: FAIL because no asset server action layer exists yet.

- [ ] **Step 3: Implement minimal actions**

```ts
export async function createAssetAction(formData: FormData) {}
export async function updateAssetAction(formData: FormData) {}
export async function preloadAssetsAction(formData: FormData) {}
```

Parse form data, call service functions, and `revalidatePath("/assets")`.

- [ ] **Step 4: Run the action test again**

Run: `npm run test -- tests/unit/assets.actions.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add web/src/app/(protected)/assets/actions.ts web/tests/unit/assets.actions.test.ts web/src/lib/assets/assets.service.ts web/src/lib/assets/assets.schemas.ts
git commit -m "feat: add asset workstation actions"
```

### Task 4: Replace the preview page with a real asset list workstation

**Files:**
- Create: `web/src/components/assets/AssetsPageShell.tsx`
- Create: `web/src/components/assets/AssetsTable.tsx`
- Create: `web/src/components/assets/AssetsFilterBar.tsx`
- Create: `web/tests/unit/assets.page.test.tsx`
- Modify: `web/src/app/(protected)/assets/page.tsx`
- Modify: `web/src/lib/admin/admin.service.ts`

- [ ] **Step 1: Write the failing workstation UI test**

```tsx
it("renders the asset workstation with filter controls and primary actions", () => {
  const html = renderToStaticMarkup(<AssetsPageShell ... />);
  expect(html).toContain("New Asset");
  expect(html).toContain("Preload Assets");
  expect(html).toContain("Current Holder");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- tests/unit/assets.page.test.tsx`
Expected: FAIL because workstation components do not exist yet.

- [ ] **Step 3: Implement minimal page shell and table**

```tsx
<AssetsFilterBar />
<AssetsTable assets={assets} />
```

Add:
- search input
- status filter
- asset type filter
- row-level `Edit` action
- primary buttons for `New Asset` and `Preload Assets`

- [ ] **Step 4: Run the workstation UI test again**

Run: `npm run test -- tests/unit/assets.page.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add web/src/components/assets web/src/app/(protected)/assets/page.tsx web/src/lib/admin/admin.service.ts web/tests/unit/assets.page.test.tsx
git commit -m "feat: add assets workstation list ui"
```

## Chunk 3: Create/Edit and Preload Modals

### Task 5: Build the full asset create/edit modal

**Files:**
- Create: `web/src/components/assets/AssetFormModal.tsx`
- Create: `web/tests/unit/asset-form-modal.test.tsx`
- Modify: `web/src/components/assets/AssetsPageShell.tsx`

- [ ] **Step 1: Write the failing modal UI test**

```tsx
it("renders full asset fields including retiredAt and disposedAt", () => {
  const html = renderToStaticMarkup(<AssetFormModal mode="create" ... />);
  expect(html).toContain("Serial Number");
  expect(html).toContain("Retired At");
  expect(html).toContain("Disposed At");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- tests/unit/asset-form-modal.test.tsx`
Expected: FAIL because the modal does not exist yet.

- [ ] **Step 3: Implement minimal create/edit modal**

```tsx
<dialog>
  <form>
    {/* full field set */}
  </form>
</dialog>
```

Ensure:
- `assetCode` is editable on create and read-only on edit
- `retiredAt` only active for `RETIRED`
- `disposedAt` only active for `DISPOSED`

- [ ] **Step 4: Run the modal UI test again**

Run: `npm run test -- tests/unit/asset-form-modal.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add web/src/components/assets/AssetFormModal.tsx web/src/components/assets/AssetsPageShell.tsx web/tests/unit/asset-form-modal.test.tsx
git commit -m "feat: add full asset form modal"
```

### Task 6: Build the preload upload modal with preview summary

**Files:**
- Create: `web/src/components/assets/AssetPreloadModal.tsx`
- Create: `web/tests/unit/asset-preload-modal.test.tsx`
- Modify: `web/src/components/assets/AssetsPageShell.tsx`

- [ ] **Step 1: Write the failing preload modal test**

```tsx
it("renders preload upload controls and validation summary", () => {
  const html = renderToStaticMarkup(<AssetPreloadModal ... />);
  expect(html).toContain("Upload CSV or XLSX");
  expect(html).toContain("Valid rows");
  expect(html).toContain("Invalid rows");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- tests/unit/asset-preload-modal.test.tsx`
Expected: FAIL because the modal does not exist yet.

- [ ] **Step 3: Implement minimal preload modal**

```tsx
<input type="file" accept=".csv,.xlsx" />
```

Show:
- parse summary
- invalid rows list
- submit button only when all rows are valid

- [ ] **Step 4: Run the preload modal test again**

Run: `npm run test -- tests/unit/asset-preload-modal.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add web/src/components/assets/AssetPreloadModal.tsx web/src/components/assets/AssetsPageShell.tsx web/tests/unit/asset-preload-modal.test.tsx
git commit -m "feat: add asset preload modal"
```

## Chunk 4: Verification

### Task 7: Run full verification and browser smoke

**Files:**
- No new files unless bug fixes are needed.

- [ ] **Step 1: Run full project verification**

Run: `npm run check`
Expected: PASS

- [ ] **Step 2: Start a local dev server for the branch**

Run: `npm run dev -- --port 3001`
Expected: app is reachable on `http://localhost:3001`

- [ ] **Step 3: Smoke the `/assets` flow in a real browser**

Manual or Playwright smoke:
- login as `adman`
- open `/assets`
- create an asset
- edit an existing asset
- upload one preload file
- verify summary and refreshed list

Expected:
- all mutations succeed
- `/assets` reflects new data

- [ ] **Step 4: Commit any bug-fix cleanup**

```bash
git add <files>
git commit -m "fix: polish asset workstation flow"
```
