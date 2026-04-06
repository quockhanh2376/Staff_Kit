# Pending Reviews Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `Pending Reviews` into an actionable IT review queue with request detail, limited inline corrections, approve/reject actions, and audit-safe review logging.

**Architecture:** Keep request submission immutable and treat review as a separate decision payload. Extend workflow services to accept reviewed values at decision time, then add a protected review detail page plus server actions that call the same service layer used by the API. Use unit tests first for receive/return review logic before wiring UI.

**Tech Stack:** Next.js 16 App Router, React 19, TypeScript, Prisma 7, PostgreSQL 16, Zod 4, Vitest 3.

---

## Chunk 1: Workflow Review Domain

### Task 1: Extend review input contracts and receive-review service behavior

**Files:**
- Create: `web/tests/unit/workflows.review-receive.test.ts`
- Modify: `web/src/lib/workflows/workflows.schemas.ts`
- Modify: `web/src/lib/workflows/workflows.service.ts`
- Modify: `web/src/lib/workflows/workflows.repository.ts`

- [ ] **Step 1: Write the failing receive-review tests**

```ts
test("approves a receive request with corrected employee and reviewed asset codes", async () => {
  // seed-backed requestKey + actor fixture
  // expect returned status APPROVED and updated requestKey
});

test("rejects a receive review when reviewed asset codes contain duplicates", async () => {
  // expect ApiError 400 duplicate_asset_codes
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- web/tests/unit/workflows.review-receive.test.ts`
Expected: FAIL because reviewed payload fields are not accepted yet.

- [ ] **Step 3: Extend review schema with reviewed payload**

```ts
export const reviewPendingRequestSchema = z.object({
  requestType: z.enum(["RECEIVE", "RETURN"]),
  requestKey: trimmedRequiredText,
  decision: z.enum(["APPROVED", "REJECTED"]),
  notes: trimmedOptionalText,
  reviewedEmployeeId: trimmedRequiredText.optional(),
  reviewedAssetCodes: z.array(trimmedRequiredText).max(50).optional(),
});
```

- [ ] **Step 4: Implement minimal receive review support**

```ts
const reviewedEmployeeId = input.reviewedEmployeeId ?? request.employee.employeeId;
const reviewedAssetCodes = input.reviewedAssetCodes ?? request.items.map((item) => item.assetCodeSnapshot);
```

Use the reviewed values for validation, assignment creation, and audit metadata while preserving the submitted snapshot.

- [ ] **Step 5: Run the receive-review test file again**

Run: `npm run test -- web/tests/unit/workflows.review-receive.test.ts`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add web/tests/unit/workflows.review-receive.test.ts web/src/lib/workflows/workflows.schemas.ts web/src/lib/workflows/workflows.service.ts web/src/lib/workflows/workflows.repository.ts
git commit -m "feat: support reviewed receive approvals"
```

### Task 2: Extend return-review service behavior and enforce restricted edits

**Files:**
- Create: `web/tests/unit/workflows.review-return.test.ts`
- Modify: `web/src/lib/workflows/workflows.service.ts`
- Modify: `web/src/lib/workflows/workflows.repository.ts`

- [ ] **Step 1: Write the failing return-review tests**

```ts
test("approves a return request after removing an invalid reviewed asset", async () => {
  // reviewedAssetCodes removes one submitted item
});

