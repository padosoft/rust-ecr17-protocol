# PLAN — rust-ecr17-protocol (Rust core + Tauri control panel)

> Authoritative implementation plan. Port of **`@padosoft/react-native-ecr17`**
> (the tested, working RN/Nitro reference) to **Rust** (protocol engine +
> publishable crate) and **Tauri** (cross-platform control-panel demo app).
> Reference package: `../ReactNative/react-native-ecr17-protocol`.
> Sibling ports: `padosoft/react-native-ecr17-protocol`, `padosoft/laravel-ecr17`.

Read order for any agent/session: **AGENTS.md** → **PROGRESS.md** (resume state) →
**docs/LESSON.md** (accumulated lessons) → this file.

---

## 1. Goal

Reproduce, identically and idiomatically, the RN package's two deliverables:

1. **Protocol core** — a pure Rust library crate `ecr17-protocol` implementing the
   Italian **ECR17** payment protocol for **Nexi Group** POS terminals over LAN.
   Publishable to **crates.io** (`cargo publish`). Name confirmed free.
2. **Demo control panel** — a **Tauri** desktop app: a debug console to exercise
   every ECR17 command against a real Nexi POS, ported from the RN `example/` app.

The RN protocol engine is C++ (`package/cpp/`); it is the **SSOT** we port. The RN
native TCP transport (Kotlin/Swift) becomes a native **tokio TCP** transport in
Rust — no bridging layer needed.

## 2. Non-negotiables (carried from the reference)

- 💰 **Money-critical.** A financial command (pay/reverse/preAuth/closure/incremental)
  is **NEVER blindly re-sent** after a reconnect (double-charge risk). Decision lives
  in `retry.rs` (`RetryPolicy`), locked by unit tests. Recovery from a lost response is
  via `sendLastResult()` (spec command `G`). The session resets its per-transaction
  state so it is reusable across reconnects.
- **CI green is the gate.** `cargo test` (protocol core, fully unit-tested) is the
  primary correctness gate; frontend checks + Playwright e2e gate the UI.
- **Do not republish the full Nexi vendor docs** (public portal, not free to
  re-license). Link the official public URL only.

## 3. Architecture

```
rust-ecr17-protocol/
├─ Cargo.toml                        # [workspace] members = crates/*; app/ is EXCLUDED
│                                     #   (self-contained Tauri project; will path-dep the crate in MACRO 6)
├─ crates/ecr17-protocol/            # ← published library crate
│  ├─ Cargo.toml                     # name="ecr17-protocol"; feature "tokio-transport"
│  ├─ README.md                      # crates.io front page (mirror of root README)
│  └─ src/
│     ├─ lib.rs        # public re-exports
│     ├─ error.rs      # Ecr17Error (thiserror)
│     ├─ lrc.rs        # LrcMode enum + LRC checksum   (C++ Lcr)
│     ├─ codec.rs      # STX/ETX/LRC framing, SOH progress  (C++ PacketCodec)
│     ├─ types.rs      # requests/results/enums, serde     (types/client.ts)
│     ├─ protocol.rs   # all message builders             (C++ Ecr17Protocol)
│     ├─ response.rs   # all parsers, outcome, DCC, PAN mask (C++ Ecr17Response)
│     ├─ transport.rs  # async Transport trait + FakeTransport (C++ Transport/FakeTransport)
│     ├─ retry.rs      # RetryPolicy — money safety        (C++ RetryPolicy.hpp)
│     ├─ session.rs    # Ecr17Session: ACK/NAK, retransmit, timeout, drain (C++ Ecr17Session)
│     ├─ client.rs     # Ecr17Client async API + events    (C++ HybridEcr17Client)
│     └─ transport/tcp.rs   # tokio TCP transport (feature-gated) (Kotlin/Swift native)
├─ app/
│  ├─ src-tauri/       # Rust backend: managed Ecr17Client, #[tauri::command] per cmd, events
│  └─ src/             # React 19 + TypeScript + Vite frontend (control panel)
│     ├─ ecr17/        # commands.ts, logger, results (PAN mask), storage, useEcr17 → Tauri IPC
│     ├─ components/   # ConnectionBar, ConfigForm, CommandPalette, CommandParamsSheet, LogConsole, BusyOverlay
│     └─ e2e/          # Playwright (Tauri IPC mocked)
├─ .claude/  rules/ + skills/         # workflow guardrails, README-sync, money-safety
├─ docs/     PLAN.md · LESSON.md
├─ AGENTS.md · CLAUDE.md · PROGRESS.md · README.md · LICENSE
└─ .github/workflows/  rust-tests · frontend-checks · e2e · tauri-build · release
```

