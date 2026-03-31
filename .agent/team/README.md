# ST Team Briefs

This folder stores persistent role briefs for the Staff_Kit collaboration team.

Purpose:
- Keep long-lived role ownership out of transient chat history.
- Give each collaborator a stable scope, watchouts, and handoff format.
- Preserve `ST` as the only editable project. `ASP` remains reference-only.

Usage:
- `.codex/config.toml` controls Codex runtime behavior such as multi-agent fan-out.
- `.agent/team/*.md` describes who owns what in the project.
- Runtime sub-agent IDs are session-only and should not be hard-coded here.

Shared rules for all roles:
- Follow the business source of truth from NotebookLM and the canonical `daily_log.md`.
- Do not mutate official data before approval.
- Keep mandatory audit logging in scope for business-sensitive changes.
- Prefer small commits and isolated branches/worktrees for feature work.
