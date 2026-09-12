# Tauri Startup WebView Paint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show Staff Kit only after React renders its first frame, removing the Windows WebView2 black-screen startup failure.

**Architecture:** Tauri keeps the main window hidden. Rust provides public, idempotent `show_main_window`; a frontend helper calls it through IPC on the next animation frame after React mounts.

**Tech Stack:** Tauri v2/Rust, React 19, TypeScript, Vitest.

## Global Constraints

- Preserve `visible: false` and `backgroundColor` in `src-tauri/tauri.conf.json`.
- Do not alter database initialization, authentication, LAN startup, business data, or dependencies.
- The startup IPC takes no data and remains public.

---

### Task 1: Frontend first-frame signal

**Files:**
- Create: `src/lib/showMainWindowWhenReady.ts`
- Create: `src/lib/showMainWindowWhenReady.test.ts`
- Modify: `src/main.tsx`
- Modify: `src/services/staff-api.ts`
- Modify: `src/services/staff-api.test.ts`

**Interfaces:**
- `scheduleMainWindowShow(show, scheduleFrame, reportError): number` schedules `show` exactly once in the next frame.
- `staffApi.showMainWindow(): Promise<void>` invokes public `show_main_window`.

- [ ] **Step 1: Write and run a failing test**

```ts
it("waits for the next animation frame before asking Tauri to show the window", async () => {
  const show = vi.fn().mockResolvedValue(undefined)
  let frameCallback: FrameRequestCallback | undefined
  const scheduleFrame = vi.fn((callback: FrameRequestCallback) => {
    frameCallback = callback
    return 7
  })
  scheduleMainWindowShow(show, scheduleFrame, vi.fn())
  expect(show).not.toHaveBeenCalled()
  frameCallback?.(0)
  await Promise.resolve()
  expect(show).toHaveBeenCalledTimes(1)
})
```

Run: `npm run test:unit -- src/lib/showMainWindowWhenReady.test.ts`

Expected: FAIL because the helper does not exist.

- [ ] **Step 2: Implement and verify frontend behavior**

```ts
export function scheduleMainWindowShow(show, scheduleFrame, reportError) {
  return scheduleFrame(() => {
    void show().catch((error) => reportError("[Staff Kit] failed to show main window:", error))
  })
}
```

Add `showMainWindow: () => callPublic<void>("show_main_window")` to `staffApi`; call the helper after `createRoot(...).render(...)` with `requestAnimationFrame`. Add a service test asserting `invoke("show_main_window", undefined)`.

Run: `npm run test:unit -- src/lib/showMainWindowWhenReady.test.ts src/services/staff-api.test.ts`

Expected: PASS.

### Task 2: Backend visibility command

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- `#[tauri::command] fn show_main_window(app: tauri::AppHandle) -> Result<(), String>` resolves `main` and calls `show()`.

- [ ] **Step 1: Add and register command**

```rust
#[tauri::command]
fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())?;
    window.show().map_err(|error| format!("failed to show main window: {error}"))
}
```

Register it beside `ping` in `generate_handler!`. Remove only the existing early `window.show()` block in `setup`.

- [ ] **Step 2: Compile backend**

Run: `npm run check:tauri`

Expected: exit code 0.

### Task 3: Verify and accept on Windows

**Files:**
- Modify: `daily_log.md`

- [ ] **Step 1: Run full quality gate**

Run: `npm run check:quality`

Expected: exit code 0.

- [ ] **Step 2: Review scope**

Run: `git diff --check` and `git diff --stat HEAD~1`

Expected: only startup helper/test, bridge, Rust command, log and plan files.

- [ ] **Step 3: Manual acceptance**

Run: `npm run tauri:dev`

Expected: Login page appears without F12/DevTools and no black client area remains.

- [ ] **Step 4: End record and commit**

Record exact check results and manual outcome in `daily_log.md`, then commit the implementation with message `fix(tauri): show window after first React frame`.
