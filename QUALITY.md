# Code Quality Gate (Staff Kit)

Nguon tham chieu:
- `.agent/workflows/development.md`
- `.agent/rules/security.md`
- `bmad_method_ref/.github/workflows/quality.yaml`

## 1) Development Discipline

- Gather context before edit:
  - xac dinh file can sua
  - doc code lien quan de theo naming/style pattern
  - liet ke dependency/config impact
- Implement end-to-end trong pham vi task.
- Self-check truoc handoff:
  - task scope complete
  - acceptance criteria dat
  - checks/tests da chay (hoac neu chua, phai report ro)
- Diff-based review mindset:
  - tim risk/bug/regression truoc khi chot

## 2) Security & Data Rules

- Khong commit secrets (`.env`, credentials).
- `ExSource/` la local input-only source, khong commit raw business data.
- Validate file input (Excel) truoc khi ghi DB.
- DB queries phai parameterized.
- Khong log plaintext sensitive fields (email, ids, tokens).

## 3) Data Pipeline Rule (Locked)

- Dev input: `ExSource/*.xlsx`
- Pipeline: `ExSource -> import/validate/normalize -> SQLite -> UI/report`
- UI va report chi doc data tu DB, khong doc truc tiep Excel.

## 4) Required Checks Before Commit

Run:

```bash
npm run check:quality
```

Equivalent:

1. `npm run lint`
2. `npm run typecheck`
3. `npm run build`
4. `cargo check --manifest-path src-tauri/Cargo.toml`

## 5) Optional CI Expansion (recommended)

- Add GitHub Actions quality workflow for:
  - lint
  - build/typecheck
  - rust check
  - markdown lint
- Keep parity between local script and CI gates.
