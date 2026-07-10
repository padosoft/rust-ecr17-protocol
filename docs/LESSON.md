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
  gh **2.88** (authenticated, git protocol **ssh**), copilot CLI **1.0.69**, git 2.55.
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
- ⚠️ **The Tauri backend does NOT build locally on this machine** — two compounding
  reasons: (1) the GNU toolchain compiles Windows resources with `windres`, which Tauri
  officially doesn't support (Tauri wants MSVC on Windows); (2) the repo path contains a
  **space** (`…\Visual Basic\…`) and `windres`/`cc1` choke on the unquoted path
  (`cc1.exe: warning: C:\Users\…\Visual: not a directory` → `tauri-winres` panics). This
  is in the `tauri-build`→`tauri-winres` build script, NOT our code. **CI is clean** (no
  space in the path; Linux/`windows-latest` MSVC). Consequence: verify the Tauri backend
  via **CI** (`cargo check`/build on ubuntu with webkit2gtk, and the installer matrix on
  the release job), and develop the backend logic behind plain unit-testable functions.
  Frontend tooling (Vite build, Vitest, Playwright) runs fine locally.
- Frontend stack scaffolded (T0.3): React 19.1 + Vite 7 + Tauri 2 + TS 5.8; test stack
  Vitest 3 (jsdom + Testing Library) + Playwright 1.5x (chromium) + Biome 2. E2E drives the
  Vite dev server on the Tauri-fixed port 1420; real UI scenarios mock the Tauri IPC with
  `@tauri-apps/api/mocks`.
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
- Status response date/time is a raw `DDMMYYhhmm` on the wire. The RN API exposes it as a
  JS `Date`. In Rust we keep `PosStatusResponse.terminal_date_time` as an **ISO 8601 String**
  (dependency-free; the frontend does `new Date(iso)`), and the MACRO 3 `response` parser
  converts raw `DDMMYYhhmm` → ISO. (Codex P2 review, PR #4.)
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

## Protocol port specifics (Rust)
- 💰 **`codec::decode` recognizes ACK/NAK by LEAD BYTE only — do NOT tighten to
  `data.len() == 1`.** On the wire an ACK/NAK is a **3-byte control frame**
  `ctrl + ETX + LRC` (that's what `encode_control` produces and what the C++ session's
  `extractFrameLocked` slices off — it reads exactly 3 bytes for a control frame). So
  `decode([ACK, ETX, LRC])` must return `Ack`. A Copilot review (MACRO 1) suggested adding
  a `len == 1` guard "for consistency"; that would make `decode` return `Unknown` for every
  real ACK → **every transaction's ACK handshake would fail**. Verified against
  `Ecr17Session.cpp` before rejecting. Locked by `decode_full_control_frame_from_encode_control`.
  Lesson: for money-adjacent code, validate a reviewer's "consistency" fix against the
  END-TO-END reference (session framing), not just the local function.
- The session owns stream→frame splitting (`extractFrameLocked`): ACK/NAK = 3 bytes,
  STX = up to ETX+LRC, SOH = up to EOT, unknown lead byte = drop 1 and resync. `decode`
  only ever sees ONE pre-framed frame — its "reject coalesced/trailing" guards are a
  belt-and-braces second line for STX/SOH.
- Receipt detection: an application payload is a receipt ('S' send-ticket) when
  `payload[9] == 'S'` (message code at position 10, 0-indexed 9) — port in `session.rs`.

## Data model (Rust, MACRO 2)
- serde `Option<T>` struct fields deserialize to `None` when the key is ABSENT — no
  `#[serde(default)]` needed. So request structs only require their non-Option fields
  (e.g. `amountCents`) and optionals are naturally omitted by the frontend.
- Match the TS string unions with `#[serde(rename_all = "camelCase")]` on structs (so
  `amount_cents` ⇄ `amountCents`) and on multi-word enums (`CardNotPresent`⇄`cardNotPresent`,
  `ClessMag`⇄`clessMag`, `UnscheduledOrOneClick`⇄`unscheduledOrOneClick`); single-word
  enums use `"lowercase"` (`Disconnected`⇄`disconnected`).
- Amounts are `i64` cents; `PaymentCardType::as_digit()` → `'0'..'3'`. In Rust there is no
  nitro namespace clash, so the DCC struct keeps the TS name `CurrencyExchange` (the C++
  `DccInfo` rename was only to avoid the generated nitro struct).
- Builders live in `protocol.rs` as pure `pub fn`s taking primitives (`&str`, `i64`, `char`,
  `bool`) and returning `Result<String, Ecr17Error>`; the enum→digit mapping happens at the
  client layer (MACRO 5). `clippy::too_many_arguments` is #[allow]ed on the payment builders
  (faithful to the fixed ECR17 field set; the ergonomic request structs wrap them).

## Rust/Tauri specifics (fill in as we learn)
- (session/client) prefer an async `Transport` trait; keep the codec/protocol/response
  layers **pure & sync** (no I/O) so they are trivially unit-testable — mirrors why the
  C++ unit target excluded the client/adapter.
- (Tauri) hold `Ecr17Client` in managed state; one `#[tauri::command]` per protocol
  command; emit `progress`/`receiptLine`/`connectionState` as Tauri events.
- (e2e) Playwright drives the Vite frontend with Tauri IPC mocked
  (`@tauri-apps/api/mocks` `mockIPC`) for deterministic UI coverage without a POS.

## Protocol facts — verified against the reference (do NOT "fix")
- **Receipt-text (128 bytes) is RIGHT-aligned** in the payment family (`P`/`X`/`p`) and
  pre-auth follow-ups (`i`/`c`): leading spaces, text at the tail. The C++
  `buildPaymentLike` uses `leftPad(receiptText, 128, ' ')` and its layout test asserts
  `substr(156,3)=="ABC"` ("text right-aligned"). A MACRO 2 Copilot review claimed text
  fields "should be left-justified" (right_pad) — REJECTED after checking the reference +
  test; switching to right_pad would misalign the field vs the terminal and break the
  layout test. Locked by `payment_receipt_text_is_right_aligned`. (By contrast, the `U`
  TAG *number* IS left-justified/`right_pad` — different field.)

## Review/CI learnings
- **Copilot local review (T0.4 bootstrap):** two comments, both correctly REJECTED after
  verification (receiving-code-review discipline):
  1. "`app/package-lock.json` missing → `npm ci` fails" — FALSE POSITIVE: the lockfile IS
     committed; Copilot only saw the *focused* diff (I deliberately excluded the huge
     lockfile from it). Lesson: when handing Copilot a focused diff, tell it which files
     were intentionally omitted, or it will flag them as missing.
  2. "`exclude = ['app']` in the root workspace is a no-op because `members` is explicit"
     — WRONG for a *nested* package: `app/src-tauri` lives under the workspace root, so
     without `exclude` cargo errors "current package believes it's in a workspace when
     it's not". Verified: `cargo metadata` from `app/src-tauri` runs cleanly *because of*
     the exclude. Keep it.
- Copilot `--autopilot --yolo -p "/review …"` with an explicit "do NOT edit, only report,
  <=N lines" instruction behaved read-only (0 file changes) and finished in ~1m40s on a
  ~560-line focused diff. Feeding the full 10k-line branch diff (mostly lockfile) would
  have timed out — keep the review diff focused on hand-authored files.
- **Requesting the remote Copilot PR reviewer:** `gh pr edit <n> --add-reviewer copilot`
  FAILS ("Could not resolve user with login 'copilot'"), and the GraphQL suggestedActors
  list does NOT include the review bot (only `copilot-swe-agent`/coding agents). The
  working recipe is the REST endpoint with the reviewer bot slug:
  `gh api -X POST repos/<owner>/<repo>/pulls/<n>/requested_reviewers -f "reviewers[]=copilot-pull-request-reviewer[bot]"`
  → the PR's `requested_reviewers` then shows `{login: "Copilot", type: "Bot"}`. Re-run it
  after each push to re-request the review.
- CI (rust-tests + frontend-checks + e2e) went green on the FIRST push of the bootstrap PR.

## Legal
- Public Nexi web docs are NOT free to republish; attribution ≠ license. Link the
  official public URL only; do not vendor the full vendor PDF into the repo.
