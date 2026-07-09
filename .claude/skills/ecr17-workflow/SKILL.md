---
name: ecr17-workflow
description: Use when implementing any task in rust-ecr17-protocol — enforces the per-task Definition-of-Done loop (TDD, local tests, local Copilot review, push, CI, PR + Copilot review, merge) and the money-safety + branch/PR conventions.
---

# ECR17 workflow

Follow this for every task/subtask in this repo. See `AGENTS.md`, `docs/PLAN.md`,
`docs/LESSON.md`, `PROGRESS.md`.

## Before coding
1. Read `PROGRESS.md` (resume point) + `docs/LESSON.md` (pass its content into any
   sub-agent prompt). Identify the current macro-task branch and the subtask.
2. Restate the subtask's **objective · implementation detail · guardrails** from `docs/PLAN.md`.

## Branch / PR model
- One **branch per macro-task**; one **PR per subtask** → that branch; macro-task complete
  → one **PR → main**. Never commit straight to `main`.

## TDD (RED → GREEN)
- Write the failing `cargo test` (or Vitest / Playwright) FIRST, watch it fail, then implement.
- Keep `codec`/`protocol`/`response` pure & sync; put I/O behind the async `Transport` trait.
- 💰 Never weaken a money-safety test to make a change pass (see `.claude/rules/rule-money-safety.md`).

## Local loop (before pushing)
1. `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check`.
   Frontend: `bun run typecheck` + `vitest run`; UI changes: `npx playwright test`.
2. Local Copilot review: `copilot --autopilot --yolo -p "/review <branch diff vs origin/main>"`
   — pass the FULL branch diff (save to a temp file if large). Copilot edits in `--yolo` and can
   be wrong → VERIFY every change; record takeaways in `docs/LESSON.md`.
3. Zero actionable comments → continue; else fix and go to 1.

## Remote loop
4. Push; wait for CI green (`rust-tests` + `frontend-checks` + `e2e` as applicable); else fix.
5. Open the PR; add **Copilot** as reviewer; ensure the review started; WAIT for CI + Copilot.
6. Fix every valid comment (reject only with a written reason), push, re-request review.
   Repeat 4–6 until ZERO actionable comments.
7. Merge. Update `PROGRESS.md` (+ `docs/LESSON.md`). Move to the next task.

## Environment notes
- `Bash` tool = git-bash (heredocs / `git commit -F -`, never PowerShell here-strings).
- Commit messages: conventional style, end with the `Co-Authored-By` trailer.
