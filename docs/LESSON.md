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
- **Two layers, two strictnesses (MACRO 4 review):** `codec::decode` recognizes a control
  frame by its LEAD BYTE only (lenient — the money-critical rule that a real 3-byte
  `ctrl+ETX+LRC` ACK is accepted). But the session's `extract_frame` is the gatekeeper that
  splits the stream, and it now FULLY validates a control frame (`ETX` at [1] AND the
  control-frame LRC at [2]) before draining it; a stray/corrupted sequence that merely
  starts with `0x06`/`0x15` is dropped and resynced, so a desynced or corrupted ACK can't
  prematurely complete a handshake. This goes BEYOND the C++ reference (which sliced 3
  bytes on the lead byte) — a deliberate robustness improvement for money code. Locked by
  `stray_ack_byte_is_resynced_not_a_false_ack` + `control_frame_with_bad_lrc_is_resynced`.
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

## Response parser known-limitations (MACRO 3, from Codex P2 review — deliberate)
- `parse_payment` treats only uppercase `'V'` as a DCC response. Codex flagged that pre-auth
  **closure** DCC responses use a lowercase `'v'` with a DCC block at a different offset
  (~pos 75). The C++ reference doesn't handle this either; adding untested offsets into
  money-critical parsing is riskier than the documented gap → left as a known limitation to
  validate against a real terminal (env-gated integration test, MACRO 5).
- The shared payment-family parser doesn't extract a **reversal action code** (pos 72-74 per
  the Nexi reversal doc). `PaymentResponse` (like the C++) has no such field, so
  `ReversalResult.action_code` stays empty via this path. Same rationale: faithful to the
  tested reference; extend only with real-terminal validation.
- Principle: for a faithful port of tested money code, do NOT speculatively add parse offsets
  from docs we can't verify in-repo — a documented gap beats an untested wrong offset.

## Async session port (MACRO 4)
- The C++ session used a background reader THREAD + condition_variable. The Rust async
  port needs **no reader task**: the session `await`s `transport.recv()` when it needs
  bytes, wrapped in `tokio::time::timeout` for ack/response deadlines. Sequential
  send→recv within an exchange (ECR17 is one transaction at a time) → the `Transport`
  trait needs no interior concurrency.
- `FakeTransport::recv` when idle does `std::future::pending().await` — the session's
  `timeout(remaining, recv())` cancels it, giving deterministic timeout tests without real
  waits (except the tiny 40ms fast-config deadlines).
- Money-safety reset: the Rust session holds NO persistent `disconnected` flag (a drop is
  observed transiently as `recv() -> Err(Disconnected)`), so `reset_for_new_transaction`
  only clears `rx_buffer` + `pending_result`. This makes the session reusable across
  reconnects (regression `recovers_and_succeeds_after_reconnect`) — a stale flag can never
  block a fresh transaction.
- `retry.rs::should_retry_after_reconnect` is a PURE function (financial → never retried);
  the SESSION just errors on drop, and the CLIENT (MACRO 5) applies the retry decision +
  reconnect. Keep the money decision in one tiny, unit-locked place.
- Made `tokio` + `async-trait` non-optional (features time/sync/rt; `macros` is a
  dev-dependency only, for `#[tokio::test]`) so the session/
  transport/retry are always testable with plain `cargo test`; the `tokio-transport`
  feature now gates ONLY the real TCP socket (net/io-util). `From<io::Error>` for
  `Ecr17Error` is feature-gated; the `Transport { kind: io::ErrorKind, message }` variant is
  always present (io::ErrorKind is Clone+Eq, preserving the error derives).

## Client + TCP transport (MACRO 5)
- The client's progress/receipt/connection-state callbacks use
  `Arc<Mutex<Option<Arc<dyn Fn(T) + Send + Sync>>>>` shared with the session: at construction
  the session's `set_on_progress`/`set_on_receipt_line` closures capture the outer Arc and
  forward to whatever the consumer later registers via `set_on_*` (Rust equivalent of the
  C++ client capturing `this`). ⚠️ The inner `Arc<dyn Fn>` is cloned OUT of the mutex and
  invoked with the lock RELEASED — never hold a lock across a user callback (re-entrancy
  deadlock if the callback re-registers itself). That requires the callback be `Send + Sync`.
