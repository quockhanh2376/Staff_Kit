---
description: Bootstrap StaffKit (Tauri v2 + Rust + React) for an existing project, adapted from BMAD Established Projects + Quick Flow.
---

# Bootstrap Workflow (StaffKit)

> [!IMPORTANT]
> Keep this workflow lean. Only include steps needed for the current StaffKit scope.

---

## Step 1: Scope Gate (From BMAD Quick Flow)

Classify the request before setup:

- Small scoped work (bug fix, small feature, limited files): continue directly.
- System-level work (new platform boundaries, major schema/API changes): write a short tech spec first, then execute.

---

## Step 2: Establish Project Context (From BMAD Established Projects)

Use these files as source-of-truth:

- `Note.md`
- `Tech_tags.md`

Keep a lean context file at `.agent/project-context.md` with:

- technology stack and runtime versions
- critical implementation rules
- data/file handling boundaries (for example: `ExSource/` is local input only, not committed)

---

## Step 3: Toolchain Prerequisites (Only What This Project Needs)

Required baseline:

- Node.js 20+ and npm
- Rust stable (`rustc`, `cargo`)
- Tauri CLI (`cargo tauri`)

Optional (only when targeting mobile):

- Android: JDK + Android Studio/SDK/NDK
- iOS: macOS + Xcode

---

## Step 4: Initialize or Align Project Skeleton

For the current stack, ensure:

- frontend scaffold exists (TypeScript + React)
- `src-tauri` exists and is buildable
- SQLite integration path is defined
- Excel input path is defined (`ExSource/`)

Do not add frameworks or packages not required by the current scope.

---

## Step 5: Baseline Verification

Run only essential checks:

- install dependencies
- app boots in dev mode
- desktop build path works
- lint/test only if already configured in the repo

If a required command or dependency is missing, install only that missing part.

---

## Step 6: Quick Execution Loop (From BMAD Quick Dev)

For small scoped implementation:

1. Capture baseline (`git status`, current commit/branch).
2. Gather context (target files, existing patterns, dependencies).
3. Execute end-to-end without unnecessary pauses.
4. Self-check against acceptance criteria.
5. Review diff and resolve findings before commit.

---

## Output Criteria

- Environment is ready for current target platform(s)
- Stack aligns with `Tech_tags.md`
- No unnecessary dependencies introduced
- Work is reproducible from repo state and docs
