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
- [x] **MACRO 4 — Session & money-safety** (`feat/session-retry`)  ✅ MERGED (PR #6, d8672ba)
- [x] **MACRO 5 — Client + TCP** (`feat/client-and-tcp`)  ✅ MERGED (PR #7)
- [x] **MACRO 6 — Tauri backend** (`feat/tauri-backend`)  ✅ MERGED (PR #8, 26099a7)
  - [x] T6.1 `src-tauri` bridge: managed Ecr17Client<TcpTransport> state, one #[tauri::command] per command, events (ecr17:progress/receipt/connection)  ✅
  - [x] T6.2 backend serde-round-trip tests + tauri-check CI (webkit deps, cargo clippy+test)  ✅
- [ ] **MACRO 7 — Control panel UI** (`feat/control-panel-ui`)  ← IN PROGRESS (impl + tests done; committing)
  - [x] T7.1 TS data model + command metadata + result/PAN-mask/log/storage helpers  ✅
  - [x] T7.2 typed Tauri invoke/event bridge + `useEcr17` hook  ✅
  - [x] T7.3 components (ConnectionBar, CommandPalette, ParamsSheet, BusyOverlay, LogConsole, ConfigForm) + App + dark theme  ✅
  - [x] T7.4 Vitest (10) + Playwright (12, with a full `@tauri-apps/api` IPC mock) — all green  ✅
  - [ ] commit → local Copilot review → push → CI → PR → Copilot → merge
- [ ] MACRO 8 — Packaging, docs, release (`chore/release-1.0`): README, release CI,
      cross-port README links (align RN+Laravel first!), knowledge consolidation, publish+tag+release

## Current position
Session 2026-07-10. MACRO 0-6 merged (protocol crate COMPLETE + publishable; Tauri backend
bridge on main). On branch `feat/control-panel-ui`: full web UI — typed Tauri bridge,
`useEcr17` hook, command palette + dynamic params sheet (money→cents coercion, required
validation, PAN masking in the log), live log console with download, config form with
localStorage persistence, busy overlay, connection bar. Frontend gates ALL green: Biome
lint clean, `tsc --noEmit` clean, 10 Vitest, `vite build` ok, 12 Playwright (with a full
`window.__TAURI_INTERNALS__` invoke/event mock). NEXT: commit → local Copilot review →
push → CI (frontend-checks + e2e) → PR → Copilot → merge. Then MACRO 8 (README, release
CI, cross-port links, publish+tag+release).

Process note: small macro-tasks bundle their subtasks into a single PR → main (still the
full validation loop). Larger macros (4, 5, 7) may use sub-PRs to the macro branch.

## Notes / decisions
- Frontend: React 19 + TS + Vite (closest port of RN UI; Playwright + Vitest).
- Crate: single `ecr17-protocol` (core + tokio transport behind feature). App not published to crates.io.
- Release: crates.io lib + Tauri installers attached to GitHub Release.
- `tauri-cli` not installed on the dev box — only needed to run `tauri dev` / `tauri build`
  locally (deferred: the desktop/installer build runs in CI; local Tauri build is blocked
  anyway by GNU/windres + the spaced repo path — see docs/LESSON.md).
