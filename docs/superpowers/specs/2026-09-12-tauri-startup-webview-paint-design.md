# Tauri Startup WebView Paint Design

## Context

On Windows, `npm run tauri:dev` launches a native Staff Kit window whose client area stays black. DevTools confirms that Vite connects, React loads without errors, and the login screen is present in `#root`. The problem is therefore not a React exception, Vite availability, or database initialization.

The current startup path creates the window with `visible: false`, then calls `window.show()` from the Rust `setup` callback. That callback occurs before the frontend has rendered its first frame. On the affected WebView2 runtime, showing at that point can leave the native surface black despite a populated DOM.

## Decision

Keep the window initially hidden. Add one public, no-data Tauri command that shows the main window. The React entry point will render `App`, then schedule that command with `requestAnimationFrame`. The command is idempotent: calling it more than once remains safe, which accommodates React Strict Mode development behavior.

The initial `show()` call in Rust will be removed. The frontend readiness signal becomes the sole route that makes the main window visible.

## Data Flow

1. Tauri creates `main` with `visible: false`.
2. Vite/WebView2 loads `src/main.tsx` and React renders `App` into `#root`.
3. The next animation frame invokes `show_main_window` through the existing IPC bridge.
4. Rust resolves the `main` webview window and calls `show()`; a missing window becomes a descriptive command error.

## Error Handling

The readiness call is best-effort for development ergonomics: a failed show is logged to the DevTools console, because the alternative is an invisible app with no user-facing surface. It does not call database APIs or mutate business data.

## Testing

- Add a focused frontend test for the readiness helper: it schedules exactly one IPC call on the next animation frame and reports an invocation failure.
- Add a Rust unit test around the command's window-resolution helper where practical; the Tauri window object itself remains an integration responsibility.
- Run the focused frontend test, `npm run check:tauri`, and `npm run check:quality`.
- Manually run `npm run tauri:dev`; the Login page must become visible without opening DevTools.

## Constraints

- Preserve `visible: false` and `backgroundColor` in `src-tauri/tauri.conf.json`.
- Do not alter database initialization, authentication, LAN server startup, IPC authorization, or runtime data.
- Use no new dependency.
