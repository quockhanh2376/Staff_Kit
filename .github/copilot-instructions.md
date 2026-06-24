# Project Guidelines

## Scope

- Treat this workspace as the Staff Kit desktop project first.
- `web/` and AssetDesk-Pro materials are reference-only unless the task explicitly targets them.
- Do not edit unrelated worktrees or generated outputs unless the task requires it.

## Workflow

- Follow a Superpowers-style workflow for non-trivial work: clarify the problem, confirm the intended behavior, then implement.
- For ambiguous or open-ended requests, start with design clarification before writing code.
- For substantial work, prefer this sequence: clarify or design, isolate if needed, plan, implement, validate, then summarize risks.
- Before starting feature work or a multi-file change, use the skills under `.agents/skills/` when relevant.
- Use `using-git-worktrees` for isolated feature work when the task is substantial, risky, or likely to conflict with in-flight changes.
- Use `subagent-driven-development` when executing a concrete implementation plan with mostly independent tasks.
- Use `dispatching-parallel-agents` only when multiple failures or tasks are clearly independent.
- Do not skip executable validation when a narrow check exists for the touched slice.

## Build and Test

- Install root dependencies with `npm install`.
- Run the desktop app with `npm run tauri:dev`.
- Use `npm run check:quality` before final handoff when the task changes product code.
- If a Tauri or Rust change is involved, validate with `npm run check:tauri` or `cargo check --manifest-path src-tauri/Cargo.toml`.

## Conventions

- Prefer minimal, root-cause fixes over broad refactors.
- Read nearby code and existing docs before editing; use `README.md`, `QUALITY.md`, and `CLAUDE.md` as local guidance.
- Keep changes aligned with the existing separation of concerns: React UI and state in `src/`, Tauri command surface and persistence in `src-tauri/`.
- Keep diffs small, avoid unrelated cleanup, and report any checks you could not run.
- Never commit secrets, raw business data, or `.env` files.