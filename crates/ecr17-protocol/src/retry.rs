//! Money-safety retry policy.
//!
//! 💰 **MONEY-CRITICAL INVARIANT:** a financial command (`safe_to_retry == false`) must
//! **never** be re-sent after a connection drop. If the socket drops after the terminal
//! has processed the payment but before the response arrives, a blind re-send would charge
//! the cardholder twice. Such cases are recovered by querying the terminal's last result
//! (command `G` / `send_last_result`), NOT by retransmitting the request.
//!
//! Only read-only / idempotent commands (status, totals, `send_last_result`,
//! enable-printing) pass `safe_to_retry == true`.
//!
//! Reconnecting the socket is a separate, always-safe action; this function only governs
//! whether the *request* is replayed. Port of the reference C++ `RetryPolicy`.

/// Decides whether a command may be safely **re-sent** after an auto-reconnect.
///
/// Returns `true` only when auto-reconnect is enabled, the transport actually dropped, and
/// the command is safe/idempotent. A financial command (`safe_to_retry == false`) always
/// returns `false`.
#[must_use]
pub fn should_retry_after_reconnect(
    auto_reconnect: bool,
    transport_dropped: bool,
    safe_to_retry: bool,
) -> bool {
    auto_reconnect && transport_dropped && safe_to_retry
}

#[cfg(test)]
mod tests {
    use super::should_retry_after_reconnect as should_retry;

    // 💰 A financial command (safe = false) must never be retried, regardless of
    // auto-reconnect / drop state. Recovery is via send_last_result ('G'), not a re-send.
    #[test]
    fn financial_command_is_never_retried() {
        assert!(!should_retry(true, true, false));
        assert!(!should_retry(false, true, false));
        assert!(!should_retry(true, false, false));
        assert!(!should_retry(false, false, false));
    }

    // A safe/idempotent command is retried ONLY when auto-reconnect is on AND the transport
    // actually dropped.
    #[test]
    fn safe_command_retried_only_on_reconnect_after_drop() {
        assert!(should_retry(true, true, true));
        assert!(!should_retry(false, true, true));
        assert!(!should_retry(true, false, true));
        assert!(!should_retry(false, false, true));
    }
}
