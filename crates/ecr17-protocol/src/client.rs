//! [`Ecr17Client`] — the high-level async API: builds requests, drives the
//! [`Ecr17Session`], parses/maps responses to the typed [`crate::types`] results, and
//! applies the money-safe auto-reconnect policy. Port of the reference C++ `HybridEcr17Client`.
//!
//! 💰 On a mid-command drop the socket is reconnected (when `auto_reconnect` is on), but a
//! **financial** command is never replayed (double-charge) — only read-only/idempotent ops
//! are retried; a lost financial response is recovered via [`Ecr17Client::send_last_result`]
//! (`G`). The decision is [`crate::retry::should_retry_after_reconnect`].

use std::sync::{Arc, Mutex};

use crate::codec::DecodedPacket;
use crate::error::Result;
use crate::response;
use crate::retry::should_retry_after_reconnect;
use crate::session::{Ecr17Session, SessionConfig};
use crate::transport::Transport;
use crate::types::{
    CardType, CardVerificationRequest, CardVerificationResult, CloseSessionResult, ConnectionState,
    CurrencyExchange, Ecr17Config, IncrementalAuthRequest, PaymentRequest, PaymentResult,
    PosStatusResponse, PreAuthClosureRequest, PreAuthRequest, PreAuthResult, ProgressEvent,
    ReceiptLine, ReversalRequest, ReversalResult, TokenizationRequest, TotalsResult,
    TransactionEntryMode, VasResult,
};

// The callback is stored behind an inner `Arc` so a caller can be cloned OUT of the mutex
// and invoked with the lock released — never hold the lock across a user callback (that
// would deadlock if the callback re-enters, e.g. re-registers itself).
type SharedCb<T> = Arc<Mutex<Option<Arc<dyn Fn(T) + Send + Sync + 'static>>>>;

/// High-level ECR17 client over a [`Transport`] `T`.
pub struct Ecr17Client<T: Transport> {
    config: Ecr17Config,
    session: Ecr17Session<T>,
    on_progress: SharedCb<ProgressEvent>,
    on_receipt_line: SharedCb<ReceiptLine>,
    on_connection_state_change: SharedCb<ConnectionState>,
}

