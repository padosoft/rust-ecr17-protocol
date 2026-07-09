# AGENTS.md — rust-ecr17-protocol

Guidance for AI agents (and humans) working in this repo. Read this first, then
`PROGRESS.md` (current task / resume state), `docs/LESSON.md` (accumulated,
hard-won engineering lessons), and `docs/PLAN.md` (the implementation plan).
**Always pass `docs/LESSON.md` into the prompt of any sub-agent you spawn, and
re-read it when starting a new session.**

## What this is
A **Rust** implementation of the Italian **ECR17** payment protocol (Nexi Group POS
terminals) over LAN, plus a **Tauri** desktop **control-panel** demo app. Port of the
tested React Native reference (`../ReactNative/react-native-ecr17-protocol`, protocol
engine in C++). Sibling ports: `padosoft/react-native-ecr17-protocol`,
`padosoft/laravel-ecr17`.

Layered pure-Rust core (`crates/ecr17-protocol/src/`):
`lrc` (LRC) → `codec` (framing) → `protocol` (builders) → `response` (parsers) →
`session` (ACK/NAK + retransmit + timeout, money-safe via `retry`) → `client` (async
API). Transport: async `Transport` trait + `transport/tcp.rs` (tokio, feature-gated) +
`FakeTransport` (tests). The Tauri app (`app/`) wraps `Ecr17Client` behind
`#[tauri::command]`s with a React+TS+Vite UI.

## Mandatory workflow (Definition of Done)
A task/phase is done ONLY after BOTH loops below pass. In automode, proceed to the
next phase only once complete.

### Local loop (per task, before pushing)
1. **Local tests green** — Rust: `cargo test` + `cargo clippy --all-targets -- -D warnings`
   + `cargo fmt --check` (the protocol core is fully unit-tested — the real correctness
   gate). Frontend: `bun run typecheck` + `vitest run`; UI changes: `playwright test`.
2. **Local Copilot review** — `copilot --autopilot --yolo -p "/review …"` with the
   **full branch diff vs `origin/main`** (save the diff to a temp file and pass the file
   if it is large; use a focused prompt if a whole-diff review times out). Copilot
   **edits in --yolo mode** and can be wrong → VERIFY every change against the code/tests.
   Record takeaways in `docs/LESSON.md`.
3. **Zero actionable comments** → continue; else fix and go to 1.

### Remote loop (REQUIRED before a task/PR is done)
4. **Push**, then **CI green** (`rust-tests` + `frontend-checks` + `e2e` as applicable);
   else fix → local loop.
5. **Remote PR** — open a PR (subtask → macro-task branch; macro-task complete → `main`).
   Add **Copilot** as reviewer; ensure its review started; **WAIT** for CI + Copilot.
6. **Fix every valid comment** (validate each against code/spec; reject only with a clear
   reason), push, re-request review. **Repeat 4–6 until ZERO actionable comments.**
7. Only then merge. Update `PROGRESS.md` and `docs/LESSON.md`.

Rationale: local verification can miss things; the remote CI + AI-review loop is a
second independent gate — never merge a PR with open, valid reviewer comments.

## Branch / PR model
One **branch per macro-task**; one **PR per subtask** targeting that branch; when the
macro-task is complete, one **PR macro-branch → main** through the full loop.

## Guardrails per task
Every task states **objective · implementation detail · guardrails**. Guardrails:
Rust → `cargo test` (TDD RED→GREEN) + clippy + fmt. Frontend logic → Vitest. UI/UX →
Playwright covering **every** interaction. Pure-code changes with no UI surface need no
Playwright.

## CI
- `rust-tests` (fast, the correctness gate): `cargo build`, `cargo test`, `clippy -D warnings`,
  `fmt --check`. Keep it green.
- `frontend-checks` (fast): `tsc`, `vitest`, lint.
- `e2e`: Playwright against the Vite frontend with Tauri IPC mocked.
- `tauri-build` (heavier, tag/manual): builds installers (Win `.msi`/NSIS, macOS `.dmg`,
  Linux `.deb`/AppImage).
- `release` (on tag `v*`): `cargo publish` the crate + attach Tauri installers to the Release.

## Hard-won rules (see docs/LESSON.md for the full list)
- 💰 **Money-critical — never blindly retry a financial command.** This terminal charges
  real cards. On a drop, reconnect the socket but do NOT re-send payments/reversals/
  pre-auths (double-charge); recover via `sendLastResult()` (command `G`). The decision
  lives in `crates/ecr17-protocol/src/retry.rs`, locked by unit tests. The session resets
  per-transaction state so it is reusable across reconnects.
- Keep the **codec/protocol/response** layers pure & sync (no I/O) → trivially unit-testable;
  put all I/O behind the async `Transport` trait.
- Nexi terminals close the TCP socket **between** transactions → detect the drop
  **proactively** with a non-destructive liveness probe (never write probe bytes on the
  peer's protocol stream).
- ECR17: status code is lowercase `'s'`; payment `'P'` = 167 bytes; progress
  `SOH`+20+`EOT` has no LRC; `decode()` treats the buffer as exactly one frame.
- The `Bash` tool is **git-bash** (heredocs / `git commit -F -`, never PowerShell here-strings).

## Conventions
- Rust 2021+ edition, latest stable toolchain. `cargo fmt` + `clippy -D warnings` clean.
- Commit messages: conventional style (no gitmoji); end with the `Co-Authored-By` trailer.
- Branch + PR per feature/subtask; keep CI green per push.
- Do NOT republish the full Nexi vendor docs — link the official public URL only.