test("rejects a return review that tries to add a new asset code", async () => {
  // expect ApiError 409 return_review_asset_add_not_allowed
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- web/tests/unit/workflows.review-return.test.ts`
Expected: FAIL because return review currently always uses submitted items.

- [ ] **Step 3: Implement minimal return review support**

```ts
const submittedCodes = new Set(request.items.map((item) => item.assetCodeSnapshot));
const reviewedCodes = input.reviewedAssetCodes ?? Array.from(submittedCodes);

for (const code of reviewedCodes) {
  if (!submittedCodes.has(code)) throw new ApiError(409, "return_review_asset_add_not_allowed", ...);
}
```

Allow removals only, block empty reviewed sets, and use the reviewed subset when closing assignments and writing audit metadata.

- [ ] **Step 4: Run the return-review test file again**

Run: `npm run test -- web/tests/unit/workflows.review-return.test.ts`
Expected: PASS

- [ ] **Step 5: Run both workflow review test files**

Run: `npm run test -- web/tests/unit/workflows.review-receive.test.ts web/tests/unit/workflows.review-return.test.ts`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add web/tests/unit/workflows.review-return.test.ts web/src/lib/workflows/workflows.service.ts web/src/lib/workflows/workflows.repository.ts
git commit -m "feat: support reviewed return approvals"
```

## Chunk 2: Review Detail UI

### Task 3: Add review detail query and server actions

**Files:**
- Create: `web/src/app/(protected)/reviews/actions.ts`
- Modify: `web/src/lib/workflows/workflows.service.ts`
- Modify: `web/src/lib/workflows/workflows.repository.ts`
- Modify: `web/src/lib/workflows/workflows.schemas.ts`

- [ ] **Step 1: Write the failing detail-query tests**

```ts
test("builds a review detail model with submitted and reviewed defaults", async () => {
  // expect submitted snapshot and editable reviewed payload
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- web/tests/unit/workflows.review-detail.test.ts`
Expected: FAIL because no detail query exists yet.

- [ ] **Step 3: Implement minimal detail query**

```ts
export async function getReviewRequestDetail(requestType: WorkflowRequestType, requestKey: string) {
  // load receive or return request
  // return submitted snapshot + reviewed defaults + validation capabilities
}
```

- [ ] **Step 4: Add protected review action wrappers**

```ts
export async function approveReviewAction(formData: FormData) { /* require admin, parse, review, revalidate */ }
export async function rejectReviewAction(formData: FormData) { /* require admin, parse, review, revalidate */ }
```

- [ ] **Step 5: Run the detail-query test file again**

Run: `npm run test -- web/tests/unit/workflows.review-detail.test.ts`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add web/tests/unit/workflows.review-detail.test.ts web/src/app/(protected)/reviews/actions.ts web/src/lib/workflows/workflows.service.ts web/src/lib/workflows/workflows.repository.ts web/src/lib/workflows/workflows.schemas.ts
git commit -m "feat: add pending review detail actions"
```

### Task 4: Build the review queue links and detail screen

**Files:**
- Create: `web/src/app/(protected)/reviews/[requestType]/[requestKey]/page.tsx`
- Modify: `web/src/app/(protected)/reviews/page.tsx`
- Optionally create: `web/src/components/reviews/ReviewDecisionForm.tsx`
- Optionally create: `web/src/components/reviews/ReviewAssetList.tsx`

- [ ] **Step 1: Write the failing UI test or route smoke spec**

```ts
test("renders a pending review detail page with submitted and review panels", async () => {
  // route/component smoke expectation
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- web/tests/unit/reviews.page.test.ts`
Expected: FAIL because no detail page or linked UI exists yet.

- [ ] **Step 3: Implement the review queue link and detail page**

```tsx
<Link href={`/reviews/${request.requestType.toLowerCase()}/${request.requestKey}`}>Review</Link>
```

Render:
- submitted snapshot panel
- editable reviewed form
- decision panel with `Approve` and `Reject`
- simple changed-field diff

- [ ] **Step 4: Run the UI test file again**

Run: `npm run test -- web/tests/unit/reviews.page.test.ts`
Expected: PASS

- [ ] **Step 5: Run full project verification**

Run: `npm run lint`
Expected: PASS

Run: `npm run typecheck`
Expected: PASS

Run: `npm run test`
Expected: PASS

Run: `npm run build`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add web/src/app/(protected)/reviews/page.tsx web/src/app/(protected)/reviews/[requestType]/[requestKey]/page.tsx web/src/components/reviews
git commit -m "feat: add pending reviews detail ui"
```

## Chunk 3: Manual Validation

### Task 5: Smoke-test the review workstation against seeded data

**Files:**
- No code changes required unless defects are found.

- [ ] **Step 1: Start or refresh the dev stack**

Run: `docker compose up --build -d`
Expected: app, db, and redis containers are healthy.

- [ ] **Step 2: Verify queue behavior manually**

Open:
- `http://localhost:3000/login`
- `http://localhost:3000/reviews`

Expected:
- pending requests are visible
- each request links to a detail page

- [ ] **Step 3: Approve and reject seeded requests**

Expected:
- approve removes request from queue and applies official effects
- reject removes request from queue without official effects
- audit page shows review activity

- [ ] **Step 4: Commit any follow-up bug fixes**

```bash
git add <files>
git commit -m "fix: polish pending reviews workflow"
```
