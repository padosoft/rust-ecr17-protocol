<div align="center">

# 💳 ecr17-protocol

**The Italian ECR17 payment protocol in pure Rust — drive Nexi Group POS terminals over LAN from any Rust app, plus a Tauri desktop control panel.**

**The most complete open-source ECR17 toolkit for Rust & the desktop.**

[![rust-tests](https://github.com/padosoft/rust-ecr17-protocol/actions/workflows/rust-tests.yml/badge.svg?branch=main)](https://github.com/padosoft/rust-ecr17-protocol/actions/workflows/rust-tests.yml)
[![frontend-checks](https://github.com/padosoft/rust-ecr17-protocol/actions/workflows/frontend-checks.yml/badge.svg?branch=main)](https://github.com/padosoft/rust-ecr17-protocol/actions/workflows/frontend-checks.yml)
[![e2e](https://github.com/padosoft/rust-ecr17-protocol/actions/workflows/e2e.yml/badge.svg?branch=main)](https://github.com/padosoft/rust-ecr17-protocol/actions/workflows/e2e.yml)
[![crates.io](https://img.shields.io/crates/v/ecr17-protocol?style=flat-square&logo=rust)](https://crates.io/crates/ecr17-protocol)
[![docs.rs](https://img.shields.io/docsrs/ecr17-protocol?style=flat-square&logo=docsdotrs)](https://docs.rs/ecr17-protocol)
[![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)](https://github.com/padosoft/rust-ecr17-protocol/blob/main/LICENSE)
[![Built with Rust + Tauri](https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-8B5CF6?style=flat-square&logo=tauri)](https://tauri.app)

<img src="https://raw.githubusercontent.com/padosoft/rust-ecr17-protocol/main/resources/banner.png" alt="ecr17-protocol banner" width="100%" />

</div>

> 🐘 **Using PHP / Laravel?** → **[padosoft/laravel-ecr17](https://github.com/padosoft/laravel-ecr17)**
> &nbsp;·&nbsp; 📱 **Using React Native / mobile?** → **[padosoft/react-native-ecr17-protocol](https://github.com/padosoft/react-native-ecr17-protocol)**
> — the same ECR17 protocol, ported to each ecosystem.

---

## 📚 Table of contents

- [What is ECR17?](#-what-is-ecr17)
- [Why this exists](#-why-this-exists)
- [Highlights](#-highlights)
- [Screenshots](#-screenshots)
- [Feature status](#-feature-status)
- [Installation](#-installation)
- [Quick start](#-quick-start)
- [Money-safety](#-money-safety)
- [Configuration](#%EF%B8%8F-configuration)
- [API reference](#-api-reference)
- [Events](#-events)
- [Tokenization & receipts](#-tokenization--receipts)
- [Protocol cheat-sheet](#-protocol-cheat-sheet)
- [Architecture](#%EF%B8%8F-architecture)
- [The Tauri control panel](#%EF%B8%8F-the-tauri-control-panel)
- [Testing](#-testing)
- [Other ports](#-other-ports)
- [Vibe-coding batteries included](#-vibe-coding-batteries-included)
- [License](#-license)

## 🧭 What is ECR17?

**ECR17** is the Italian standard protocol — supported by **Nexi Group** terminals —
that integrates an *Electronic Cash Register* (ECR) with an *EFT-POS* payment
terminal over a local LAN connection. The cash register sends a request
(payment, reversal, status…), the terminal talks to the acquiring host, and
replies synchronously.

This crate speaks that protocol from Rust: a pure, `#![forbid(unsafe_code)]`
protocol engine plus an async `Ecr17Client` and an optional **tokio TCP**
transport — no C, no FFI, no bridge.

> 📚 **Official protocol reference (public):**
> <https://developer.nexigroup.com/traditionalpos/en-EU/docs/> — the
> authoritative source. Field positions, message codes and `lrcMode` may vary by
> terminal/firmware; always check against the official docs.

## 🎯 Why this exists

Integrating Italian POS terminals has long been needlessly painful. The ECR17
protocol is **not publicly documented** — the specifications are shared under NDA,
mostly with established point-of-sale software vendors — so everyone else
reverse-engineers it by trial and error across terminals and firmware versions.
(The classic trap that blocks almost everyone: the LRC is computed over a base of
`0x7F`, not `0x00` — handled here, and configurable per terminal.)

A few community efforts exist for server-side languages, but there was **nothing
idiomatic for Rust or the desktop**. To our knowledge this is the **most complete
open-source ECR17 toolkit for Rust**: the full command set, response parsing, the
ACK/NAK + retransmit orchestration, configurable LRC modes, and payment-safety —
all tested, all `async`, all pure Rust.

The goal is simple: **Rust and desktop developers should no longer struggle to
talk to Italian POS terminals.** No NDA hunting, no guesswork — just
`client.pay(&req).await`.

> 🤝 Compatibility notes (lrcMode, field quirks per terminal/firmware) are
> welcome as issues, so we can build, together, the reference the ecosystem
> never had.

## ✨ Highlights

- 🦀 **Pure-Rust protocol core** — framing / LRC / orchestration, `#![forbid(unsafe_code)]`, no FFI.
- 🔄 **Async, `Future`-based API** — `client.pay(&req).await`, built on tokio.
- 🧱 **Full command set** — payment, extended payment, reversal, pre-auth (request / incremental / closure), card verification, close session, totals, last result, ECR printing, reprint, VAS.
- 🛡️ **Robust by design** — fixed-width field validation, defensive response parsing, ACK/NAK handshake with **retransmit** and timeouts.
- 💰 **Money-safe** — a financial command is **never blindly re-sent** after a reconnect (double-charge protection), locked by unit tests. Recover a lost response via `send_last_result()` (spec `G`).
- 📡 **Live events** — progress messages, streamed receipt lines, connection state — via callbacks.
- 🔌 **Transport-agnostic** — I/O lives behind an async `Transport` trait; the real **tokio TCP** socket is one feature flag away, and the in-memory `FakeTransport` makes the whole stack testable with no hardware.
- 🖥️ **Tauri control panel** — a cross-platform desktop debug console that exercises every command and streams the behind-the-scenes log live.
- ✅ **Heavily tested** — **108 Rust** unit / flow / money-safety tests (LRC, codec, every builder, every parser, full session orchestration) plus **12 Vitest** + **12 Playwright** e2e for the UI, all green in CI.
- 🤖 **Vibe-coding batteries included** — ships first-class AI-agent context (`AGENTS.md`, `CLAUDE.md`, `docs/LESSON.md`, `PROGRESS.md`). See [below](#-vibe-coding-batteries-included).

## 📸 Screenshots

The repo ships a Tauri **Control Panel** app that exercises every ECR17 command
against a real terminal and streams the behind-the-scenes log (sent / progress /
receipt / result / error) live — with card PANs masked.

<table>
  <tr>
    <td align="center" width="62%">
      <img src="https://raw.githubusercontent.com/padosoft/rust-ecr17-protocol/main/resources/screenshots/control-panel.png" alt="ECR17 Control Panel — command grid & live log" width="100%" /><br/>
      <sub>Command grid &amp; live log (PAN masked)</sub>
    </td>
    <td align="center" width="38%">
      <img src="https://raw.githubusercontent.com/padosoft/rust-ecr17-protocol/main/resources/screenshots/params-sheet.png" alt="ECR17 Control Panel — dynamic parameters sheet" width="100%" /><br/>
      <sub>Dynamic parameters sheet (money → cents)</sub>
    </td>
  </tr>
</table>

## 📊 Feature status

| Area | Status |
|------|:------:|
| Packet framing + LRC (4 modes) | ✅ |
| All request builders (`P X p i c H U C T G E R K s S`) | ✅ |
| Response parsing (`E/V/s/T/C/e/K`, incl. DCC) | ✅ |
| Session orchestration (ACK/NAK, retransmit, timeout, progress/receipt) | ✅ |
| Async client API + events | ✅ |
| Auto-reconnect, tokenization (`U`) flow, receipt streaming | ✅ |
| tokio TCP transport (non-destructive liveness probe) | ✅ *(feature `tokio-transport`)* |
| Tauri desktop control panel (Win / macOS / Linux) | ✅ *(CI-built installers)* |

## 📦 Installation

```bash
# Protocol core only (bring your own transport):
cargo add ecr17-protocol

# With the real tokio TCP transport:
cargo add ecr17-protocol --features tokio-transport
cargo add tokio --features full
```

**MSRV 1.85** (the TCP liveness probe uses `std::task::Waker::noop`).

## 🚀 Quick start

```rust
use std::time::Duration;
use ecr17_protocol::{Ecr17Client, Ecr17Config, LrcMode, PaymentRequest, ReversalRequest};
use ecr17_protocol::transport::tcp::TcpTransport; // needs feature "tokio-transport"

#[tokio::main]
async fn main() -> ecr17_protocol::Result<()> {
    let config = Ecr17Config {
        host: "192.168.1.50".into(),   // terminal IP on the LAN
        port: Some(10000),              // configured ECR port
        terminal_id: "12345678".into(),
        cash_register_id: "00000001".into(),
        lrc_mode: Some(LrcMode::Std),
        keep_alive: Some(true),
        auto_reconnect: Some(true),
        connection_timeout_ms: Some(5000),
        response_timeout_ms: Some(60000),
        ack_timeout_ms: Some(2000),
        receipt_drain_ms: None,
        retry_count: Some(3),
        retry_delay_ms: Some(200),
        debug: Some(false),
    };

    let transport = TcpTransport::new(config.host.clone(), 10000, Duration::from_secs(5));
    let mut client = Ecr17Client::new(transport, config);

    client.connect().await?;

    let result = client.pay(&PaymentRequest {
        amount_cents: 650,               // €6.50
        cash_register_id: None,
        payment_type: None,
        card_already_present: None,
        receipt_text: None,
        tokenization: None,
    }).await?;

    if result.outcome == ecr17_protocol::TransactionOutcome::Ok {
        println!(
            "Approved: auth {:?} PAN {:?}",
            result.auth_code, result.pan, // both Option<String> — masked PAN
        );
    } else {
        eprintln!("Declined: {:?}", result.error_description);
    }

    // Reversal ("annullamento") of the last transaction (no STAN → reverses the last):
    client.reverse(&ReversalRequest { cash_register_id: None, stan: None }).await.ok();

    let _status = client.status().await?;   // PosStatusResponse
    client.disconnect().await;
    Ok(())
}
```

> Request structs are plain data — construct the fields you need and leave the
> `Option` ones `None`. The optional `..` fields map straight onto the wire
> defaults.

## 💰 Money-safety

This crate drives a terminal that **charges real cards**, so payment integrity is
a first-class, tested invariant — not an afterthought:

- **A financial command is never blindly re-sent.** `pay`, `pay_extended`,
  `reverse`, `pre_auth`, `incremental_auth`, `pre_auth_closure` are **not** replayed
  after a transport drop/reconnect (a blind retry can double-charge). The decision
  lives in one tiny, unit-locked place — `should_retry_after_reconnect` — which
  only ever allows retrying safe/idempotent ops (`status`, `totals`).
- **Recover a lost response, don't replay it.** If the reply is lost mid-flight,
  call `send_last_result()` (the spec's `G` command) to fetch the real outcome.
- **Proactive drop detection.** Nexi terminals close the TCP socket *between*
  transactions, so the transport does a **non-destructive liveness probe**
  (`poll_peek`) **before** sending — never writing bytes on the peer's protocol
  stream — so a financial command never even starts on a stale socket.
- **Reusable across reconnects.** The session holds no sticky "disconnected" flag;
  it resets its per-transaction state, so a fresh transaction is never blocked by a
  previous drop.

These rules are guarded by regression tests (`financial_command_not_replayed_on_drop`,
`safe_command_retried_after_reconnect`, `recovers_and_succeeds_after_reconnect`, …)
that must stay green.

## ⚙️ Configuration

`Ecr17Config`: `host` (required), `port?`, `terminal_id` (required),
`cash_register_id` (required), `lrc_mode?`, `keep_alive?`, `auto_reconnect?`,
`connection_timeout_ms?`, `response_timeout_ms?`, `ack_timeout_ms?`,
`receipt_drain_ms?`, `retry_count?`, `retry_delay_ms?`, `debug?`.

All fields serde-`camelCase` on the wire (`cash_register_id` ⇄ `cashRegisterId`),
so the same config round-trips cleanly to the Tauri frontend / any JSON consumer.

## 📖 API reference

Every command is **async** and performs a full request/response exchange.
`new` / `configuration` are synchronous.

| Method | Command | Returns |
|--------|:------:|---------|
| `connect()` / `disconnect()` / `is_connected()` | — | `Result<()>` / `()` / `bool` |
| `status()` | `s` | `PosStatusResponse` |
| `pay(&req)` / `pay_extended(&req)` | `P` / `X` | `PaymentResult` |
| `reverse(&req)` | `S` | `ReversalResult` |
| `pre_auth(&req)` / `incremental_auth(&req)` / `pre_auth_closure(&req)` | `p` / `i` / `c` | `PreAuthResult` / `PaymentResult` |
| `verify_card(&req)` | `H` | `CardVerificationResult` |
| `close_session()` / `totals()` | `C` / `T` | `CloseSessionResult` / `TotalsResult` |
| `send_last_result()` | `G` | `PaymentResult` |
| `enable_ecr_printing(bool)` / `reprint(bool)` | `E` / `R` | `Result<()>` |
| `vas(&xml)` | `K` | `VasResult` |

Commands require an open connection (`connect()` first) and error on
timeout / retransmission exhaustion / disconnect.

## 📡 Events

```rust
client.set_on_progress(|e| println!("progress: {}", e.message));
client.set_on_receipt_line(|l| append_to_receipt(l.text));
client.set_on_connection_state_change(|s| println!("connection: {s:?}"));
```

> Callbacks are `Fn(..) + Send + Sync + 'static`. They are invoked with no client
> lock held, so a callback may safely call back into the client.

## 🧾 Tokenization & receipts

```rust
// Tokenization: attach a contract to a payment / preAuth / verifyCard. The 'U'
// additional-data message is sent automatically (P -> ACK -> U -> ACK -> result).
use ecr17_protocol::{TokenizationRequest, TokenizationService};

let req = PaymentRequest {
    amount_cents: 1000,
    tokenization: Some(TokenizationRequest {
        service: TokenizationService::Recurring,
        contract_code: "1666354841608".into(),
    }),
    cash_register_id: None, payment_type: None,
    card_already_present: None, receipt_text: None,
};

// Receipts printed by the ECR: enable printing, set receipt_drain_ms in the config,
// and receive lines via the event.
client.enable_ecr_printing(true).await?;
client.set_on_receipt_line(|l| append_to_receipt(l.text));
```

## 🔐 Protocol cheat-sheet

App frame: `STX(0x02)` · payload · `ETX(0x03)` · `LRC`. Progress: `SOH(0x01)` ·
20 chars · `EOT(0x04)` (no LRC). Confirmation: `ACK(0x06)` / `NAK(0x15)` · `ETX` ·
`LRC`. LRC = `0x7F` XOR-folded; the framing bytes folded in are selectable via
`lrc_mode` (`stx` / `std` / `noext` / `stx_noext`). Status code is lowercase `s`;
a `P` payment request is **167 bytes**; receipts arrive as one or more `S`
messages. Outcome map: `00→ok`, `01→ko`, `05→cardNotPresent`, `09→unknownTag`.

## 🏗️ Architecture

```
crates/ecr17-protocol/src/
├── lrc.rs        # LRC (4 modes, base 0x7F) + LrcMode
├── codec.rs      # framing: STX·ETX·SOH·EOT·ACK·NAK + LRC (encode/decode)
├── types.rs      # request/result/enum data model (serde, camelCase)
├── protocol.rs   # request builders (all commands), fixed-width + validated
├── response.rs   # response field parsers -> plain structs (incl. DCC, PAN mask)
├── retry.rs      # 💰 RetryPolicy — a financial command is never replayed
├── transport.rs  # async Transport trait + in-memory FakeTransport (tests)
├── session.rs    # ACK/NAK + retransmit + timeout + receipt-drain orchestration
├── client.rs     # Ecr17Client async API + events + auto/proactive reconnect
└── transport/tcp.rs   # tokio TCP transport (feature "tokio-transport")
app/
├── src-tauri/    # Rust backend: managed Ecr17Client, one #[tauri::command] per cmd, events
└── src/          # React 19 + TypeScript + Vite control panel (typed IPC, useEcr17 hook)
```

**Design.** The codec / protocol / response layers are **pure and sync** — trivially
unit-testable with no runtime. I/O lives behind the async `Transport` trait, so the
session/client run against a scripted `FakeTransport` in tests (including simulated
mid-exchange drops) and a real tokio socket in production.

## 🖥️ The Tauri control panel

`app/` is a cross-platform **Tauri 2** desktop app (React 19 + TypeScript + Vite)
that holds an `Ecr17Client` in managed state and exposes one `#[tauri::command]`
per protocol command. It’s the fastest way to poke a terminal: pick a command,
fill the dynamically-generated parameters sheet (money fields coerce € → cents),
hit run, and watch the sent/progress/receipt/result log stream live — PANs masked.
Native installers (Windows `.msi`/NSIS, macOS `.dmg`, Linux `.deb`/AppImage) are
built in CI and attached to each GitHub Release.

## 🧪 Testing

```bash
cargo test --all-features          # 108 unit / flow / money-safety tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Frontend (from app/):
bun run typecheck && bun run test  # 12 Vitest
bun run e2e                        # 12 Playwright (Tauri IPC mocked)
```

The Rust tests cover LRC, packet (de)framing edge cases, every builder's byte
layout, every response parser, and the documented payment / reversal / re-pay /
progress / receipt / NAK-retransmit / timeout / **reconnect-recovery** flows
(against an in-memory `FakeTransport`).

### Against a real terminal (opt-in)

An `#[ignore]`d integration test runs the core over a real TCP socket. It is
**skipped** unless `ECR17_TEST_HOST` is set:

```bash
ECR17_TEST_HOST=192.168.1.50 ECR17_TEST_PORT=10000 \
  cargo test --features tokio-transport -- --ignored real_terminal
```

## 🧩 Other ports

The same ECR17 protocol, maintained across ecosystems:

| Port | Repo | Stack |
|------|------|-------|
| **Rust / Tauri** *(this)* | [padosoft/rust-ecr17-protocol](https://github.com/padosoft/rust-ecr17-protocol) | Rust core + tokio + Tauri desktop |
| **React Native** | [padosoft/react-native-ecr17-protocol](https://github.com/padosoft/react-native-ecr17-protocol) | C++ core + Nitro (iOS/Android) |
| **Laravel** | [padosoft/laravel-ecr17](https://github.com/padosoft/laravel-ecr17) | PHP / Laravel package + console |

## 🤖 Vibe-coding batteries included

Building on an undocumented payment protocol is exactly where AI assistants get
things subtly wrong. This repo ships the context to prevent that, so an agent (or
a new contributor) is productive and *safe* from minute one:

- **[`AGENTS.md`](https://github.com/padosoft/rust-ecr17-protocol/blob/main/AGENTS.md)** /
  **[`CLAUDE.md`](https://github.com/padosoft/rust-ecr17-protocol/blob/main/CLAUDE.md)** — project guide, the mandatory
  per-task workflow, CI strategy, and the **money-critical** rules (e.g. never
  blindly retry a payment).
- **[`docs/LESSON.md`](https://github.com/padosoft/rust-ecr17-protocol/blob/main/docs/LESSON.md)** — accumulated, verified engineering lessons
  (Rust/Tauri APIs, toolchain traps, protocol facts, money-safety) — the gotchas
  already solved.
- **[`PROGRESS.md`](https://github.com/padosoft/rust-ecr17-protocol/blob/main/PROGRESS.md)** — crash-safe resume state across sessions.

The result: less hallucination, fewer footguns, and changes that respect the
payment-safety invariants by default.

## 📄 License

[MIT](https://github.com/padosoft/rust-ecr17-protocol/blob/main/LICENSE) © [padosoft](https://github.com/padosoft)

> **Disclaimer:** independent integration library. "ECR17", "Nexi" and related
> marks belong to their respective owners and are referenced for interoperability only.
