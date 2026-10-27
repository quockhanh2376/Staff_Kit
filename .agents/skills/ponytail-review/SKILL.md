---
name: ponytail-review
description: >
  Quality and lean review of a change (Ponytail 5.3.2): evaluates correctness, security,
  boundary edge cases, test coverage, and unnecessary code bloat. Reads connected callers,
  not only the diff. Use for "review this", "code review", "is this over-engineered", /ponytail-review.
metadata:
  upstream: https://github.com/DietrichGebert/ponytail
  version: "5.3.2"
  license: MIT
  adaptation: Staff_Kit scoped on-demand skill
---

# Ponytail Review

Review a change like the senior developer who will be paged when it breaks.
Order of importance: correct, safe, holds under load, tested, fast, lean.
Lean matters: every extra line must be read, tested and maintained later.

> **Precedence in Staff Kit:**
> This review supplements Qeen independent verification and does NOT replace `check:quality` or beads validation.

## 1. Understand first

- Review what the user names: uncommitted changes, a branch, a PR, or specific files.
- Read the diff, then the code it touches: callers of every changed function, functions it calls, tests.
- Trace the real data flow: input validation, database query, IPC boundary, output serialization.
- Grep every caller when a signature, return value or behavior changes.

## 2. Review checklist

1. **Bug:** wrong result, crash, missed edge cases (empty, null, boundary, rounding).
2. **Security & Data Safety:** SQLCipher parameterization, input sanitization, token checks, session invalidation, error handling.
3. **Scale & Concurrency:** async vs sync, connection reuse, memory leaks, query storms.
4. **Missing tests:** risky new branches or logic without failing test proof.
5. **Lean & YAGNI:**
   - delete: dead code, speculative options
   - reuse: existing repository helpers
   - standard platform / library: avoid new dependencies
   - merge: copy-pasted duplicates

## 3. Findings format

Explain every technical issue clearly with concrete examples:

Start with `What this change does:` in 2–3 sentences.

Then group findings (skip empty groups):
- **Must fix:** bug, security flaw, data corruption, broken caller.
- **Should fix:** missing test for risky logic, duplicate code, unnecessary complexity.
- **Nice to have:** small cleanups or shorter idiomatic forms.

Every finding must provide:
- **What this is:** Short summary of the component or code touched.
- **Problem:** Exact case where it fails or causes issues.
- **Fix:** Minimal concrete fix with zero unrequested abstraction.
- **If we skip it:** Real-world consequence.

End with:
- `Verdict: Ship.` or `Verdict: fix <N> first.`
- `Lean: -<N> lines possible.` (if applicable)
- `Not checked:` one line if any aspect was out of scope.
