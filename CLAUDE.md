# CLAUDE.md

This repository ships **first-class context for AI coding agents** ("vibe-coding
batteries included"). If you're an AI assistant working here, start with these:

- **[AGENTS.md](AGENTS.md)** — project guide, the mandatory per-task workflow, CI
  strategy, and verified Rust/Tauri know-how. **Read it first.**
- **[docs/PLAN.md](docs/PLAN.md)** — the authoritative implementation plan (macro-tasks,
  subtasks, guardrails).
- **[docs/LESSON.md](docs/LESSON.md)** — accumulated engineering lessons (environment,
  protocol facts, money-safety, Rust/Tauri specifics). Re-read at the start of every
  session, and pass its content into every sub-agent prompt you spawn.
- **[PROGRESS.md](PROGRESS.md)** — current task / resume state for crash-safe
  continuation across sessions.

## Non-negotiables

- 💰 **Money-critical:** a **financial command is never blindly re-sent** after a
  reconnect (double-charge risk). The decision lives in
  `crates/ecr17-protocol/src/retry.rs` and is locked by unit tests; recovery from a lost
  response is via `sendLastResult()` (spec command `G`).
- **Keep CI green** — `rust-tests` (the protocol core, fully unit-tested) + `frontend-checks`
  + `e2e`. Native installer builds are a heavier tag/manual workflow.
- **Per-task loop:** local tests → local Copilot review → push → CI green → PR → Copilot
  review → merge.
- The official protocol source is the public Nexi developer portal; do **not** re-publish
  the full vendor docs in the repo (kept local/private) — link the public URL.

## Sibling ports (keep cross-links in sync)
`padosoft/react-native-ecr17-protocol` (React Native / Nitro, C++ engine) and
`padosoft/laravel-ecr17` (PHP / Laravel). When releasing, this package's README links
both, and both siblings' READMEs link this Tauri/Rust package (see `docs/PLAN.md` T8.3).