- The money-safe auto-reconnect lives in `client::run_transaction`/`run_ack_only` →
  `recover_after_error` → `should_retry_after_reconnect`: on a mid-command error, reconnect
  (if `auto_reconnect` + dropped) then replay ONLY safe/idempotent ops; a financial op
  surfaces the error (recover via `send_last_result`/`G`). Tests
  `financial_command_not_replayed_on_drop` + `safe_command_retried_after_reconnect`.
- `PosStatusResponse.terminal_date_time` is produced as ISO 8601 by
  `client::raw_datetime_to_iso` (raw `DDMMYYhhmm` → `20YY-MM-DDThh:mm:00`).
- **TCP liveness probe:** `TcpTransport::is_connected` uses `TcpStream::poll_peek` with
  `std::task::Waker::noop()` for a synchronous, non-destructive check — `Poll::Ready(Ok(0))`
  = peer FIN (dead), `Ok(n>0)` = data buffered (alive), `Pending` = open/idle (alive). This
  detects the between-transaction TCP close BEFORE sending, so a financial command never
  starts on a stale socket. `poll_peek` returns `Poll<io::Result<usize>>` (byte count), NOT
  `()`. `Waker::noop()` is stable since Rust **1.85** → the crate MSRV is 1.85 (a manual
  noop waker would need `unsafe`, which `#![forbid(unsafe_code)]` disallows).
- Real TCP transport is covered by local `TcpListener` tests (roundtrip, peer-close→EOF,
  not-connected) under `--features tokio-transport`, plus an `#[ignore]` env-gated
  (`ECR17_TEST_HOST`) real-terminal `status()` integration test.

## Rust/Tauri specifics (fill in as we learn)
- (session/client) prefer an async `Transport` trait; keep the codec/protocol/response
  layers **pure & sync** (no I/O) so they are trivially unit-testable — mirrors why the
  C++ unit target excluded the client/adapter.
- (Tauri) hold `Ecr17Client` in managed state; one `#[tauri::command]` per protocol
  command; emit `progress`/`receiptLine`/`connectionState` as Tauri events.
- (e2e) Playwright drives the Vite frontend with Tauri IPC mocked
  (`@tauri-apps/api/mocks` `mockIPC`) for deterministic UI coverage without a POS.

## Frontend / control panel (MACRO 7)
- **Tauri v2 IPC contract (verified by grepping `node_modules/@tauri-apps/api`):** the
  frontend does NOT need `@tauri-apps/api/mocks`. `invoke(cmd, args)` calls
  `window.__TAURI_INTERNALS__.invoke(cmd, args, opts)`; `listen(event, cb)` calls
  `invoke("plugin:event|listen", { event, target, handler: transformCallback(cb) })` and
  the runtime later calls `window.__TAURI_INTERNALS__.callbacks[id]` (registered via
  `transformCallback`). Our Playwright mock (`e2e/tauri-mock.ts`) installs
  `window.__TAURI_INTERNALS__.{invoke,transformCallback,unregisterCallback}`, special-cases
  `plugin:event|listen` to stash the handler, and exposes `window.__ecr17mock` with
  `setResponse/setError/setDelayed/emit/reset`. This is more faithful than mockIPC (tests
  the real event path) and needs no bundler cooperation.
- **Command names are snake_case, arg keys are camelCase.** A Rust `#[tauri::command] fn
  pay_extended(request: …)` is invoked as `invoke("pay_extended", { request })`. Serde
  `rename_all="camelCase"` only touches the *struct fields*, not the command name or the
  top-level arg key (which mirrors the Rust *parameter* name). Getting either wrong yields
  a silent "command not found" at runtime.
- **React event pooling bites `setState` updaters:** reading `e.currentTarget.value`
  *inside* a `setValues(v => …)` updater returns null — the updater runs after the event
  object is recycled. Capture `const val = e.currentTarget.value` BEFORE calling the
  setter. (Hit in every onChange of the params sheet.)
- **Biome a11y on a click-to-close modal backdrop:** `noStaticElementInteractions` +
  `useKeyWithClickEvents` fire on a `<div onClick>` backdrop, and `stopPropagation` on an
  inner div trips them again. Clean fix that keeps click-outside-to-close: render a
  full-screen transparent `<button aria-label="Close">` *behind* the panel (z-index) for
  the outside-click, give the panel `role="dialog" aria-modal`, and add a `window` Escape
  keydown listener for the keyboard path — no `stopPropagation`, no biome-ignore.
