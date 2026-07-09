# LESSON.md — accumulated learnings (rust-ecr17-protocol)

> **Context rule:** the content of this file MUST be passed into the prompt of
> every parallel subagent, and re-read at the start of every new session, so
> hard-won knowledge is never lost. Update it continuously — especially after
> Copilot/CI feedback and after fixing any bug.

## Environment & tooling
- Host is **Windows 11**. The `Bash` tool runs **git-bash** (POSIX sh); the
  `PowerShell` tool runs pwsh. ⚠️ Do **not** use PowerShell here-strings (`@'…'@`)
  inside the Bash tool — the `@` leaks into the arg. Use a bash heredoc
  (`<<'EOF' … EOF`) or `git commit -F -`.
- Toolchain verified 2026-07-10: rustc/cargo **1.96**, node **25**, npm 11, bun **1.3**,
  gh **2.88** (auth `lopadova`, git protocol **ssh**), copilot CLI **1.0.69**, git 2.55.
- `tauri-cli` is **not preinstalled** → `cargo install tauri-cli` (or scaffold via
  `npm create tauri-app@latest`). Tauri 2 is the current major.
- ⚠️ **The default `x86_64-pc-windows-msvc` toolchain is BROKEN on this machine** — the
  linker fails with `LNK1104: cannot open 'msvcrt.lib'` (MSVC/Windows-SDK libs not on the
  linker path; matches the RN reference's "MSVC VS18 broken" note). Fix: use the
  **GNU** toolchain, which bundles its own MinGW linker + libs and needs no MSVC/SDK:
  `rustup toolchain install stable-x86_64-pc-windows-gnu` (+ `rustup component add clippy
  rustfmt --toolchain stable-x86_64-pc-windows-gnu`) then a **directory-local**
  `rustup override set stable-x86_64-pc-windows-gnu` (NOT a committed `rust-toolchain.toml`
  — CI runs on Linux and must keep its own default). With GNU, `cargo build/test/clippy/fmt`
  and the `tokio-transport` feature all compile clean. NOTE: a Windows **Tauri** build may
  still prefer MSVC (WebView2) — do the installer build in CI on `windows-latest` (proper
  MSVC) rather than locally.
- Latest crate versions (2026-07-10): tokio 1.52.3, serde 1.0.228, thiserror 2.0.18,
  async-trait 0.1.89, serde_json 1.0.150, tauri 2.11.5.
- `copilot` CLI is present for the local review loop. It **edits & commits in
  `--yolo`** — treat output as proposals to VERIFY, never trust blindly.
- Repo remote: `git@github.com:padosoft/rust-ecr17-protocol.git`, default branch `main`.

## crates.io
- Registry = **crates.io**, driven by Cargo. Publish: `cargo login` → set Cargo.toml
  metadata → `cargo publish --dry-run` → `cargo publish`. Names are global/unique.
- `ecr17-protocol` verified **free** on 2026-07-10 (crates.io API 404 = available).

## Porting map (C++ reference → Rust)
- `Lcr` → `lrc.rs`; `PacketCodec` → `codec.rs`; `Ecr17Protocol` → `protocol.rs`;
  `Ecr17Response` → `response.rs`; `Transport`/`FakeTransport` → `transport.rs`;
  `RetryPolicy.hpp` → `retry.rs`; `Ecr17Session` → `session.rs`;
  `HybridEcr17Client` → `client.rs`; Kotlin/Swift native TCP → `transport/tcp.rs` (tokio).
- The RN native bridge (Nitro/JNI) has **no equivalent** in Rust/Tauri — tokio TCP is
  native and cross-platform, so all the RN JNI/threadscope lessons are N/A here.

## ECR17 protocol facts (must hold — from the reference)
- App frame = `STX(0x02)` payload `ETX(0x03)` `LRC`. LRC base `0x7F`, XOR-folded; the
  folded bytes are selected by `LrcMode` (`stx`/`std`/`noext`/`stx_noext`).
- Progress = `SOH(0x01)` + 20-char message + `EOT(0x04)`, **no LRC**; `decode()` rejects
  an SOH frame whose last byte != EOT.
- Status code is lowercase `'s'`. Payment `'P'` request = **167 bytes**.
- Receipts = one or more `S` messages (concatenate). Reversal request = `'S'`.
- `decode()` treats the buffer as exactly one frame (LRC = final byte); stream→frame
  splitting belongs to the transport layer.
- Outcome map: `"00"→ok`, `"01"→ko`, `"05"→cardNotPresent`, `"09"→unknownTag`.

## Money-safety (💰 non-negotiable)
- A financial command (pay/reverse/preAuth/closure/incremental) is **NEVER blindly
  re-sent** after a reconnect (double-charge). `RetryPolicy` (`retry.rs`) allows retry
  only for safe/idempotent ops (status/totals). Recover a lost response via
  `sendLastResult()` (spec command `G`). The session resets per-transaction state
  (`reset_for_new_transaction`) so it is reusable across reconnects.
- Nexi terminals **close the TCP socket between transactions** → detect the drop
  **proactively** (a non-destructive liveness probe / peek before sending), not
  reactively after the send. Never write bytes on the peer's protocol stream to probe
  (the RN bug: `sendUrgentData(0xFF)` corrupted the next frame under `SO_OOBINLINE`).

## Rust/Tauri specifics (fill in as we learn)
- (session/client) prefer an async `Transport` trait; keep the codec/protocol/response
  layers **pure & sync** (no I/O) so they are trivially unit-testable — mirrors why the
  C++ unit target excluded the client/adapter.
- (Tauri) hold `Ecr17Client` in managed state; one `#[tauri::command]` per protocol
  command; emit `progress`/`receiptLine`/`connectionState` as Tauri events.
- (e2e) Playwright drives the Vite frontend with Tauri IPC mocked
  (`@tauri-apps/api/mocks` `mockIPC`) for deterministic UI coverage without a POS.

## Review/CI learnings
- (to be filled after the first Copilot/CI cycles)

## Legal
- Public Nexi web docs are NOT free to republish; attribution ≠ license. Link the
  official public URL only; do not vendor the full vendor PDF into the repo.
