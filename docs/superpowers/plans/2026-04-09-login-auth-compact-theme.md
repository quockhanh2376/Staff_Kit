# Login Auth Compact Theme Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle the `Login` and `Forgot Password` experience so it matches the compact Settings visual system while keeping the existing auth flow, header brand, and theme toggle intact.

**Architecture:** Keep all auth behavior in `useAuthState.ts` unchanged and restyle only the shared shell in `LoginPage.tsx`. Add a dedicated script-level UI rail for the auth screen so the compact dark/light styling and mode-specific controls stay locked in without touching backend logic.

**Tech Stack:** React, TypeScript, Tailwind utility classes, existing app CSS tokens, Node `assert` script rails

---

## File Structure

- Modify: `src/features/auth/LoginPage.tsx`
  - Own the visual shell for both `Login` and `Forgot Password`
  - Keep header brand and theme toggle
  - Render compact card structure and shared buttons/inputs
- Create: `scripts/login-auth-ui.test.ts`
  - Assert key auth UI markers for both modes so the compact theme stays stable
- Optional modify: `src/index.css`
  - Only if a tiny auth-specific utility is needed to keep `LoginPage.tsx` readable

## Chunk 1: Add Auth UI Rail

### Task 1: Lock the intended auth shell markers

**Files:**
- Create: `scripts/login-auth-ui.test.ts`
- Read: `src/features/auth/LoginPage.tsx`

- [ ] **Step 1: Write the failing test**

Create `scripts/login-auth-ui.test.ts` with assertions that expect the compact auth shell markers to exist:

```ts
import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const loginPage = readFileSync("src/features/auth/LoginPage.tsx", "utf8")

assert.match(loginPage, /max-w-\[420px\]/)
assert.match(loginPage, /bg-\[#161b22\]/)
assert.match(loginPage, /bg-\[#0d1117\]/)
assert.match(loginPage, /Forgot Password/)
assert.match(loginPage, /Back to login/)
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
node --experimental-strip-types scripts/login-auth-ui.test.ts
```

Expected: FAIL because the current `LoginPage.tsx` still uses the older auth shell markers.

- [ ] **Step 3: Commit the red test**

```bash
git add scripts/login-auth-ui.test.ts
git commit -m "test: add login compact auth ui rail"
```

## Chunk 2: Restyle Login And Forgot Password

### Task 2: Apply the compact auth card for both modes

**Files:**
- Modify: `src/features/auth/LoginPage.tsx`
- Optional modify: `src/index.css`
- Test: `scripts/login-auth-ui.test.ts`

- [ ] **Step 1: Implement the minimal UI change**

Update `LoginPage.tsx` so that:

- the top `Staff Kit` + `Dark / Light` header stays
- the centered auth card uses compact spacing and the darker mock-aligned surfaces
- both modes share the same compact shell
- the login primary button uses emerald emphasis
- the secondary buttons use outline/ghost styling
- input rows and labels match the compact Settings tone

Keep these behaviors unchanged:

- `handleLoginSubmit`
- `handleForgotPasswordSubmit`
- password visibility toggles
- loading banners and global error rendering

- [ ] **Step 2: Run the auth UI rail**

Run:

```bash
node --experimental-strip-types scripts/login-auth-ui.test.ts
```

Expected: PASS

- [ ] **Step 3: Run frontend verification**

Run:

```bash
npm run check:frontend
```

Expected: PASS

- [ ] **Step 4: Refactor only if needed**

If `LoginPage.tsx` becomes too dense, extract only tiny auth-specific styling helpers. Do not move auth behavior out of `useAuthState.ts`.

- [ ] **Step 5: Commit**

```bash
git add src/features/auth/LoginPage.tsx src/index.css scripts/login-auth-ui.test.ts
git commit -m "style: compact login auth theme"
```

## Final Verification

- [ ] Run:

```bash
node --experimental-strip-types scripts/login-auth-ui.test.ts
npm run check:frontend
```

- [ ] Confirm both `Login` and `Forgot Password` still render from the same component and that no auth behavior code changed outside the page shell.
