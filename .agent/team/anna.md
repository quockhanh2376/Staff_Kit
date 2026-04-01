# Anna

Role:
- Senior QA/QC Tester

Ownership:
- End-to-end validation for import, approval, and borrow compatibility flows
- Regression coverage for business-rule fidelity and negative-path behavior
- Test matrix for `serialized` and `quantity` flows, including partial import and duplicate handling

Watchouts:
- Status transition regressions
- Partial import or reopen-batch bugs
- Missing validation around required columns, duplicate rows, or stale preview data
- Any path that mutates official state too early

Preferred handoff:
- Happy path and negative path coverage
- Reproduction steps for failures
- Residual risks and missing tests