**Async model.** Core is `async` (tokio). `Transport` is an async trait. `FakeTransport`
gives deterministic, scripted replies for unit tests (incl. simulated mid-exchange
drops); `tcp.rs` is the real transport behind the `tokio-transport` feature so pure
consumers can depend on the protocol codec without an I/O runtime.

**Tauri bridge.** `Ecr17Client` lives in Tauri managed state (`Arc<Mutex<…>>` /
async-aware). One `#[tauri::command]` per protocol command mirrors the RN client
API. Native events (`progress`, `receiptLine`, `connectionState`) are emitted to the
webview — the analog of `useEcr17.ts` wiring.

**E2E strategy.** Playwright drives the Vite frontend with the **Tauri IPC mocked**
(`@tauri-apps/api/mocks` `mockIPC`) → deterministic full-UI coverage with no POS.
The real-terminal test stays an **env-gated** Rust integration test (mirrors the RN
`test_integration_terminal`).

## 4. Command set (ported verbatim)

`status s` · `pay P` · `payExtended X` · `reverse S` · `preAuth p` ·
`incrementalAuth i` · `preAuthClosure c` · `verifyCard H` · `closeSession C` ·
`totals T` · `sendLastResult G` · `enableEcrPrinting E` · `reprint R` · `vas K` ·
plus tokenization `U`, receipt streaming, auto/proactive reconnect.

Data model (serde): `Ecr17Config`, `PaymentRequest`, `ReversalRequest`,
`PreAuthRequest`, `IncrementalAuthRequest`, `PreAuthClosureRequest`,
`CardVerificationRequest`, `TokenizationRequest`; results `PosStatusResponse`,
`PaymentResult`, `ReversalResult`, `PreAuthResult`, `CardVerificationResult`,
`TotalsResult`, `CloseSessionResult`, `VasResult`; events `ProgressEvent`,
`ReceiptLine`; enums `LrcMode`, `ConnectionState`, `TransactionOutcome`, `CardType`,
`TransactionEntryMode`, `PaymentCardType`, `TokenizationService`, `PosTerminalStatus`,
plus `CurrencyExchange` (DCC). Mirror `types/client.ts` field-for-field.

## 5. ECR17 protocol facts (from the reference LESSON.md — must hold)

- App frame = `STX(0x02)` payload `ETX(0x03)` `LRC`. LRC base `0x7F`, XOR-folded; which
  framing bytes are folded is selected by `LrcMode` (`stx`/`std`/`noext`/`stx_noext`).
- Progress update = `SOH(0x01)` + 20-char message + `EOT(0x04)`, **no LRC**. `decode()`
  must reject an SOH frame whose last byte != EOT.
- Status command code is lowercase `'s'`. Payment `'P'` request = **167 bytes**.
- Receipts arrive as one or more `S` messages (concatenate). Reversal request = `'S'`.
- `decode()` treats the buffer as exactly one frame (LRC = final byte); stream→frame
  splitting belongs to the transport layer.
- Outcome map: `"00"→ok`, `"01"→ko`, `"05"→cardNotPresent`, `"09"→unknownTag`.
- Never reuse a generated struct name for a clashing concept (RN hit `CurrencyExchange`
  vs parser DCC → renamed `DccInfo`); keep DCC struct clearly named.

## 6. Guardrails (Definition of Done per task)

Every task/subtask states: **objective · implementation detail · guardrails.** Guardrails:
- **Rust code** → `cargo test` unit tests, TDD RED→GREEN (primary gate) + `cargo clippy -D warnings` + `cargo fmt --check`.
- **Frontend logic** → **Vitest** unit tests (command metadata, form/state, PAN masking).
- **UI/UX** → **Playwright** e2e covering *every* interaction (see Macro 7). Pure-code
  changes with no UI surface need no Playwright.

## 7. Mandatory per-task workflow (Definition of Done loop)

A task/subtask is done ONLY after BOTH loops pass. In automode, advance only when complete.

**Local loop (before pushing):**
1. Local tests green — `cargo test` (+ `clippy`/`fmt`); frontend `vitest`/`tsc`; `playwright test` where UI changed.
2. Local Copilot review — `copilot --autopilot --yolo -p "/review <diff of branch vs origin/main>"`.
   Pass the **full branch diff** (save to a temp file if large and pass the file). Use a
   focused prompt if a whole-diff review times out. Copilot **edits in --yolo** and can be
   wrong → VERIFY every change; record takeaways in `docs/LESSON.md`.
3. Zero actionable comments → continue; else fix and go to 1.

**Remote loop (before a task/PR is done):**
4. Push; wait for CI green (`rust-tests` + `frontend-checks` + `e2e` as applicable); else fix → local loop.
5. Open PR toward the **macro-task branch** (subtask) / toward **main** (macro-task complete).
   Add **Copilot** as reviewer; ensure its review started; WAIT for CI + Copilot comments.
