//! [`Ecr17Session`] — drives one ECR17 request/response exchange over a [`Transport`].
//!
//! It frames the request, runs the physical `ACK`/`NAK` handshake with retransmission,
//! waits for the application response while forwarding progress (`SOH`) and receipt (`S`)
//! messages, and `ACK`/`NAK`s incoming frames per their LRC validity. One exchange runs at
//! a time (ECR17 is one transaction per terminal). Port of the reference C++ `Ecr17Session`.
//!
//! 💰 The session throws [`Ecr17Error::Disconnected`] on a mid-exchange drop and resets its
//! per-transaction state at the start of every exchange, so it is reusable across
//! reconnects. It never blindly re-sends: the money-safe retry decision lives in
//! [`crate::retry`] and is applied by the client, and a lost response is recovered via
//! `send_last_result` (`G`), never by retransmitting a financial request.

use std::time::{Duration, Instant};

use crate::codec::{DecodedPacket, PacketCodec, PacketType, ACK, ETX, NAK, SOH, STX};
use crate::error::{Ecr17Error, Result};
use crate::lrc::LrcMode;
use crate::transport::Transport;

/// Session timing/retry configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionConfig {
    /// LRC framing mode.
    pub lrc_mode: LrcMode,
    /// Wait for the physical `ACK`/`NAK` (ms).
    pub ack_timeout_ms: u64,
    /// Wait for the application response (ms).
    pub response_timeout_ms: u64,
    /// Retransmissions on `NAK`/timeout (spec: up to 3).
    pub retry_count: u32,
    /// Delay between retransmissions (ms).
    pub retry_delay_ms: u64,
    /// After the result, keep forwarding `S` receipt lines until this many ms of silence
    /// (`0` = off).
    pub receipt_drain_ms: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            lrc_mode: LrcMode::Std,
            ack_timeout_ms: 2000,
            response_timeout_ms: 60_000,
            retry_count: 3,
            retry_delay_ms: 200,
            receipt_drain_ms: 0,
        }
    }
}

type EventCallback = Box<dyn Fn(String) + Send + 'static>;

/// Outcome of waiting for the next frame.
enum WaitOutcome {
    Frame(DecodedPacket),
    Timeout,
    Disconnected,
}

/// Drives ECR17 exchanges over a [`Transport`] `T`.
pub struct Ecr17Session<T: Transport> {
    transport: T,
    config: SessionConfig,
    codec: PacketCodec,
    rx_buffer: Vec<u8>,
    /// Holds an application response that arrived during the ACK handshake (some terminals
    /// send the result before/without a physical ACK). Consumed by `wait_for_result` so a
    /// completed transaction's result is never dropped.
    pending_result: Option<DecodedPacket>,
    on_progress: Option<EventCallback>,
    on_receipt_line: Option<EventCallback>,
}

impl<T: Transport> std::fmt::Debug for Ecr17Session<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ecr17Session")
            .field("config", &self.config)
            .field("rx_buffered", &self.rx_buffer.len())
            .field("has_pending_result", &self.pending_result.is_some())
            .finish_non_exhaustive()
    }
}

impl<T: Transport> Ecr17Session<T> {
    /// Creates a session over `transport` with `config`.
    pub fn new(transport: T, config: SessionConfig) -> Self {
        let codec = PacketCodec::new(config.lrc_mode);
        Self {
            transport,
            config,
            codec,
            rx_buffer: Vec::new(),
            pending_result: None,
            on_progress: None,
            on_receipt_line: None,
        }
    }

    /// Sets the progress-update callback (`SOH` frames during a procedure).
    pub fn set_on_progress(&mut self, cb: impl Fn(String) + Send + 'static) {
        self.on_progress = Some(Box::new(cb));
    }

    /// Sets the receipt-line callback (`S` messages when ECR printing is on).
    pub fn set_on_receipt_line(&mut self, cb: impl Fn(String) + Send + 'static) {
        self.on_receipt_line = Some(Box::new(cb));
    }

    /// Borrows the underlying transport (e.g. to reconnect it).
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Mutably borrows the underlying transport.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Opens the transport connection.
    pub async fn connect(&mut self) -> Result<()> {
        self.transport.connect().await
    }

    /// Closes the transport connection.
    pub async fn disconnect(&mut self) {
        self.transport.disconnect().await;
    }

    /// Whether the transport currently believes it is connected.
    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    /// Sends `request_payload` (the application message, without STX/ETX) and returns the
    /// decoded application result. Errors on retransmission exhaustion, ACK/response
    /// timeout, or transport disconnect.
    pub async fn exchange(&mut self, request_payload: &str) -> Result<DecodedPacket> {
        self.reset_for_new_transaction();
        self.ack_handshake(request_payload).await?;
        self.wait_for_result().await
    }

