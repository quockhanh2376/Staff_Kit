# ECC Lite Manifest

## Audit record

- Upstream: <https://github.com/affaan-m/ECC>
- Upstream commit audited: `c714dc56540ec514271fc18820f228204cd2a3c1`
- Audit date: 2026-07-26
- Installation scope: Staff Kit repository only
- Installed by copying: nothing. Each file below is a short Staff Kit-specific adapter based on audited ECC patterns.

Staff Kit instructions remain authoritative in this order: user request; safety/Codex harness; `AGENTS.md`; feature instructions; approved specification/plan; existing Staff Kit/Superpowers workflow; ECC Lite support; generic defaults.

## Selected components

| Local component | Type | Trigger | Staff Kit value | Supplements | Conflict/dependency/file count |
| --- | --- | --- | --- | --- | --- |
| `.agents/skills/staffkit-research-first/SKILL.md` | Skill | Version-sensitive API, dependency, integration, or uncertain implementation decision | Local-code-first evidence and sourced conclusions | `.agent/rules/research.md` | No conflict; no dependency; 1 file |
| `.agents/skills/staffkit-security-review/SKILL.md` | Skill | Requested review or sensitive auth/IPC/LAN/import/filesystem/backup/DB change | Tauri/Axum/SQLCipher-specific review boundary | `CheckSecurity.md`, `.agent/rules/security.md` | No conflict; review-only; 1 file |
| `.agents/skills/staffkit-architecture-review/SKILL.md` | Skill | Cross-layer design or code review | Minimal ownership/data-flow map and diff packet | `.agent/project-context.md` and existing review workflow | No conflict; no memory system; 1 file |
| `.agents/skills/staffkit-delivery-verification/SKILL.md` | Skill | Before delivery/release handoff | Exact Staff Kit command selection and honest verification report | `QUALITY.md`, `AGENTS.md` quality gate | No conflict; no hook; 1 file |

All four are `ON_DEMAND`; no `DAILY` ECC surface was added because Staff Kit already has concise project instructions and the added capabilities are task-specific.

## Rejected components

| ECC component/group | Reason |
| --- | --- |
| Full installer, `full` profile, full skills/agents/rules/commands | Violates selective project-local scope and adds duplicate workflow surface. |
| Worktree, planning, parallel-agent, and generic implementation workflows | Staff Kit already has canonical skills and instructions. |
| Codex role agents and `.codex/config.toml` changes | No configured canonical role registry is needed; preserves Headroom MCP/config. |
| Rust, TypeScript, and database reviewer agents | Generic reviewers duplicate existing guidance; database reviewer is PostgreSQL/Supabase oriented rather than SQLite/rusqlite. |
| `delivery-gate`, hooks, scripts, daemon, memory/instinct/control-plane/dashboard | Automatic enforcement, persistent state, or background behavior is out of scope. |
| MCP bundle, Context7/external search MCP, paid/API-backed services | Existing Codex capability is sufficient; no added config, key, or paid dependency. |
| Generic security scan/AgentShield | Would add external tooling; Staff Kit needs a review-only capability. |
| Non-stack ecosystems and business/media/cloud/mobile/social tooling | No Staff Kit use case. |

## Conflict check

- Preserved unchanged: `AGENTS.md`, `.codex/config.toml`, Headroom MCP, `.github/copilot-instructions.md`, `.github/instructions/`, `QUALITY.md`, `CheckSecurity.md`, `daily_log.md`, existing `.agents/skills/`.
- No product code under `src/` or `src-tauri/`, no `web/` files, hooks, MCP servers, commands, dependencies, global configuration, telemetry, secrets, or background services were added.
- Skill names are unique: `staffkit-research-first`, `staffkit-security-review`, `staffkit-architecture-review`, `staffkit-delivery-verification`.

## Update and uninstall

To update, clone/audit ECC outside Staff Kit, record the reviewed commit, and manually adapt only compatible material after rerunning this conflict check. Do not run ECC installers.

To uninstall, delete the four directories under `.agents/skills/` named above and this manifest, or revert the installation commit. No runtime state or global configuration needs cleanup.

## Validation record

Static validation must confirm unique frontmatter names, local paths, cited Staff Kit commands, absence of secrets/config/hook/MCP changes, and a docs-only diff. Product build checks are intentionally not required because no product code changed.
