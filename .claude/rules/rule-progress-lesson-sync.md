# PROGRESS & LESSON sync

Two files keep work crash-safe and knowledge durable. Keep them current — they are the
contract that lets any session (or sub-agent) resume without losing context.

## PROGRESS.md
- Update at **every subtask boundary**: check off what's done, set the "Current position"
  block (branch, what just happened, the exact NEXT step), and record any decision.
- On a new session, read `AGENTS.md → PROGRESS.md → docs/LESSON.md → docs/PLAN.md` before acting.

## docs/LESSON.md
- Append a lesson after **every** Copilot/CI fix, bug fix, or non-obvious discovery
  (environment quirk, protocol detail, Rust/Tauri API gotcha, review takeaway).
- **Always pass the full content of `docs/LESSON.md` into the prompt of any sub-agent you
  spawn**, and re-read it at session start.
- Never delete a lesson because it looks obvious in hindsight; only correct it if it was wrong.

If a change affects public API, config fields, protocol/transport behavior, or safety
rules, also update the README/docs in the same branch.
