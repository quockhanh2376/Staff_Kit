# Staff Kit — Agent Guide

## Scope

- This workspace is **Staff Kit** (`ST`) only. The `web/` directory is **AssetDesk-Pro** (`ASP`) — reference-only, do not edit.
- Active code paths: `src/` (React frontend) and `src-tauri/` (Rust backend).

## Commands

| Command | What it does |
|---|---|
| `npm run dev` | Vite dev server only (no Tauri) |
| `npm run tauri:dev` | Full desktop app (use `dev.ps1` to add `~/.cargo/bin` to PATH first) |
| `npm run lint` | ESLint on `src/` |
| `npm run typecheck` | `tsc -b` (strict, noUnusedLocals/Parameters) |
| `npm run build` | `tsc -b && vite build` |
| `npm run check:tauri` | `cargo check` via shared-target wrapper |
| `npm run check:quality` | **lint → typecheck → build → cargo check** (full gate) |
| `npm run test:tauri` | `cargo test` via shared-target wrapper |
| `npm run codex:headroom` | Launch Codex through the repo-local Headroom wrapper |

- `.npmrc` sets `script-shell=powershell.exe` — npm script conditionals use PowerShell syntax (`; if ($LASTEXITCODE -ne 0)`).
- Tauri/cargo commands go through `scripts/run-with-shared-cargo-target.mjs` which sets `CARGO_TARGET_DIR` to a shared dir under `.git` for worktree reuse.
- `.codex/config.toml` is the repo-local Codex config. Keep the `headroom` MCP entry there so Codex can recover compressed context when launched via `npm run codex:headroom`.

## Architecture

- **Frontend** (`src/`): React 19 + TypeScript, feature-oriented under `src/features/` (auth, employees, teams, borrow, assets, settings, columns, import). Shared utilities in `src/lib/`, types in `src/types/`. IPC bridge: `src/services/staff-api.ts` (thin typed wrapper over `@tauri-apps/api/core` `invoke`).
- **Backend** (`src-tauri/`): Tauri v2 commands in `src/lib.rs` (thin wrappers), logic split across `db/` modules (employee, team, auth, import, asset, backup, borrow, etc.). Uses rusqlite with SQLCipher (`bundled-sqlcipher-vendored-openssl`). LAN borrow server via axum in `lan_server.rs`.
- **Data flow**: Excel/input → validate/normalize → SQLite → UI. Not the other way around.
- **Auth**: Local accounts with password hashing (argon2) + recovery codes. Roles: `super_admin`, `admin`, `user`.
- **Theme**: Per-user scoped localStorage keys (`staffkit-<scope>-theme`). Dark default.
- **Column prefs**: Scoped per-user + per-staff-group. Stored in localStorage.
- **Backup**: SQLite backup + snapshot/restore system. Snapshot on close (`app_close`).
- **Tauri window**: Starts `visible: false` to avoid black flash; shown after setup completes.

## Engineering Conventions

- Keep diffs small, no unrelated cleanup. Feature work on dedicated branches/worktrees.
- Prefer root-cause fixes over broad refactors. Read nearby code before editing.
- Excel/import validation happens before DB writes. DB queries are parameterized. No plaintext secrets in logs.
- `ExSource/` is local input only — never committed.
- `imports/`, `exports/`, `data/local/`, `data/runtime/` are gitignored.
- Refer to existing instruction files: `CLAUDE.md`, `QUALITY.md`, `.github/copilot-instructions.md`, `.github/instructions/*.md`, `.agent/project-context.md`.
- `daily_log.md` is the canonical NotebookLM source. Keep one source of truth.

## Skills (`.agents/skills/`)

- `using-git-worktrees` — for isolated feature work
- `subagent-driven-development` — for executing implementation plans
- `dispatching-parallel-agents` — for independent tasks
- Beads workflow: `bd ready` → `bd show <id>` / `bd note <id>`

## Validation

- For frontend-only changes: narrowest relevant check first.
- For Rust/Tauri changes: `npm run check:tauri` (or `cargo check --manifest-path src-tauri/Cargo.toml`).
- For anything touching product code: `npm run check:quality` before final handoff.