impl<T: Transport> std::fmt::Debug for Ecr17Client<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ecr17Client")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl<T: Transport> Ecr17Client<T> {
    /// Creates a client over `transport` with `config`. The transport must already target
    /// the configured host/port (it is (re)connected on demand).
    pub fn new(transport: T, config: Ecr17Config) -> Self {
        let mut session = Ecr17Session::new(transport, session_config(&config));

        let on_progress: SharedCb<ProgressEvent> = Arc::new(Mutex::new(None));
        let on_receipt_line: SharedCb<ReceiptLine> = Arc::new(Mutex::new(None));

        let p = Arc::clone(&on_progress);
        session.set_on_progress(move |message| {
            let cb = p.lock().unwrap().clone(); // clone the Arc, then release the lock
            if let Some(cb) = cb {
                cb(ProgressEvent { message });
            }
        });
        let r = Arc::clone(&on_receipt_line);
        session.set_on_receipt_line(move |text| {
            let cb = r.lock().unwrap().clone();
            if let Some(cb) = cb {
                cb(ReceiptLine { text });
            }
        });

        Self {
            config,
            session,
            on_progress,
            on_receipt_line,
            on_connection_state_change: Arc::new(Mutex::new(None)),
        }
    }

    /// The active configuration.
    #[must_use]
    pub fn configuration(&self) -> &Ecr17Config {
        &self.config
    }

    /// Registers the progress-update listener (`SOH` frames during a procedure).
    pub fn set_on_progress(&self, cb: impl Fn(ProgressEvent) + Send + Sync + 'static) {
        *self.on_progress.lock().unwrap() = Some(Arc::new(cb));
    }

    /// Registers the receipt-line listener (`S` messages when ECR printing is on).
    pub fn set_on_receipt_line(&self, cb: impl Fn(ReceiptLine) + Send + Sync + 'static) {
        *self.on_receipt_line.lock().unwrap() = Some(Arc::new(cb));
    }

    /// Registers the connection-state listener.
    pub fn set_on_connection_state_change(
        &self,
        cb: impl Fn(ConnectionState) + Send + Sync + 'static,
    ) {
        *self.on_connection_state_change.lock().unwrap() = Some(Arc::new(cb));
    }

    /// Borrows the underlying session's transport (read-only).
    pub fn transport(&self) -> &T {
        self.session.transport()
    }

    /// Opens the connection (emits `Connecting` → `Connected`/`Disconnected`).
    pub async fn connect(&mut self) -> Result<()> {
        self.ensure_connected().await
    }

    /// Closes the connection (emits `Disconnected`).
    pub async fn disconnect(&mut self) {
        self.session.disconnect().await;
        self.emit_state(ConnectionState::Disconnected);
    }

    /// Whether the transport currently believes it is connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.session.is_connected()
    }

    // --- Commands ---

    /// Status (`s`).
    pub async fn status(&mut self) -> Result<PosStatusResponse> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_status(&self.config.terminal_id)?;
        let pkt = self.run_transaction(&payload, None, true).await?;
        Ok(map_status(&response::parse_status(&as_str(&pkt))))
    }

    /// Payment (`P`).
    pub async fn pay(&mut self, request: &PaymentRequest) -> Result<PaymentResult> {
        self.ensure_connected().await?;
        let payload = self.payment_payload('P', request)?;
        let pkt = self
            .run_transaction(&payload, request.tokenization.as_ref(), false)
            .await?;
        Ok(map_payment(&response::parse_payment(&as_str(&pkt))))
    }

    /// Extended payment (`X`).
    pub async fn pay_extended(&mut self, request: &PaymentRequest) -> Result<PaymentResult> {
        self.ensure_connected().await?;
        let payload = self.payment_payload('X', request)?;
        let pkt = self
            .run_transaction(&payload, request.tokenization.as_ref(), false)
            .await?;
        Ok(map_payment(&response::parse_payment(&as_str(&pkt))))
    }

    /// Reversal (`S`).
    pub async fn reverse(&mut self, request: &ReversalRequest) -> Result<ReversalResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_reversal(
            &self.config.terminal_id,
            self.cash_register_id_or(request.cash_register_id.as_deref()),
            request.stan.as_deref().unwrap_or("000000"),
        )?;
        let pkt = self.run_transaction(&payload, None, false).await?;
        Ok(map_reversal(&response::parse_payment(&as_str(&pkt))))
    }

    /// Pre-auth (`p`).
    pub async fn pre_auth(&mut self, request: &PreAuthRequest) -> Result<PreAuthResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_pre_auth(
            &self.config.terminal_id,
            self.cash_register_id_or(request.cash_register_id.as_deref()),
            request.amount_cents,
            request.payment_type.unwrap_or_default().as_digit(),
            request.card_already_present.unwrap_or(false),
            request.tokenization.is_some(),
            request.receipt_text.as_deref().unwrap_or(""),
        )?;
        let pkt = self
            .run_transaction(&payload, request.tokenization.as_ref(), false)
            .await?;
        Ok(map_pre_auth(&response::parse_pre_auth(&as_str(&pkt))))
    }

    /// Incremental pre-auth (`i`).
    pub async fn incremental_auth(
        &mut self,
        request: &IncrementalAuthRequest,
    ) -> Result<PreAuthResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_incremental(
            &self.config.terminal_id,
            self.cash_register_id_or(request.cash_register_id.as_deref()),
            request.amount_cents,
            &request.original_pre_auth_code,
            false,
            request.receipt_text.as_deref().unwrap_or(""),
        )?;
        let pkt = self.run_transaction(&payload, None, false).await?;
        Ok(map_pre_auth(&response::parse_pre_auth(&as_str(&pkt))))
    }

    /// Pre-auth closure (`c`).
    pub async fn pre_auth_closure(
        &mut self,
        request: &PreAuthClosureRequest,
    ) -> Result<PaymentResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_pre_auth_closure(
            &self.config.terminal_id,
            self.cash_register_id_or(request.cash_register_id.as_deref()),
            request.amount_cents,
            &request.original_pre_auth_code,
            false,
            request.receipt_text.as_deref().unwrap_or(""),
        )?;
        let pkt = self.run_transaction(&payload, None, false).await?;
        Ok(map_payment(&response::parse_payment(&as_str(&pkt))))
    }

    /// Card verification (`H`).
    pub async fn verify_card(
        &mut self,
        request: &CardVerificationRequest,
    ) -> Result<CardVerificationResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_card_verification(
            &self.config.terminal_id,
            self.cash_register_id_or(request.cash_register_id.as_deref()),
            request.payment_type.unwrap_or_default().as_digit(),
            request.tokenization.is_some(),
        )?;
        let pkt = self
            .run_transaction(&payload, request.tokenization.as_ref(), false)
            .await?;
        Ok(map_card_verify(&response::parse_payment(&as_str(&pkt))))
    }

    /// Close session (`C`).
    pub async fn close_session(&mut self) -> Result<CloseSessionResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_close_session(
            &self.config.terminal_id,
            &self.config.cash_register_id,
            false,
        )?;
        let pkt = self.run_transaction(&payload, None, false).await?;
        Ok(map_close(&response::parse_close(&as_str(&pkt))))
    }

    /// Totals (`T`).
    pub async fn totals(&mut self) -> Result<TotalsResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_totals(
            &self.config.terminal_id,
            &self.config.cash_register_id,
            false,
        )?;
        let pkt = self.run_transaction(&payload, None, true).await?;
        Ok(map_totals(&response::parse_totals(&as_str(&pkt))))
    }

    /// Send last result (`G`) — recovers a lost financial response without re-charging.
    pub async fn send_last_result(&mut self) -> Result<PaymentResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_send_last_result(
            &self.config.terminal_id,
            &self.config.cash_register_id,
            false,
        )?;
        let pkt = self.run_transaction(&payload, None, true).await?;
        Ok(map_payment(&response::parse_payment(&as_str(&pkt))))
    }

    /// Enable/disable ECR printing (`E`).
    pub async fn enable_ecr_printing(&mut self, enabled: bool) -> Result<()> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_enable_ecr_print(&self.config.terminal_id, enabled)?;
        self.run_ack_only(&payload, true).await
    }

    /// Reprint the last ticket (`R`).
    pub async fn reprint(&mut self, to_ecr: bool) -> Result<()> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_reprint(&self.config.terminal_id, to_ecr, '0')?;
        self.run_ack_only(&payload, false).await
    }

    /// VAS (`K`).
    ///
    /// Reads a single VAS response message (as the reference does). A multi-part response
    /// (`more_messages` set) is not concatenated here — that is a documented limitation to
    /// validate against a real terminal before implementing.
    pub async fn vas(&mut self, xml_request: &str) -> Result<VasResult> {
        self.ensure_connected().await?;
        let payload = crate::protocol::build_vas(
            &self.config.terminal_id,
            &self.config.cash_register_id,
            xml_request,
        )?;
        let pkt = self.run_transaction(&payload, None, false).await?;
        Ok(map_vas(&response::parse_vas(&as_str(&pkt))))
    }

    // --- internals ---

    fn cash_register_id_or<'a>(&'a self, override_id: Option<&'a str>) -> &'a str {
        override_id.unwrap_or(&self.config.cash_register_id)
    }

    fn payment_payload(&self, code: char, request: &PaymentRequest) -> Result<String> {
        let builder = if code == 'X' {
            crate::protocol::build_extended_payment
        } else {
            crate::protocol::build_payment
        };
        builder(
            &self.config.terminal_id,
            self.cash_register_id_or(request.cash_register_id.as_deref()),
            request.amount_cents,
            request.payment_type.unwrap_or_default().as_digit(),
            request.card_already_present.unwrap_or(false),
            request.tokenization.is_some(),
            request.receipt_text.as_deref().unwrap_or(""),
        )
    }

    fn emit_state(&self, state: ConnectionState) {
        let cb = self.on_connection_state_change.lock().unwrap().clone();
        if let Some(cb) = cb {
            cb(state);
        }
    }

    /// Ensures an open connection. `is_connected()` performs the transport's (proactive,
    /// non-destructive) liveness probe, so a peer-closed/half-open socket — common because
    /// Nexi terminals close TCP between transactions — is detected here, before a command
    /// is sent on a stale socket.
    async fn ensure_connected(&mut self) -> Result<()> {
        if self.session.is_connected() {
            return Ok(());
        }
        self.emit_state(ConnectionState::Connecting);
        match self.session.connect().await {
            Ok(()) => {
                self.emit_state(ConnectionState::Connected);
                Ok(())
            }
            Err(e) => {
                self.emit_state(ConnectionState::Disconnected);
                Err(e)
            }
        }
    }

    async fn do_exchange(
        &mut self,
        main_payload: &str,
        tokenization: Option<&TokenizationRequest>,
    ) -> Result<DecodedPacket> {
        if let Some(tok) = tokenization {
            let tag = crate::protocol::format_tokenization_tag(
                tok.service.is_recurring(),
                &tok.contract_code,
            )?;
            let additional = crate::protocol::build_additional_tags(
                &self.config.terminal_id,
                &tag,
                "62",
                "DF8D01",
            )?;
            self.session
                .exchange_with_additional_data(main_payload, &additional)
                .await
        } else {
            self.session.exchange(main_payload).await
        }
    }

    /// Runs an exchange with the money-safe auto-reconnect policy: on a mid-command error,
    /// reconnect (if `auto_reconnect` and the transport dropped), then replay ONLY when
    /// [`should_retry_after_reconnect`] allows it — never a financial op.
    async fn run_transaction(
        &mut self,
        main_payload: &str,
        tokenization: Option<&TokenizationRequest>,
        safe_to_retry: bool,
    ) -> Result<DecodedPacket> {
        match self.do_exchange(main_payload, tokenization).await {
            Ok(pkt) => Ok(pkt),
            Err(original) => {
                if self.recover_after_error(safe_to_retry).await {
                    self.do_exchange(main_payload, tokenization).await
                } else {
                    Err(original)
                }
            }
        }
    }

    /// Like [`run_transaction`](Self::run_transaction) for commands whose only reply is the
    /// physical ACK (`E`/`R`).
    async fn run_ack_only(&mut self, payload: &str, safe_to_retry: bool) -> Result<()> {
        match self.session.send_ack_only(payload).await {
            Ok(()) => Ok(()),
            Err(original) => {
                if self.recover_after_error(safe_to_retry).await {
                    self.session.send_ack_only(payload).await
                } else {
                    Err(original)
                }
            }
        }
    }

    /// Reconnects after a mid-command error (when `auto_reconnect` and the transport
    /// dropped) and returns whether the caller may **replay** the command. Returns `false`
    /// for a financial op (never replay → surface the original error) or a failed reconnect.
    async fn recover_after_error(&mut self, safe_to_retry: bool) -> bool {
        let auto_reconnect = self.config.auto_reconnect.unwrap_or(false);
        let dropped = !self.session.is_connected();
        if auto_reconnect && dropped && self.ensure_connected().await.is_err() {
            return false; // reconnect failed → the caller surfaces the original error
        }
        should_retry_after_reconnect(auto_reconnect, dropped, safe_to_retry)
    }
}

