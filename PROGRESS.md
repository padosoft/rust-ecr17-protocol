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
  - [x] T0.1 Process assets (.claude rules/skills, AGENTS/CLAUDE/LESSON/PROGRESS/PLAN)  ← doing
  - [ ] T0.2 Cargo workspace + `ecr17-protocol` crate skeleton compiling
  - [ ] T0.3 Tauri app scaffold (React19+TS+Vite+Tauri2) + Vitest + Playwright wired
  - [ ] T0.4 CI skeleton (rust-tests, frontend-checks, e2e) green
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
Session 2026-07-10. Design approved. Branch `chore/bootstrap` created off `main`
(only Initial commit). crates.io name `ecr17-protocol` verified free. Writing T0.1
process assets. NEXT: T0.2 Cargo workspace scaffold.

## Notes / decisions
- Frontend: React 19 + TS + Vite (closest port of RN UI; Playwright + Vitest).
- Crate: single `ecr17-protocol` (core + tokio transport behind feature). App not published to crates.io.
- Release: crates.io lib + Tauri installers attached to GitHub Release.
- `tauri-cli` not yet installed on this machine — install before T0.3.