    /// Like [`exchange`](Self::exchange) but sends an extra additional-data message (`U`,
    /// tokenization) after the main request is ACKed, before the result:
    /// `request(flag=1) -> ACK -> 'U' -> ACK -> result`.
    pub async fn exchange_with_additional_data(
        &mut self,
        request_payload: &str,
        additional_payload: &str,
    ) -> Result<DecodedPacket> {
        self.reset_for_new_transaction();
        self.ack_handshake(request_payload).await?;
        self.ack_handshake(additional_payload).await?;
        self.wait_for_result().await
    }

    /// For commands whose only reply is the physical `ACK` (e.g. enable/disable ECR
    /// printing `E`): performs the ACK handshake with retransmission and returns once ACK
    /// is received; does NOT wait for an application response.
    pub async fn send_ack_only(&mut self, request_payload: &str) -> Result<()> {
        self.reset_for_new_transaction();
        self.ack_handshake(request_payload).await
    }

    // --- internals ---

    /// Clears stale RX bytes and any stashed result so the session is reusable across
    /// reconnects (a new transaction starts from a clean state).
    fn reset_for_new_transaction(&mut self) {
        self.rx_buffer.clear();
        self.pending_result = None;
    }

    /// Sends a request and completes the physical ACK handshake (with retransmission).
    async fn ack_handshake(&mut self, request_payload: &str) -> Result<()> {
        let frame = self.codec.encode_application(request_payload.as_bytes());
        self.transport.send(&frame).await?;
        let ack_timeout = Duration::from_millis(self.config.ack_timeout_ms);
        let mut attempts: u32 = 1;
        let mut deadline = Instant::now() + ack_timeout;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                if attempts > self.config.retry_count {
                    return Err(Ecr17Error::AckTimeout { attempts });
                }
                self.retransmit(&frame, &mut attempts, &mut deadline, ack_timeout)
                    .await?;
                continue;
            }
            match self.wait_for_frame(remaining).await {
                WaitOutcome::Frame(pkt) => match pkt.packet_type {
                    PacketType::Ack => return Ok(()),
                    PacketType::Nak => {
                        if attempts > self.config.retry_count {
                            return Err(Ecr17Error::Nak { attempts });
                        }
                        self.retransmit(&frame, &mut attempts, &mut deadline, ack_timeout)
                            .await?;
                    }
                    PacketType::Application => {
                        // Some terminals send the result before/without a physical ACK.
                        self.pending_result = Some(pkt);
                        return Ok(());
                    }
                    // Ignore progress / unknown frames that may precede the ACK.
                    _ => {}
                },
                WaitOutcome::Disconnected => return Err(Ecr17Error::Disconnected),
                // Timed out waiting; loop re-checks the deadline and retransmits.
                WaitOutcome::Timeout => {}
            }
        }
    }

    async fn retransmit(
        &mut self,
        frame: &[u8],
        attempts: &mut u32,
        deadline: &mut Instant,
        ack_timeout: Duration,
    ) -> Result<()> {
        tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
        self.transport.send(frame).await?;
        *attempts += 1;
        *deadline = Instant::now() + ack_timeout;
        Ok(())
    }

    /// Waits for the application result after the ACK handshake, forwarding progress and
    /// receipt frames and NAKing invalid-LRC frames.
    async fn wait_for_result(&mut self) -> Result<DecodedPacket> {
        let deadline = Instant::now() + Duration::from_millis(self.config.response_timeout_ms);
        loop {
            let pkt = if let Some(p) = self.pending_result.take() {
                p
            } else {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(Ecr17Error::ResponseTimeout);
                }
                match self.wait_for_frame(remaining).await {
                    WaitOutcome::Frame(p) => p,
                    WaitOutcome::Disconnected => return Err(Ecr17Error::Disconnected),
                    WaitOutcome::Timeout => continue,
                }
            };
            match pkt.packet_type {
                PacketType::Progress => self.emit_progress(&pkt.payload),
                PacketType::Application => {
                    if !pkt.valid_lrc {
                        self.send_control(NAK).await;
                    } else {
                        self.send_control(ACK).await;
                        if Self::is_receipt(&pkt.payload) {
                            self.emit_receipt(&pkt.payload);
                        } else {
                            self.drain_receipts().await;
                            return Ok(pkt);
                        }
                    }
                }
                // Stray confirmation frames are ignored.
                PacketType::Ack | PacketType::Nak => {}
                PacketType::Unknown => self.send_control(NAK).await,
            }
        }
    }

    /// After the result, keep forwarding `S` receipt lines that arrive until the terminal
    /// is quiet for `receipt_drain_ms` (`0` = off).
    async fn drain_receipts(&mut self) {
        if self.config.receipt_drain_ms == 0 {
            return;
        }
        let drain = Duration::from_millis(self.config.receipt_drain_ms);
        loop {
            match self.wait_for_frame(drain).await {
                WaitOutcome::Frame(pkt) => match pkt.packet_type {
                    PacketType::Application => {
                        if pkt.valid_lrc {
                            self.send_control(ACK).await;
                            if Self::is_receipt(&pkt.payload) {
                                self.emit_receipt(&pkt.payload);
                            }
                        } else {
                            self.send_control(NAK).await;
                        }
                    }
                    PacketType::Progress => self.emit_progress(&pkt.payload),
                    _ => {}
                },
                // Idle (no more receipts) or dropped: stop draining.
                WaitOutcome::Timeout | WaitOutcome::Disconnected => return,
            }
        }
    }

    /// Extracts one complete frame from the front of `rx_buffer`, dropping leading junk
    /// bytes to resynchronize. `None` if no complete frame is available yet.
    fn extract_frame(&mut self) -> Option<Vec<u8>> {
        while let Some(&first) = self.rx_buffer.first() {
            if first == ACK || first == NAK {
                if self.rx_buffer.len() < 3 {
                    return None; // wait for ETX + LRC
                }
                return Some(self.rx_buffer.drain(0..3).collect());
            }
            if first == STX {
                let Some(etx) = self.rx_buffer.iter().position(|&b| b == ETX) else {
                    return None; // wait for ETX
                };
                if etx + 1 >= self.rx_buffer.len() {
                    return None; // wait for the trailing LRC
                }
                return Some(self.rx_buffer.drain(0..=etx + 1).collect());
            }
            if first == SOH {
                let Some(eot) = self.rx_buffer.iter().position(|&b| b == crate::codec::EOT) else {
                    return None; // wait for EOT
                };
                return Some(self.rx_buffer.drain(0..=eot).collect());
            }
            // Unrecognized lead byte: drop it and resynchronize.
            self.rx_buffer.remove(0);
        }
        None
    }

    /// Waits up to `timeout` for the next decodable frame, buffering inbound bytes.
    async fn wait_for_frame(&mut self, timeout: Duration) -> WaitOutcome {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(frame) = self.extract_frame() {
                return WaitOutcome::Frame(self.codec.decode(&frame));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return WaitOutcome::Timeout;
            }
            match tokio::time::timeout(remaining, self.transport.recv()).await {
                Ok(Ok(bytes)) => self.rx_buffer.extend_from_slice(&bytes),
                Ok(Err(_dropped)) => return WaitOutcome::Disconnected,
                Err(_elapsed) => return WaitOutcome::Timeout,
            }
        }
    }

    async fn send_control(&mut self, control: u8) {
        let frame = self.codec.encode_control(control);
        // Best effort: a failed ACK/NAK send means we're already disconnected, which the
        // next read surfaces.
        let _ = self.transport.send(&frame).await;
    }

    fn emit_progress(&self, payload: &[u8]) {
        if let Some(cb) = &self.on_progress {
            cb(String::from_utf8_lossy(payload).into_owned());
        }
    }

    fn emit_receipt(&self, payload: &[u8]) {
        if let Some(cb) = &self.on_receipt_line {
            cb(String::from_utf8_lossy(payload).into_owned());
        }
    }

    /// A send-ticket ('S') message has code `'S'` at position 10 (0-indexed 9).
    fn is_receipt(payload: &[u8]) -> bool {
        payload.get(9) == Some(&b'S')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::FakeTransport;
    use std::sync::{Arc, Mutex};

    fn fast_config() -> SessionConfig {
        SessionConfig {
            lrc_mode: LrcMode::Std,
            ack_timeout_ms: 40,
            response_timeout_ms: 40,
            retry_count: 2,
            retry_delay_ms: 1,
            receipt_drain_ms: 0,
        }
    }

    fn codec() -> PacketCodec {
        PacketCodec::new(LrcMode::Std)
    }

    fn concat(mut a: Vec<u8>, b: Vec<u8>) -> Vec<u8> {
        a.extend(b);
        a
    }

    fn progress_frame(msg20: &str) -> Vec<u8> {
        let mut f = vec![SOH];
        f.extend_from_slice(msg20.as_bytes());
        f.push(crate::codec::EOT);
        f
    }

    const RESULT: &str = "123456780E0000DATA"; // code 'E' at pos 10 -> result
    const RECEIPT: &str = "123456780SLINE 1"; // code 'S' at pos 10 -> receipt

    fn sent_any(t: &FakeTransport, lead: u8) -> bool {
        t.sent_frames().iter().any(|f| f.first() == Some(&lead))
    }

    #[tokio::test]
    async fn happy_path_returns_result_and_acks() {
        let c = codec();
        let mut t = FakeTransport::new();
        t.enqueue_response(concat(
            c.encode_control(ACK),
            c.encode_application(RESULT.as_bytes()),
        ));
        let mut session = Ecr17Session::new(t, fast_config());

        let result = session.exchange("123456780P...").await.unwrap();
        assert_eq!(result.packet_type, PacketType::Application);
        assert!(result.valid_lrc);
        assert_eq!(result.payload, RESULT.as_bytes());
        assert_eq!(session.transport().application_request_count(), 1);
        assert!(sent_any(session.transport(), ACK));
    }

    #[tokio::test]
    async fn nak_triggers_retransmit_then_succeeds() {
        let c = codec();
        let mut t = FakeTransport::new();
        t.enqueue_response(c.encode_control(NAK)); // reply to attempt 1
        t.enqueue_response(concat(
            c.encode_control(ACK),
            c.encode_application(RESULT.as_bytes()),
        ));
        let mut session = Ecr17Session::new(t, fast_config());

        let result = session.exchange("123456780P...").await.unwrap();
        assert_eq!(result.payload, RESULT.as_bytes());
        assert_eq!(session.transport().application_request_count(), 2); // initial + 1 retransmit
    }

    #[tokio::test]
    async fn ack_timeout_exhausts_retries_then_errors() {
        let mut session = Ecr17Session::new(FakeTransport::new(), fast_config());
        let err = session.exchange("123456780P...").await.unwrap_err();
        assert!(matches!(err, Ecr17Error::AckTimeout { attempts: 3 }));
        assert_eq!(session.transport().application_request_count(), 3); // initial + retry_count
    }

    #[tokio::test]
    async fn bad_lrc_response_sends_nak() {
        let c = codec();
        let mut t = FakeTransport::new();
        let mut bad = c.encode_application(RESULT.as_bytes());
        *bad.last_mut().unwrap() ^= 0xFF; // corrupt LRC
        t.enqueue_response(concat(c.encode_control(ACK), bad));
        let mut session = Ecr17Session::new(t, fast_config());

        let err = session.exchange("123456780P...").await.unwrap_err();
        assert!(matches!(err, Ecr17Error::ResponseTimeout));
        assert!(sent_any(session.transport(), NAK));
    }

    #[tokio::test]
    async fn progress_messages_forwarded() {
        let c = codec();
        let mut response = c.encode_control(ACK);
        response.extend(progress_frame("ATTENDERE PREGO     "));
        response.extend(c.encode_application(RESULT.as_bytes()));
        let mut t = FakeTransport::new();
        t.enqueue_response(response);

        let progress = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&progress);
        let mut session = Ecr17Session::new(t, fast_config());
        session.set_on_progress(move |m| sink.lock().unwrap().push(m));

        let result = session.exchange("123456780P...").await.unwrap();
        assert_eq!(result.payload, RESULT.as_bytes());
        assert_eq!(
            *progress.lock().unwrap(),
            vec!["ATTENDERE PREGO     ".to_string()]
        );
    }

    #[tokio::test]
    async fn receipt_lines_forwarded_before_result() {
        let c = codec();
        let mut response = c.encode_control(ACK);
        response.extend(c.encode_application(RECEIPT.as_bytes()));
        response.extend(c.encode_application(RESULT.as_bytes()));
        let mut t = FakeTransport::new();
        t.enqueue_response(response);

        let receipts = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&receipts);
        let mut session = Ecr17Session::new(t, fast_config());
        session.set_on_receipt_line(move |l| sink.lock().unwrap().push(l));

        let result = session.exchange("123456780P...").await.unwrap();
        assert_eq!(result.payload, RESULT.as_bytes());
        assert_eq!(*receipts.lock().unwrap(), vec![RECEIPT.to_string()]);
    }

    // Regression: some terminals send the RESULT before (or instead of) the physical ACK.
    #[tokio::test]
    async fn result_before_ack_is_not_lost() {
        let c = codec();
        let mut t = FakeTransport::new();
        t.enqueue_response(c.encode_application(RESULT.as_bytes())); // result, no leading ACK
        let mut session = Ecr17Session::new(t, fast_config());

        let result = session.exchange("123456780P...").await.unwrap();
        assert_eq!(result.payload, RESULT.as_bytes());
        assert_eq!(session.transport().application_request_count(), 1); // no spurious retransmit
        assert!(sent_any(session.transport(), ACK));
    }

    #[tokio::test]
    async fn response_timeout_after_ack_errors() {
        let c = codec();
        let mut t = FakeTransport::new();
        t.enqueue_response(c.encode_control(ACK)); // ACK only, no result
        let mut session = Ecr17Session::new(t, fast_config());
        assert!(matches!(
            session.exchange("123456780P...").await,
            Err(Ecr17Error::ResponseTimeout)
        ));
    }

    #[tokio::test]
    async fn disconnect_during_exchange_errors() {
        let mut t = FakeTransport::new();
        t.disconnect_on_next_request();
        let mut session = Ecr17Session::new(t, fast_config());
        assert!(matches!(
            session.exchange("123456780P...").await,
            Err(Ecr17Error::Disconnected)
        ));
    }

    // 💰 The session must recover after a drop — no stale disconnected state.
    #[tokio::test]
    async fn recovers_and_succeeds_after_reconnect() {
        let mut t = FakeTransport::new();
        t.disconnect_on_next_request();
        let mut session = Ecr17Session::new(t, fast_config());

        assert!(matches!(
            session.exchange("123456780P...").await,
            Err(Ecr17Error::Disconnected)
        ));

        let c = codec();
        session.transport_mut().rearm();
        session.transport_mut().enqueue_response(concat(
            c.encode_control(ACK),
            c.encode_application(RESULT.as_bytes()),
        ));

        let result = session.exchange("123456780P...").await.unwrap();
        assert_eq!(result.payload, RESULT.as_bytes());
    }

    #[tokio::test]
    async fn send_ack_only_returns_on_ack() {
        let c = codec();
        let mut t = FakeTransport::new();
        t.enqueue_response(c.encode_control(ACK));
        let mut session = Ecr17Session::new(t, fast_config());
        session.send_ack_only("123456780E1").await.unwrap();
        assert_eq!(session.transport().application_request_count(), 1);
    }

    #[tokio::test]
    async fn send_ack_only_retransmits_on_nak() {
        let c = codec();
        let mut t = FakeTransport::new();
        t.enqueue_response(c.encode_control(NAK));
        t.enqueue_response(c.encode_control(ACK));
        let mut session = Ecr17Session::new(t, fast_config());
        session.send_ack_only("123456780E0").await.unwrap();
        assert_eq!(session.transport().application_request_count(), 2);
    }

    #[tokio::test]
    async fn send_ack_only_times_out() {
        let mut session = Ecr17Session::new(FakeTransport::new(), fast_config());
        assert!(matches!(
            session.send_ack_only("123456780E1").await,
            Err(Ecr17Error::AckTimeout { .. })
        ));
    }

    #[tokio::test]
    async fn exchange_with_additional_data_sends_two_requests() {
        let c = codec();
        let mut t = FakeTransport::new();
        t.enqueue_response(c.encode_control(ACK)); // ACK for the main 'P'
        t.enqueue_response(concat(
            c.encode_control(ACK),
            c.encode_application(RESULT.as_bytes()),
        ));
        let mut session = Ecr17Session::new(t, fast_config());

        let result = session
            .exchange_with_additional_data("123456780P...", "123456780U...")
            .await
            .unwrap();
        assert_eq!(result.payload, RESULT.as_bytes());
        assert_eq!(session.transport().application_request_count(), 2); // P + U
    }

    #[tokio::test]
    async fn receipt_drain_forwards_receipts_after_result() {
        let c = codec();
        let mut response = c.encode_control(ACK);
        response.extend(c.encode_application(RESULT.as_bytes())); // result first
        response.extend(c.encode_application(RECEIPT.as_bytes())); // then a receipt line
        let mut t = FakeTransport::new();
        t.enqueue_response(response);

        let mut cfg = fast_config();
        cfg.receipt_drain_ms = 30;
        let receipts = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&receipts);
        let mut session = Ecr17Session::new(t, cfg);
        session.set_on_receipt_line(move |l| sink.lock().unwrap().push(l));

        let result = session.exchange("123456780P...").await.unwrap();
        assert_eq!(result.payload, RESULT.as_bytes());
        assert_eq!(*receipts.lock().unwrap(), vec![RECEIPT.to_string()]);
    }
}
