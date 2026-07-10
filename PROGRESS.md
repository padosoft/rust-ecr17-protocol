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
- [x] **MACRO 0 — Governance & scaffolding** (`chore/bootstrap`)  ✅ MERGED (PR #1, squash da326f8)
  - [x] T0.1 Process assets · T0.2 Cargo workspace · T0.3 Tauri scaffold · T0.4 CI  ✅
- [x] **MACRO 1 — Protocol primitives** (`feat/protocol-primitives`)  ✅ MERGED (PR #2, fb4119b)
  - [x] T1.1 `lrc.rs` · T1.2 `codec.rs` — tests ported from the C++ reference  ✅
- [x] **MACRO 2 — Message builders** (`feat/protocol-builders`)  ✅ MERGED (PR #4, 9568454)
  - [x] T2.1 `types.rs` + `error.rs` · T2.2 `protocol.rs` (all builders) — tests ported  ✅
- [x] **MACRO 3 — Response parsers** (`feat/protocol-parsers`)  ✅ MERGED (PR #5, 4b77509)
  - [x] T3.1 `response.rs` (all raw parsers + outcome + DccInfo) — tests ported  ✅
- [ ] **MACRO 4 — Session & money-safety** (`feat/session-retry`)  ← IN PROGRESS (impl done)
  - [x] T4.1 `transport.rs` (async Transport trait + FakeTransport)  ✅
  - [x] T4.2 `retry.rs` (💰 RetryPolicy — financial never replayed)  ✅
  - [x] T4.3 `session.rs` (ACK/NAK + retransmit + timeout + drain + reset) — tests ported  ✅
  - [ ] local Copilot review → push → PR → CI + Copilot → merge
- [ ] MACRO 5 — Client + TCP (`feat/client-and-tcp`): client, tcp, crate polish
- [ ] MACRO 6 — Tauri backend (`feat/tauri-backend`)
- [ ] MACRO 7 — Control panel UI (`feat/control-panel-ui`)
- [ ] MACRO 8 — Packaging, docs, release (`chore/release-1.0`): README, release CI,
      cross-port README links (align RN+Laravel first!), knowledge consolidation, publish+tag+release

## Current position
Session 2026-07-10. MACRO 0-3 merged. On branch `feat/session-retry`: `retry.rs`
(pure money-safety), `transport.rs` (async Transport trait + FakeTransport), `session.rs`
(async ACK/NAK+retransmit+timeout+drain, reusable across reconnects) ported; 94 tests
green, clippy/fmt/doc clean, default + tokio-transport feature build. NEXT: local Copilot
review → push → PR → merge. Then MACRO 5 (client + real TCP transport).

Process note: small macro-tasks bundle their subtasks into a single PR → main (still the
full validation loop). Larger macros (4, 5, 7) may use sub-PRs to the macro branch.

## Notes / decisions
- Frontend: React 19 + TS + Vite (closest port of RN UI; Playwright + Vitest).
- Crate: single `ecr17-protocol` (core + tokio transport behind feature). App not published to crates.io.
- Release: crates.io lib + Tauri installers attached to GitHub Release.
- `tauri-cli` not installed on the dev box — only needed to run `tauri dev` / `tauri build`
  locally (deferred: the desktop/installer build runs in CI; local Tauri build is blocked
  anyway by GNU/windres + the spaced repo path — see docs/LESSON.md).
