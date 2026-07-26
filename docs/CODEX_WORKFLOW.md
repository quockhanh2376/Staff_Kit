# Codex Operating Model

This is the canonical operating model for non-trivial Staff Kit work. Priority is:
user request, Codex safety, `AGENTS.md`, feature instructions, approved specs/plans,
this document, then skills. Staff Kit governance and validation always win.

## Ownership

| Component | Owns | Must not own |
| --- | --- | --- |
| `AGENTS.md` | Scope, architecture, security, commands, definition of done | Workflow implementation details |
| Project-local Superpowers | Brainstorming, plans, TDD, debugging, review, worktrees, delivery | Project truth, output style |
| ECC Lite | Four on-demand research/security/architecture/delivery capabilities | Workflow, memory, hooks, MCP, autonomous loops |
| `staffkit-focused-output` | Action-first progress and evidence formatting | Technical decisions or validation |
| Headroom | Compression, retrieval, statistics, context transport | tokensave, Serena, learning, memory, output shaping, config authority |
| `daily_log.md` | Human-readable session history | Specification or implementation history |
| Git | Committed implementation history | Runtime state |

## Canonical workflow

1. Read `AGENTS.md`, this document, applicable feature instructions, and `daily_log.md`.
2. Classify the task and inspect scope/status before editing.
3. For non-trivial work use project-local Superpowers: brainstorm → plan → isolated
   worktree when justified → TDD/implementation → review → verification → branch finish.
4. Use ECC Lite only when its trigger matches; it never becomes a second workflow.
5. Run the narrowest relevant Staff Kit check, then the applicable quality gate.
6. Report evidence, checks, risks, and next action.

`openspec/changes/` stores business specification and acceptance intent.
`docs/superpowers/plans/` stores executable implementation plans.
Neither replaces `daily_log.md` or Git history.

## Installed inventory

- Superpowers is project-local under `.agents/skills/`, pinned to upstream
  `obra/superpowers` commit `3dcbd5c4b48e02263fbf4a3c01e3fe4f81d584d9`.
- Required active skills are: `using-superpowers`, `brainstorming`,
  `systematic-debugging`, `using-git-worktrees`, `writing-plans`, `executing-plans`,
  `subagent-driven-development`, `dispatching-parallel-agents`,
  `test-driven-development`, `requesting-code-review`, `receiving-code-review`,
  `verification-before-completion`, and `finishing-a-development-branch`.
- ECC Lite is exactly the four skills listed in `docs/ecc-lite-manifest.md`.
- `staffkit-focused-output` is presentation only.
- Global plugin caches are not imported into the Staff Kit runtime.

## Headroom runtime

Canonical launch: `npm run codex:headroom`. Explicit fail-open launch:
`npm run codex:direct`. The wrapper resolves the repository from its own location,
creates an isolated runtime under the machine-local application-data directory,
keeps authentication outside Git, and invokes project-local Headroom with
`--no-context-tool --no-tokensave --no-serena`. It never passes `--learn` or
`--memory`. Headroom failure warns and falls back to direct Codex using the isolated
runtime. `.codex/config.toml` is a portable template and must remain unchanged by
runtime use. The only approved project MCP is Headroom.

Headroom exposes only `headroom_stats`, `headroom_compress`, and
`headroom_retrieve` to this project. Each has an explicit per-tool
`approval_mode = "approve"` because it operates against the local Headroom context
store and does not change project files. Adding another Headroom tool requires
explicit review and a tracked config change; broad server-wide approval is forbidden.

`codex_apps` is a built-in/host-provided Codex application capability. Evidence:
it is absent from `codex mcp list`, absent from project/user MCP config, and appears
only in Codex's host-managed app tool cache/namespace. It is not a
project-configured external MCP. User-global capabilities are likewise outside the
isolated Staff Kit project inventory. The acceptance rule is: no unapproved
project-configured external MCP servers.

`scripts/verify-codex-tooling.ps1` is a read-only static audit. `STATIC READY`
means the tracked topology/configuration passes; only a separate fresh Headroom
session that calls stats, compress, and retrieve without changing the tracked config
may establish the final live `READY` verdict.

The direct Codex session is the repair/fail-open path and must remain usable without
Headroom. Do not launch tooling-repair work through the wrapper while repairing it.

## Legacy compatibility

`.agent/workflows/`, `.agent/rules/`, and `.agent/skills/` are legacy reference
material only. Files with historic workflows carry an explicit legacy header. They
are not an operational authority and must not require unavailable MCPs, broad
default research, mandatory waits, or a competing planning lifecycle.
Move useful Staff Kit business, Windows, Tauri, Rust, database, and security
constraints into canonical documentation or a scoped project skill when needed.

Tracked tooling and current documentation use repository-relative paths. Absolute
machine paths belong only in ignored runtime state or clearly labeled historical
diagnostics.

## Session continuity

Before repository-changing work append a Start record to `daily_log.md`; before
handoff, machine switch, or final conclusion append an End record. Never write
secrets, auth, tokens, or employee-private data. The log is tracked and is the only
canonical human-readable session log.

Second-machine bootstrap: clone the repository, install Node/Rust/MSVC/Perl
prerequisites, create `.headroom-venv` with Python 3.13, install
`headroom-ai[all]`, then run `scripts/verify-codex-tooling.ps1`.
