//! The async [`Transport`] abstraction and an in-memory [`FakeTransport`] for tests.
//!
//! The protocol engine never touches sockets directly — all I/O goes through `Transport`,
//! so the [`crate::session::Ecr17Session`] can be unit-tested deterministically against
//! `FakeTransport`. The real tokio TCP transport is added in MACRO 5 (behind the
//! `tokio-transport` feature); until then, `FakeTransport` is the only implementation.

use std::collections::VecDeque;

use async_trait::async_trait;

use crate::error::{Ecr17Error, Result};

/// A bidirectional byte transport to an ECR17 terminal.
///
/// The session drives it strictly sequentially within an exchange (send, then await
/// [`recv`](Transport::recv)), so implementations need not be internally concurrent.
#[async_trait]
pub trait Transport: Send {
    /// Opens the connection. Idempotent if already connected.
    async fn connect(&mut self) -> Result<()>;
    /// Closes the connection.
    async fn disconnect(&mut self);
    /// Whether the transport currently believes it is connected.
    fn is_connected(&self) -> bool;
    /// Sends raw bytes (a fully framed packet).
    async fn send(&mut self, bytes: &[u8]) -> Result<()>;
    /// Awaits the next inbound chunk of bytes. Returns [`Ecr17Error::Disconnected`] if the
    /// transport dropped. When no data is available it pends until data arrives or the
    /// caller's timeout cancels the future.
    async fn recv(&mut self) -> Result<Vec<u8>>;
}

/// In-memory [`Transport`] for unit tests. Deterministic: `enqueue_response` scripts a
/// reply that is delivered when the session sends the next **application** request (a frame
/// starting with `STX`); control sends (`ACK`/`NAK`) are only recorded. This lets a test
/// script "ACK + result", "NAK then ACK+result", progress/receipt streams, mid-exchange
/// drops, or no reply at all (to exercise timeouts) with no real sockets or threads.
#[derive(Debug)]
pub struct FakeTransport {
    connected: bool,
    disconnect_on_request: bool,
    sent: Vec<Vec<u8>>,
    responses: VecDeque<Vec<u8>>,
    pending: VecDeque<Vec<u8>>,
}

impl Default for FakeTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeTransport {
    /// Start of an application frame.
    pub const STX: u8 = 0x02;

    /// Creates a fake transport that is already connected (a test double is "talking to a
    /// terminal", so it starts connected; call [`disconnect`](Transport::disconnect) or
    /// [`disconnect_on_next_request`](Self::disconnect_on_next_request) to model a drop).
    #[must_use]
    pub fn new() -> Self {
        Self {
            connected: true,
            disconnect_on_request: false,
            sent: Vec::new(),
            responses: VecDeque::new(),
            pending: VecDeque::new(),
        }
    }

    /// Queues a scripted terminal reply, delivered on the next application request.
    pub fn enqueue_response(&mut self, bytes: Vec<u8>) {
        self.responses.push_back(bytes);
    }

    /// Makes the next application-request send drop the connection instead of replying.
    pub fn disconnect_on_next_request(&mut self) {
        self.disconnect_on_request = true;
    }

    /// Simulates a successful reconnect: clears the drop flag and marks connected.
    pub fn rearm(&mut self) {
        self.disconnect_on_request = false;
        self.connected = true;
    }

    /// All frames the session has sent, in order.
    #[must_use]
    pub fn sent_frames(&self) -> &[Vec<u8>] {
        &self.sent
    }

    /// How many application requests (frames starting with `STX`) were sent.
    #[must_use]
    pub fn application_request_count(&self) -> usize {
        self.sent
            .iter()
            .filter(|f| f.first() == Some(&Self::STX))
            .count()
    }
}

#[async_trait]
impl Transport for FakeTransport {
    async fn connect(&mut self) -> Result<()> {
        // A real reconnect establishes a fresh socket, so clear any simulated drop state
        // (this is what the session's client relies on to recover after a drop).
        self.connected = true;
        self.disconnect_on_request = false;
        Ok(())
    }

    async fn disconnect(&mut self) {
        self.connected = false;
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Ecr17Error::Disconnected);
        }
        self.sent.push(bytes.to_vec());
        let is_application_request = bytes.first() == Some(&Self::STX);
        if is_application_request {
            if self.disconnect_on_request {
                self.connected = false; // the send lands on a socket that then drops
            } else if let Some(next) = self.responses.pop_front() {
                self.pending.push_back(next);
            }
        }
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        if let Some(bytes) = self.pending.pop_front() {
            return Ok(bytes);
        }
        if !self.connected {
            return Err(Ecr17Error::Disconnected);
        }
        // Connected but nothing to deliver: pend forever. The session always calls recv
        // inside a timeout, which cancels this future when the wait elapses.
        std::future::pending::<()>().await;
        unreachable!("pending() never resolves")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{PacketCodec, ACK};
    use crate::lrc::LrcMode;

    // A realistic on-the-wire control frame (`ctrl + ETX + LRC`), as the session sends/
    // receives — not a bare 0x06 byte.
    fn ack_frame() -> Vec<u8> {
        PacketCodec::new(LrcMode::Std).encode_control(ACK)
    }

    #[tokio::test]
    async fn delivers_enqueued_response_on_application_send() {
        let mut t = FakeTransport::new();
        t.enqueue_response(ack_frame());
        t.send(&[FakeTransport::STX, b'X']).await.unwrap();
        assert_eq!(t.recv().await.unwrap(), ack_frame());
        assert_eq!(t.application_request_count(), 1);
    }

    #[tokio::test]
    async fn control_send_does_not_consume_a_response() {
        let mut t = FakeTransport::new();
        t.enqueue_response(ack_frame());
        t.send(&ack_frame()).await.unwrap(); // a control frame, not an app request
        assert_eq!(t.application_request_count(), 0);
        // The response is still queued for the next real application request.
        t.send(&[FakeTransport::STX]).await.unwrap();
        assert_eq!(t.recv().await.unwrap(), ack_frame());
    }

    #[tokio::test]
    async fn send_and_recv_require_a_connection() {
        let mut t = FakeTransport::new();
        assert!(t.is_connected()); // a fresh fake starts connected
        t.disconnect().await;
        assert!(!t.is_connected());
        assert_eq!(
            t.send(&[FakeTransport::STX]).await,
            Err(Ecr17Error::Disconnected)
        );
        assert_eq!(t.recv().await, Err(Ecr17Error::Disconnected));
        // connect() restores it.
        t.connect().await.unwrap();
        t.enqueue_response(ack_frame());
        t.send(&[FakeTransport::STX]).await.unwrap();
        assert_eq!(t.recv().await.unwrap(), ack_frame());
    }

    #[tokio::test]
    async fn disconnect_on_next_request_makes_recv_error() {
        let mut t = FakeTransport::new();
        t.disconnect_on_next_request();
        t.send(&[FakeTransport::STX, b'P']).await.unwrap();
        assert!(!t.is_connected());
        assert_eq!(t.recv().await, Err(Ecr17Error::Disconnected));
        // After a reconnect the transport delivers again.
        t.rearm();
        t.enqueue_response(ack_frame());
        t.send(&[FakeTransport::STX]).await.unwrap();
        assert_eq!(t.recv().await.unwrap(), ack_frame());
    }
}