fn session_config(config: &Ecr17Config) -> SessionConfig {
    SessionConfig {
        lrc_mode: config.lrc_mode.unwrap_or_default(),
        ack_timeout_ms: u64::from(config.ack_timeout_ms.unwrap_or(2000)),
        response_timeout_ms: u64::from(config.response_timeout_ms.unwrap_or(60_000)),
        retry_count: config.retry_count.unwrap_or(3),
        retry_delay_ms: u64::from(config.retry_delay_ms.unwrap_or(200)),
        receipt_drain_ms: u64::from(config.receipt_drain_ms.unwrap_or(0)),
    }
}

fn as_str(pkt: &DecodedPacket) -> String {
    String::from_utf8_lossy(&pkt.payload).into_owned()
}

// --- raw response -> typed result mappers (port of the C++ map* helpers) ---

fn opt_str(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn opt_i64(s: &str) -> Option<i64> {
    s.trim().parse::<i64>().ok()
}

fn opt_f64(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn opt_i32(s: &str) -> Option<i32> {
    s.trim().parse::<i32>().ok()
}

fn map_card_type(raw: &str) -> Option<CardType> {
    match raw {
        "1" => Some(CardType::Debit),
        "2" => Some(CardType::Credit),
        "3" => Some(CardType::Other),
        _ => None,
    }
}

fn map_entry_mode(raw: &str) -> Option<TransactionEntryMode> {
    match raw {
        "ICC" => Some(TransactionEntryMode::Icc),
        "MAG" => Some(TransactionEntryMode::Mag),
        "MAN" => Some(TransactionEntryMode::Manual),
        "CLM" => Some(TransactionEntryMode::ClessMag),
        "CLI" => Some(TransactionEntryMode::ClessIcc),
        _ => None,
    }
}

fn map_payment(p: &response::PaymentResponse) -> PaymentResult {
    PaymentResult {
        outcome: p.outcome,
        result_code: p.result_code.clone(),
        pan: opt_str(&p.pan),
        entry_mode: map_entry_mode(&p.transaction_type),
        auth_code: opt_str(&p.auth_code),
        host_date_time: opt_str(&p.host_date_time),
        card_type: map_card_type(&p.card_type),
        acquirer_id: opt_str(&p.acquirer_id),
        stan: opt_str(&p.stan),
        online_id: opt_str(&p.online_id),
        error_description: opt_str(&p.error_description),
        // DCC values are exposed as the terminal's RAW integers (rate/amount unscaled),
        // with `precision` giving the number of implied decimals — matching the reference.
        // Consumers scale with `precision` (e.g. rate 12345 @ precision 4 = 1.2345); the
        // library does not scale so no rounding is imposed on money values.
        currency_exchange: p.currency.applied.then(|| CurrencyExchange {
            applied: true,
            rate: opt_f64(&p.currency.rate),
            currency_code: opt_str(&p.currency.currency_code),
            amount_cents: opt_i64(&p.currency.amount),
            precision: opt_i32(&p.currency.precision),
        }),
    }
}

fn map_reversal(p: &response::PaymentResponse) -> ReversalResult {
    ReversalResult {
        outcome: p.outcome,
        result_code: p.result_code.clone(),
        pan: opt_str(&p.pan),
        entry_mode: map_entry_mode(&p.transaction_type),
        host_date_time: opt_str(&p.host_date_time),
        card_type: map_card_type(&p.card_type),
        acquirer_id: opt_str(&p.acquirer_id),
        stan: opt_str(&p.stan),
        online_id: opt_str(&p.online_id),
        action_code: None,
        error_description: opt_str(&p.error_description),
    }
}

fn map_card_verify(p: &response::PaymentResponse) -> CardVerificationResult {
    CardVerificationResult {
        outcome: p.outcome,
        result_code: p.result_code.clone(),
        pan: opt_str(&p.pan),
        entry_mode: map_entry_mode(&p.transaction_type),
        auth_code: opt_str(&p.auth_code),
        host_date_time: opt_str(&p.host_date_time),
        card_type: map_card_type(&p.card_type),
        acquirer_id: opt_str(&p.acquirer_id),
        stan: opt_str(&p.stan),
        online_id: opt_str(&p.online_id),
        action_code: None,
        error_description: opt_str(&p.error_description),
    }
}

fn map_pre_auth(p: &response::PreAuthResponse) -> PreAuthResult {
    PreAuthResult {
        outcome: p.outcome,
        result_code: p.result_code.clone(),
        pan: opt_str(&p.pan),
        entry_mode: map_entry_mode(&p.transaction_type),
        auth_code: opt_str(&p.auth_code),
        pre_authorized_amount_cents: opt_i64(&p.pre_authorized_amount),
        pre_auth_code: opt_str(&p.pre_auth_code),
        action_code: opt_str(&p.action_code),
        host_date_time: opt_str(&p.host_date_time),
        card_type: map_card_type(&p.card_type),
        acquirer_id: opt_str(&p.acquirer_id),
        stan: opt_str(&p.stan),
        online_id: opt_str(&p.online_id),
        error_description: opt_str(&p.error_description),
    }
}

fn map_status(s: &response::StatusResponse) -> PosStatusResponse {
    PosStatusResponse {
        terminal_id: s.terminal_id.clone(),
        terminal_date_time: raw_datetime_to_iso(&s.date_time_raw),
        status: s.status,
        software_release: s.software_release.clone(),
    }
}

fn map_totals(t: &response::TotalsResponse) -> TotalsResult {
    TotalsResult {
        outcome: t.outcome,
        result_code: t.result_code.clone(),
        pos_total_cents: opt_i64(&t.pos_total).unwrap_or(0),
    }
}

fn map_close(c: &response::CloseResponse) -> CloseSessionResult {
    CloseSessionResult {
        outcome: c.outcome,
        result_code: c.result_code.clone(),
        pos_total_cents: opt_i64(&c.pos_total),
        host_total_cents: opt_i64(&c.host_total),
        action_code: opt_str(&c.action_code),
        error_description: opt_str(&c.error_description),
    }
}

fn map_vas(v: &response::VasResponse) -> VasResult {
    VasResult {
        response_id: v.response_id.clone(),
        response_message: v.response_message.clone(),
        order_id: opt_str(&v.order_id),
        raw_xml: v.raw_xml.clone(),
    }
}

/// Converts the terminal's raw `DDMMYYhhmm` to an ISO 8601 string
/// (`20YY-MM-DDThh:mm:00`); empty string if the input is too short.
fn raw_datetime_to_iso(raw: &str) -> String {
    if raw.len() < 10 {
        return String::new();
    }
    let (dd, mm, yy, hh, min) = (&raw[0..2], &raw[2..4], &raw[4..6], &raw[6..8], &raw[8..10]);
    format!("20{yy}-{mm}-{dd}T{hh}:{min}:00")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{PacketCodec, ACK, EOT, SOH};
    use crate::error::Ecr17Error;
    use crate::lrc::LrcMode;
    use crate::transport::FakeTransport;
    use crate::types::PaymentCardType;

    fn config() -> Ecr17Config {
        Ecr17Config {
            host: "127.0.0.1".into(),
            port: None,
            terminal_id: "12345678".into(),
            cash_register_id: "00000001".into(),
            lrc_mode: Some(LrcMode::Std),
            keep_alive: None,
            auto_reconnect: Some(true),
            connection_timeout_ms: None,
            response_timeout_ms: Some(40),
            ack_timeout_ms: Some(40),
            receipt_drain_ms: None,
            retry_count: Some(2),
            retry_delay_ms: Some(1),
            debug: None,
        }
    }

    // A minimal spec-shaped payment result ('E' code, result "00" = OK) with a PAN.
    fn ok_payment_result() -> Vec<u8> {
        let c = PacketCodec::new(LrcMode::Std);
        // header(8 id +'0'+'E') + result "00" + pan(19) + txType"ICC"(3) + auth"AUTH01"(6)
        // + hostDate(7) + cardType"2" + acquirer"ACQ"(11) + stan(6) + online(6)
        let mut payload = String::from("123456780E00");
        payload.push_str(&"0".repeat(19)); // pan
        payload.push_str("ICC");
        payload.push_str("AUTH01");
        payload.push_str("2111520");
        payload.push('2');
        payload.push_str("ACQ        ");
        payload.push_str("000042");
        payload.push_str("000099");
        c.encode_application(payload.as_bytes())
    }

    fn ack_then(result: Vec<u8>) -> Vec<u8> {
        let mut r = PacketCodec::new(LrcMode::Std).encode_control(ACK);
        r.extend(result);
        r
    }

    #[tokio::test]
    async fn pay_maps_an_approved_result() {
        let mut t = FakeTransport::new();
        t.enqueue_response(ack_then(ok_payment_result()));
        let mut client = Ecr17Client::new(t, config());

        let req = PaymentRequest {
            amount_cents: 650,
            cash_register_id: None,
            payment_type: Some(PaymentCardType::Credit),
            card_already_present: None,
            receipt_text: None,
            tokenization: None,
        };
        let result = client.pay(&req).await.unwrap();
        assert_eq!(result.outcome, crate::types::TransactionOutcome::Ok);
        assert_eq!(result.result_code, "00");
        assert_eq!(result.entry_mode, Some(TransactionEntryMode::Icc));
        assert_eq!(result.card_type, Some(CardType::Credit));
        assert_eq!(result.auth_code.as_deref(), Some("AUTH01"));
        assert_eq!(result.stan.as_deref(), Some("000042"));
    }

    #[tokio::test]
    async fn client_forwards_progress_to_listener() {
        let c = PacketCodec::new(LrcMode::Std);
        let mut response = c.encode_control(ACK);
        let mut progress_frame = vec![SOH];
        progress_frame.extend_from_slice(b"ATTENDERE PREGO     ");
        progress_frame.push(EOT);
        response.extend(progress_frame);
        response.extend(ok_payment_result());
        let mut t = FakeTransport::new();
        t.enqueue_response(response);
        let mut client = Ecr17Client::new(t, config());

        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&seen);
        client.set_on_progress(move |e| sink.lock().unwrap().push(e.message));

        let req = PaymentRequest {
            amount_cents: 650,
            cash_register_id: None,
            payment_type: None,
            card_already_present: None,
            receipt_text: None,
            tokenization: None,
        };
        client.pay(&req).await.unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            vec!["ATTENDERE PREGO     ".to_string()]
        );
    }

    #[tokio::test]
    async fn status_maps_iso_datetime() {
        let c = PacketCodec::new(LrcMode::Std);
        // termId(8) '0' 's' reserved(10) dateTime"0102251530" status"2" sw"V1.2.3"
        let mut payload = String::from("123456780s");
        payload.push_str(&"0".repeat(10));
        payload.push_str("0102251530");
        payload.push('2');
        payload.push_str("V1.2.3");
        let mut t = FakeTransport::new();
        t.enqueue_response(ack_then(c.encode_application(payload.as_bytes())));
        let mut client = Ecr17Client::new(t, config());

        let s = client.status().await.unwrap();
        assert_eq!(s.terminal_id, "12345678");
        assert_eq!(s.status, 2);
        assert_eq!(s.terminal_date_time, "2025-02-01T15:30:00");
        assert_eq!(s.software_release, "V1.2.3");
    }

    // 💰 A financial command that drops mid-exchange is NOT replayed even with
    // auto_reconnect on; the error surfaces (recover via send_last_result).
    #[tokio::test]
    async fn financial_command_not_replayed_on_drop() {
        let mut t = FakeTransport::new();
        t.disconnect_on_next_request();
        let mut client = Ecr17Client::new(t, config()); // auto_reconnect = true

        let req = PaymentRequest {
            amount_cents: 1000,
            cash_register_id: None,
            payment_type: None,
            card_already_present: None,
            receipt_text: None,
            tokenization: None,
        };
        let err = client.pay(&req).await.unwrap_err();
        assert!(matches!(err, Ecr17Error::Disconnected));
        // Exactly one application request was sent — never a blind financial replay.
        assert_eq!(client.transport().application_request_count(), 1);
    }

    // A safe/idempotent command (status) IS retried after an auto-reconnect.
    #[tokio::test]
    async fn safe_command_retried_after_reconnect() {
        let c = PacketCodec::new(LrcMode::Std);
        let mut t = FakeTransport::new();
        t.disconnect_on_next_request(); // first status attempt drops
                                        // status payload the second attempt will get an ACK + status result
        let mut payload = String::from("123456780s");
        payload.push_str(&"0".repeat(10));
        payload.push_str("0102251530");
        payload.push('2');
        payload.push_str("V1");
        t.enqueue_response(ack_then(c.encode_application(payload.as_bytes())));
        let mut client = Ecr17Client::new(t, config());

        let s = client.status().await.unwrap();
        assert_eq!(s.status, 2);
        // initial (dropped) + retry after reconnect = 2 application requests.
        assert_eq!(client.transport().application_request_count(), 2);
    }

    #[test]
    fn iso_datetime_conversion() {
        assert_eq!(raw_datetime_to_iso("0102251530"), "2025-02-01T15:30:00");
        assert_eq!(raw_datetime_to_iso("short"), "");
    }
}