- **Biome `noLabelWithoutControl` + conditional control:** a `<label>` whose control is
  rendered via a ternary can't be statically proven to wrap a control, so biome flags it
  even though the association is valid at runtime. Fix explicitly with `htmlFor={id}` on
  the label + matching `id` on each branch's input (use `useId()` for a stable base).
- **Biome `useExhaustiveDependencies` for "run on change X" effects:** an effect that must
  re-run when a value changes but whose body only reads refs (e.g. auto-scroll on new log
  entries) is legitimately flagged. A `// biome-ignore lint/correctness/useExhaustiveDependencies:
  <reason>` on the line above the `useEffect` is the correct, honest resolution.

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
- **Copilot local review (MACRO 7 control-panel UI):** 10 items, verdicts anchored to the
  RN reference as SSOT:
  1. ACCEPTED — required money/number fields lacked a positive-amount guard (`isMissing`
     only checked non-empty), so `pay`/`preAuth`/… accepted `0`/negative. The RN reference
     explicitly guards `typeof v !== "number" || v <= 0` ("guard against a zero-amount
     financial transaction"). Ported it + added a regression test (`keeps submit disabled
     for a zero or negative amount`).
  2. REJECTED — "regex-scrub embedded PANs in errorDescription/rawXml": the SSOT
     `maskSensitive` masks ONLY the `pan` key; a 13–19-digit regex would over-mask STANs
     and diverge from the reference. Mine already matches the SSOT exactly.
  3. REJECTED — "applyConfig before every run rebuilds the client (money-safety)": the RN
     reference ALSO calls `applyConfig` inside `run()`, gated on a config *change*. The only
     difference is my Tauri backend's `configure()` disconnects+rebuilds (RN updates in
     place). But a config edit is an explicit operator action, only fires between
     transactions (command buttons disabled while `busy`), and the follow-up command's
     `ensure_connected()` is a FIRST send — not a replay. Double-charge protection lives in
     `recover_after_error`/`should_retry_after_reconnect`, which still refuses to replay
     financial commands. Faithful + safe → keep.
  4–7,10. REJECTED — `isFailure` actionCode nuance, `vas ${responseId}` toast fallback,
     unused `"number"` FieldKind, `maskPan("123")→"****"`, no upper amount bound: all match
     the RN reference behavior verbatim (e.g. reference `maskPan` returns `'****'` for
     `digits.length <= 4`). Faithful port → no change.
  8–9. Confirmations (no action): Tauri IPC snake_case-command / camelCase-arg contract
     verified against `lib.rs`; no blind financial retry (Disconnect disabled while busy).
  Takeaway: for a faithful port, the SSOT is the arbiter — accept a review item only when
  the reference agrees (here #1) and reject "improvements" that would diverge (#2), citing
  the reference each time.
- **Remote PR review (MACRO 7, PR #9) — Codex + Copilot:** 7 comments; 5 accepted, 2 rejected.
  - Codex P2 "validate money AFTER cent conversion" — ACCEPTED (real divergence). The RN
    `MoneyField` stores integer **cents** in the param and validates the cents (`v <= 0`), so
    `0.004€ → round(0.4) = 0 → disabled`. My port stored euros-strings and validated raw
    euros (`0.004 > 0 → enabled`), then coerced to 0 cents → a zero-value financial send.
    Fixed `isMissing` to validate the **coerced** value (`coerce(field, raw) <= 0`), matching
    the reference's effective behavior; added a sub-cent regression test. Lesson: when a port
    changes WHERE a value is converted (keystroke vs submit), re-check every guard that ran on
    the old representation.
  - Codex P2 "confirm no-field danger commands (closeSession)" — REJECTED. The RN reference
    `onPick` is byte-identical: `if (cmd.fields.length === 0) doRun(cmd.key, {})` — it also
    auto-runs `closeSession` (danger, no fields) with no confirm. Faithful to SSOT → no change.
  - Copilot "e2e mock resolves null for any unconfigured command → hides typos" — ACCEPTED
    (test hardening, no SSOT bearing). Mock now resolves null only for the 5 void backend
    commands (`Result<(), String>`: configure/connect/disconnect/enable_ecr_printing/reprint)
    and THROWS for any other unmocked command, so a forgotten `setResponse` fails loudly.
  - Copilot "showToast setTimeout not cancelled" — ACCEPTED. Rapid runs let an older timer
    clear a newer toast; store the id in a `useRef` and `clearTimeout` before rescheduling
    (+ clear on unmount).
  - Copilot "ConfigForm numOrUndef yields floats → u16/u32 IPC deserialize fails" — ACCEPTED.
    `Number("1.5")=1.5` would fail serde on the Rust side; switched to `Number.parseInt` and
    clamp `port` to 0..65535.
  - Copilot ×2 "PROGRESS.md vitest count off" — ACCEPTED (doc). Reconciled to the actual count.
  Note: Codex/Copilot on a Tauri port are strong at surfacing IPC-type and JS-representation
  bugs (float→uint, euros→cents) that the RN reference never had because RN typed the value
  natively — these are *port-introduced* and worth accepting even though the SSOT is silent.
- **Remote PR review (MACRO 7, PR #9) — second Copilot pass** after the fixes: 5 new polish
  nits, all accepted (web-only, no SSOT bearing): download() revokes the object URL on the
  next tick (+append/remove the anchor) so the browser starts the download first; money
  `<input type=number>` gets `step="0.01"` (integers `step="1"`) so `6.50` isn't marked
  invalid; the mock throws `new Error(...)` instead of raw strings; the sheet close "✕"
  button gets `aria-label="Close"`. Codex re-raised the no-field-danger-confirm P2 a second
  time — REJECTED again (SSOT `onPick` auto-runs it; a confirm dialog would diverge). Note:
  GitHub re-anchors a bot's ORIGINAL review comments onto the newest commit, so already-fixed
  items reappear in the comment list — dedupe by comment **id** (the first pass was ids
  `35596…`, genuinely-new ones `35651…`) rather than assuming every listed comment is fresh.
- Copilot also flagged `originalPreAuthCode: str(...) ?? ""` in the dispatcher as a "silent
  empty default that can send an invalid incremental-auth / pre-auth-closure request" —
  REJECTED: byte-identical to the RN reference SSOT, AND already unreachable via the UI
  (the field is `required: true` in commands.ts, so the params sheet disables submit until
  it is filled). The `?? ""` is only a type-satisfying fallback for the required backend
  field. Lesson: a "missing required value" finding is moot when an upstream required-field
  guard already blocks the empty case — verify the whole path before "hardening" a fallback.

## Packaging, docs & release (MACRO 8)
- **README is authored ONCE, in `crates/ecr17-protocol/README.md`** (the crates.io
  front page) and mirrored **byte-identically** to the root `README.md` by
  `scripts/sync-readme.mjs`. It works because every badge/image/link uses an
  **absolute** URL (`https://raw.githubusercontent.com/padosoft/rust-ecr17-protocol/main/…`)
  — no path rewriting, so a plain copy is correct on both GitHub and crates.io.
  `node scripts/sync-readme.mjs` writes the mirror; `--check` exits 1 if stale.
  A `readme-check` CI job runs `--check` on every push/PR so a stale mirror never
  lands. **Never hand-edit the root `README.md`** — edit the crate copy and re-sync.
- **Screenshots/banner are generated, not hand-shot.** `app/scripts/capture-assets.mjs`
  drives Playwright's bundled Chromium: the banner is a standalone `setContent`
  HTML page (no server), and the two app shots load the real Vite dev server on
  `:1420` with the SAME `window.__TAURI_INTERNALS__` IPC mock the e2e suite installs
  (inlined so the script is standalone). To (re)generate: `bun run dev` in `app/`
  (wait for `:1420`), then `node scripts/capture-assets.mjs` → `resources/banner.png`
  + `resources/screenshots/{control-panel,params-sheet}.png`. Deterministic, no POS.
  - Banner layout gotcha: the 💳 emoji glyph renders WIDER than its `font-size` box
    and bleeds left, so the subtitle clipped behind it. Fix = shrink the coin
    (168px) + push it right (`right:60px`) + cap the `<p>` `max-width:820px` so text
    wraps before the card. Always eyeball the rendered PNG (Read it) after a change.
- **Release pipeline (`.github/workflows/release.yml`, on tag `v*`):** two jobs.
  (1) `publish-crate` on ubuntu: verifies the tag == crate version, then a **guarded,
  idempotent** publish — `if: secrets.CARGO_REGISTRY_TOKEN != ''` (no-ops when the
  secret is absent) and skips when the crates.io API already returns 200 for that
  version (so a manual publish doesn't turn the release red). (2) `build-installers`
  matrix (ubuntu-22.04 / windows-latest / macOS aarch64 + x86_64) using
  **`tauri-apps/tauri-action@v0`**, which builds the bundles (`bundle.targets:"all"`)
  and attaches them to the GitHub Release (needs `permissions: contents: write` +
  `GITHUB_TOKEN`). Linux needs the same apt webkit deps as `tauri-check.yml`; macOS
  needs the target added via `dtolnay/rust-toolchain{targets}`.
- **crates.io API REQUIRES a descriptive `User-Agent`** — a `curl` with the default
  UA gets **403** (not 404), so the release.yml idempotency probe must send
  `curl -A "..."`, else a brand-new version reads as "inconclusive" and never
  auto-publishes. Verified: no-UA → 403, `-A "..."` → 200 for a published version.
- **v1.0.0 RELEASED (2026-07-12):** `cargo publish -p ecr17-protocol` succeeded from a
  clean `main` (GNU toolchain) → **live on crates.io** (`max_version` 1.0.0, 17 files,
  ~46 KiB). Tag `v1.0.0` pushed → `release.yml` builds the installer matrix + creates the
  GitHub Release. NOTE: two stray "release" workflow runs showed `failure`/0s on the
  branch pushes — startup noise, harmless; the real run is the one on the `v1.0.0` tag.
- **crates.io publish is done LOCALLY for v1.0.0** — a usable token is in
  `~/.cargo/credentials.toml` (`[registry]`), and there is **no `CARGO_REGISTRY_TOKEN`
  repo secret** (verified `gh secret list` empty). `cargo publish --dry-run -p
  ecr17-protocol --allow-dirty` packages 17 files, ~46KiB compressed, verifies clean
  under the GNU toolchain. The GNU box builds the pure crate fine (only the Tauri
  *app* is blocked locally — installers come from CI). Did NOT copy the user's token
  into a GH secret unprompted; the CI publish job stays green by being guarded.
- **A version bump touches FOUR files:** `crates/ecr17-protocol/Cargo.toml`,
  `app/src-tauri/Cargo.toml`, `app/src-tauri/tauri.conf.json`, `app/package.json`
  — plus regenerate both lockfiles (`cargo update -p ecr17-protocol` at the root and
  `cargo update -p app` in `app/src-tauri`; pure resolution, no build, so the spaced
  path / windres issue doesn't bite).
- **CI gotcha — `secrets` is NOT allowed in `if:` conditionals (Codex P1, PR #10) AND
  don't hoist a secret to job-level `env` to work around it (over-broad exposure).**
  `if: ${{ secrets.CARGO_REGISTRY_TOKEN != '' }}` is INVALID — GitHub Actions doesn't
  expose the `secrets` context to `if:`. The tempting fix (map it to a **job-level**
  `env` and test `if: env.X != ''`) works but leaks the token into EVERY step of the
  job (the commit security review flagged it). Correct fix: **no `if:` at all** — put
  the secret on the **publish step's own `env:`** (least exposure) and do the skip in
  the script: `if [ -z "$CARGO_REGISTRY_TOKEN" ]; then echo skip; exit 0; fi`. Same
  no-op-when-absent + idempotent behavior, secret scoped to the one step that needs it.
- **PR #10 had TWO bots reviewing** (like MACRO 7): Copilot (2 nits — camelCase method
  names in the money-safety prose → snake_case; `workflow_dispatch` running tag-only
  jobs on a branch → `if: startsWith(github.ref,'refs/tags/v')` on both jobs) and
  Codex (the P1 `secrets`-in-`if` above). All three accepted; all valid.
- **Sibling cross-port links (T8.3):** the RN + Laravel repos each carry an "other
  ports" callout that must now list Rust/Tauri. Their local clones live at
  `../../ReactNative/react-native-ecr17-protocol` and `C:/xampp/htdocs/laravel-ecr17`
  — **`git fetch` + `git pull` FIRST** (they may be stale), edit near the top of the
  README, commit, push to `main`.

## Legal
- Public Nexi web docs are NOT free to republish; attribution ≠ license. Link the
  official public URL only; do not vendor the full vendor PDF into the repo.