6. Fix every valid comment (validate against code/spec; reject only with a clear reason),
   push, re-request review. Repeat 4–6 until ZERO actionable comments.
7. Only then merge. Update `PROGRESS.md` and `docs/LESSON.md`.

**Branch/PR model:** one **branch per macro-task**; one **PR per subtask** targeting that
branch; when the macro-task is complete, one **PR macro-branch → main** through the full loop.

## 8. Task breakdown

### MACRO 0 — Governance & scaffolding  (branch `chore/bootstrap`)
- **T0.1 Process assets [FIRST, priority].** Port & adapt to Rust/Tauri: `.claude/rules/*`
  (readme-sync, progress/lesson-sync, money-safety), `.claude/skills/*` (workflow-loop,
  docs), `.claude/settings.json`, `AGENTS.md`, `CLAUDE.md`, `docs/LESSON.md` (seeded),
  `PROGRESS.md` (seeded), this `docs/PLAN.md`. Objective: procedure survives a session
  interruption. Guardrail: files present, internally consistent, committed.
- **T0.2 Cargo workspace + crate skeleton.** `Cargo.toml` workspace; `crates/ecr17-protocol`
  compiles (`cargo build`, `cargo test` with a trivial test); `rustfmt.toml`, `clippy` clean.
- **T0.3 Tauri app scaffold.** `app/` React 19 + TS + Vite (latest) + Tauri 2 (latest); empty
  window runs; Vitest + Playwright wired with one trivial green test each.
- **T0.4 CI skeleton.** `.github/workflows/`: `rust-tests` (build+test+clippy+fmt),
  `frontend-checks` (tsc+vitest+lint), `e2e` (playwright). All green on the scaffold.
- **Macro-0 DoD:** trivial `cargo test` + `vitest` + `playwright` green in CI; PR → main merged.

### MACRO 1 — Protocol primitives  (branch `feat/protocol-primitives`)
- **T1.1 `lrc.rs`** — `LrcMode` enum + LRC compute (base `0x7F`, fold per mode). Tests port `test_lrc.cpp`.
- **T1.2 `codec.rs`** — `PacketCodec::encode/decode` (STX/ETX/LRC; SOH progress no-LRC + EOT check;
  single-frame decode). Tests port `test_packet_codec.cpp`.
- **Guardrail:** RED→GREEN unit tests per function; byte-exact framing.

### MACRO 2 — Message builders  (branch `feat/protocol-builders`)
- **T2.1 `types.rs`** — all requests/results/enums with serde; field-exact mirror of `types/client.ts`.
- **T2.2 `protocol.rs`** — every builder: payment family (`P`/`X`/`p` 167B), pre-auth
  integration/closure (`i`/`c` 176B), card verify (`H` 39B), session (`C`/`T` 26B),
  `G`(22B), `E`(11B), `R`(22B), status `s`(10B), reversal `S`(26B), VAS `K`, additional
  tags `U` + `format_tokenization_tag`. Fixed-width validation → error on overflow.
- **Guardrail:** byte-exact builder tests (port `test_protocol.cpp` + `test_protocol_commands.cpp`).

### MACRO 3 — Response parsers  (branch `feat/protocol-parsers`)
- **T3.1 `response.rs`** — parsers for status/payment/reversal/preauth/cardverify/totals/
  closesession/vas; outcome mapping; entry mode; card type; DCC (`DccInfo`); PAN masking.
- **Guardrail:** parser tests from captured frames (port `test_response.cpp`).

### MACRO 4 — Session & money-safety  (branch `feat/session-retry`)
- **T4.1 `transport.rs`** — async `Transport` trait + `FakeTransport` (scripted replies,
  `disconnect_on_next_request`/`rearm`, ACK/NAK injection).
- **T4.2 `retry.rs` (CRITICAL)** — `RetryPolicy`: financial commands never replayed; safe/
  idempotent (status/totals) may retry. Tests port `test_retry_policy.cpp`.
- **T4.3 `session.rs`** — `Ecr17Session`: ACK/NAK + retransmit + timeouts + `reset_for_new_transaction`
  + receipt drain + proactive liveness. Tests port `test_session.cpp` + `test_flows.cpp`,
  incl. `recovers_and_succeeds_after_reconnect`.
- **Guardrail:** money-safety tests locked; reconnect-recovery test green.

### MACRO 5 — Client API + TCP transport  (branch `feat/client-and-tcp`)
- **T5.1 `client.rs`** — `Ecr17Client` async API (all commands, event channels, auto +
  proactive reconnect, tokenization `U` wiring, `sendLastResult`/`G` recovery, tx mutex).
  Tests: full flows via `FakeTransport` (port happy paths of `test_integration_terminal.cpp`).
- **T5.2 `transport/tcp.rs`** — tokio TCP transport behind `tokio-transport`; non-destructive
  liveness probe (peek) analog of the RN Kotlin probe; env-gated real-terminal integration test.
