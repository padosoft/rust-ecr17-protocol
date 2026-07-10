# Money-safety (💰 non-negotiable)

This library drives a POS terminal that **charges real cards**. A financial command
(`pay`, `payExtended`, `reverse`, `preAuth`, `incrementalAuth`, `preAuthClosure`) must
**never be blindly re-sent** after a transport drop/reconnect — a blind retry can
**double-charge**.

Rules when touching `session.rs`, `retry.rs`, `client.rs`, or any transport code:

- The retry decision lives ONLY in `crates/ecr17-protocol/src/retry.rs` (`RetryPolicy`).
  Financial commands → **not** replayed. Safe/idempotent ops (`status`, `totals`) → may retry.
- Recovery from a lost response is via `sendLastResult()` (spec command `G`) — not a replay.
- The session must reset its per-transaction state (`reset_for_new_transaction`) at the
  start of every exchange so it is reusable across reconnects (a stale "disconnected"
  flag must never block a fresh transaction).
- Detect a dropped socket **proactively** (non-destructive liveness probe / peek BEFORE
  sending), never reactively after the send. A probe must **never write bytes** on the
  peer's protocol stream.
- Every change here MUST keep the `retry.rs` unit tests green and add a regression test
  for any new path. Do not weaken these tests to make a change pass.

If a Copilot/CI suggestion would relax any of the above, **reject it** with a written
reason and record the exchange in `docs/LESSON.md`.
