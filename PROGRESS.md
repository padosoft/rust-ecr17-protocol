# PROGRESS — rust-ecr17-protocol

Crash-safe resume log. Update at every subtask boundary. On a new session read:
**AGENTS.md → this file → docs/LESSON.md → docs/PLAN.md**.

Plan: `docs/PLAN.md`. Reference package: `../ReactNative/react-native-ecr17-protocol`.

## Per-task Definition of Done (loop)
1. Local tests green (`cargo test` + `clippy`/`fmt`; `vitest`/`tsc`; `playwright` if UI).
2. Local Copilot review: `copilot --autopilot --yolo -p "/review <branch diff vs origin/main>"`
   (diff to temp file if large). Record learnings in `docs/LESSON.md`.
3. Zero actionable comments → continue; else fix and go to 1.
4. Push; CI all green; else fix → local loop.
5. PR (subtask→macro branch; macro→main); Copilot as reviewer; wait CI + Copilot; fix loop.
6. Zero actionable → merge → update this file. Next task.

## Macro-task status
- [ ] **MACRO 0 — Governance & scaffolding** (`chore/bootstrap`)  ← IN PROGRESS
  - [x] T0.1 Process assets (.claude rules/skills, AGENTS/CLAUDE/LESSON/PROGRESS/PLAN)  ✅
  - [x] T0.2 Cargo workspace + `ecr17-protocol` crate skeleton compiling  ✅ (build/test/clippy/fmt green; GNU toolchain)
  - [x] T0.3 Tauri app scaffold (React19+TS+Vite+Tauri2) + Vitest + Playwright wired  ✅ (typecheck+vitest+build+playwright green locally; backend built in CI only)
  - [x] T0.4 CI skeleton (rust-tests, frontend-checks, e2e) authored  ← running copilot review → push → PR → CI green → merge
- [ ] MACRO 1 — Protocol primitives (`feat/protocol-primitives`): lrc, codec
- [ ] MACRO 2 — Message builders (`feat/protocol-builders`): types, protocol
- [ ] MACRO 3 — Response parsers (`feat/protocol-parsers`): response
- [ ] MACRO 4 — Session & money-safety (`feat/session-retry`): transport, retry, session
- [ ] MACRO 5 — Client + TCP (`feat/client-and-tcp`): client, tcp, crate polish
- [ ] MACRO 6 — Tauri backend (`feat/tauri-backend`)
- [ ] MACRO 7 — Control panel UI (`feat/control-panel-ui`)
- [ ] MACRO 8 — Packaging, docs, release (`chore/release-1.0`): README, release CI,
      cross-port README links (align RN+Laravel first!), knowledge consolidation, publish+tag+release

## Current position
Session 2026-07-10. Branch `chore/bootstrap`. T0.1–T0.3 DONE & committed. Tauri app
scaffolded in `app/` (excluded from the Cargo workspace); frontend guardrails green
locally (typecheck, Vitest 1✓, Vite build, Playwright 1✓). Tauri backend build is
CI-only here (GNU/windres + spaced path — see LESSON). NEXT: T0.4 add CI workflows
(`rust-tests`, `frontend-checks`, `e2e`) + push branch + open first PR toward main.

## Notes / decisions
- Frontend: React 19 + TS + Vite (closest port of RN UI; Playwright + Vitest).
- Crate: single `ecr17-protocol` (core + tokio transport behind feature). App not published to crates.io.
- Release: crates.io lib + Tauri installers attached to GitHub Release.
- `tauri-cli` not installed on the dev box — only needed to run `tauri dev` / `tauri build`
  locally (deferred: the desktop/installer build runs in CI; local Tauri build is blocked
  anyway by GNU/windres + the spaced repo path — see docs/LESSON.md).