- **T5.3 crate polish** — `crates/ecr17-protocol/README.md`, doc comments, `cargo publish --dry-run` green.
- **Guardrail:** client flow tests via FakeTransport; `--dry-run` publish clean.

### MACRO 6 — Tauri backend bridge  (branch `feat/tauri-backend`)
- **T6.1 `src-tauri`** — managed `Ecr17Client` state; `#[tauri::command]` per command
  (configure/connect/disconnect/isConnected + all protocol cmds); emit events; PAN masking in logs.
- **T6.2 backend tests** — serde round-trips of IPC types; dispatch table matches core API surface.
- **Guardrail:** `cargo test` on backend; command list == core API list.

### MACRO 7 — Control panel UI  (branch `feat/control-panel-ui`)
- **T7.1 logic port** — `commands.ts` metadata, theme, logger, `results` (PAN mask), `storage`
  (persist config), `useEcr17` hook → Tauri IPC. Vitest unit tests.
- **T7.2 components** — ConnectionBar, ConfigForm, CommandPalette, CommandParamsSheet + fields,
  LogConsole, BusyOverlay. Vitest for pure logic.
- **T7.3 Playwright e2e (all interactions)** — connect success/failure, empty-host guard, each
  command opens+submits its param sheet, danger-command styling/confirm, money/text/bool/enum
  fields, log console filtering + PAN masking, busy overlay during a run, config persistence
  across reload, connection-state bar transitions. Run against mocked Tauri IPC.
- **Guardrail:** Vitest + full Playwright suite green in CI.

### MACRO 8 — Packaging, docs, release  (branch `chore/release-1.0`)
- **T8.1 Wow README** — root `README.md` + `crates/ecr17-protocol/README.md` mirror; banner
  (reuse/adapt `resources/banner.png`), badges, screenshots (captured from the running Tauri
  app), protocol cheat-sheet, architecture, API reference, testing, vibe-coding section.
- **T8.2 Release CI** — `tauri-build` (installer matrix: Windows `.msi`/NSIS, macOS `.dmg`,
  Linux `.deb`/AppImage) + `release.yml` (on tag `v*` → `cargo publish` the crate **and**
  attach Tauri installers to the GitHub Release).
- **T8.3 Cross-port README links.** In THIS README, add an "other ports" section linking the
  **React Native** and **Laravel** siblings. Then, in each sibling repo — **`git fetch` +
  `git pull` FIRST (local copies may be stale)** — add the **Tauri/Rust** package to their
  "other ports / sibling port" section (RN README ~line 20; Laravel README ~line 20), commit,
  and **push to `main`**. Repos: `padosoft/react-native-ecr17-protocol`
  (`../ReactNative/react-native-ecr17-protocol`), `padosoft/laravel-ecr17` (`C:/xampp/htdocs/laravel-ecr17`).
- **T8.4 Knowledge consolidation (final).** Review `docs/LESSON.md` + everything learned; create/
  strengthen `.claude/` rules, skills, and `AGENTS.md` with the new Rust/Tauri know-how.
- **T8.5 Publish & release.** `cargo publish` `ecr17-protocol`; tag `v1.0.0`; GitHub Release with
  installers + changelog.
- **Guardrail:** `cargo publish --dry-run`, `cargo tauri build` succeed; release workflow validated;
  both sibling READMEs updated & pushed.

## 9. Publishing to crates.io (answer to the standing question)

Rust's package registry is **crates.io**, driven by **Cargo**:
1. `cargo login <token>` — token from <https://crates.io/me> (crates.io account via GitHub login).
2. In `crates/ecr17-protocol/Cargo.toml`: `name = "ecr17-protocol"` (verified free), `version`,
   `license`, `description`, `repository`, `readme`, `keywords`, `categories`, `authors`.
3. `cargo publish --dry-run` to validate the package, then `cargo publish`.
4. Names are global & unique — `ecr17-protocol` is available. The Tauri app is NOT published to
   crates.io; it ships as installers on the GitHub Release.

## 10. Toolchain (verified on this machine)

rustc/cargo 1.96 · node 25 · bun 1.3 · gh 2.88 (authenticated, ssh) · copilot CLI 1.0.69 ·
git 2.55. **`tauri-cli` not yet installed** → `cargo install tauri-cli` (or `npm create tauri-app@latest`).
Use the **latest** stable releases of Tauri 2, React 19, Vite, Playwright, Vitest, tokio, serde,
thiserror at scaffold time (pin exact versions in lockfiles).

## 11. Progress tracking

`PROGRESS.md` is the crash-safe resume log — update it at every subtask boundary.
`docs/LESSON.md` accumulates hard-won lessons — update after every Copilot/CI fix and pass its
content into every sub-agent prompt and every new session.
