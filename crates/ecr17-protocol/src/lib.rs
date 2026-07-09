//! # ecr17-protocol
//!
//! A pure-Rust implementation of the Italian **ECR17** payment protocol used by
//! **Nexi Group** POS terminals over a local LAN connection.
//!
//! The crate is layered so the protocol logic is trivially testable and free of I/O:
//!
//! - [`lrc`] — LRC checksum and [`lrc::LrcMode`] framing selector
//! - [`codec`] — STX/ETX/LRC framing (encode/decode)
//! - [`types`] — request/result/enum data model (serde)
//! - `protocol` — message builders (one per ECR17 command)
//! - `response` — response parsers
//! - `retry` — money-safety retry policy (a financial command is never blindly replayed)
//! - `session` — ACK/NAK, retransmit, timeout orchestration over a [`transport::Transport`]
//! - `client` — the async [`Ecr17Client`] API
//!
//! I/O lives behind the async [`transport::Transport`] trait; the real tokio TCP
//! transport is available under the `tokio-transport` feature.
//!
//! 💰 **Money-critical:** this drives a terminal that charges real cards. Financial
//! commands are never blindly re-sent after a reconnect — recover a lost response via
//! `send_last_result()` (spec command `G`).
//!
//! > Protocol reference (public): <https://developer.nexigroup.com/traditionalpos/en-EU/docs/>

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

// Layers are added incrementally per the implementation plan (docs/PLAN.md).
// MACRO 1+ will populate these modules.

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_builds_and_tests_run() {
        // Placeholder guardrail proving the crate compiles and the test harness runs.
        // Replaced by real layer tests starting at MACRO 1.
        assert_eq!(2 + 2, 4);
    }
}
