//! Real tokio TCP [`Transport`] to an ECR17 terminal (feature `tokio-transport`).

use std::task::{Context, Poll, Waker};
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;

use crate::error::{Ecr17Error, Result};
use crate::transport::Transport;

/// A TCP connection to an ECR17 terminal.
///
/// [`is_connected`](Transport::is_connected) performs a **non-destructive liveness probe**
/// (a `MSG_PEEK`-style read that never consumes a byte): it detects a peer-closed/half-open
/// socket — common because Nexi terminals close TCP between transactions — so the client can
/// reconnect BEFORE sending a command on a stale socket (avoiding a false money-safety error
/// on a financial command). See `docs/LESSON.md`.
#[derive(Debug)]
pub struct TcpTransport {
    host: String,
    port: u16,
    connect_timeout: Duration,
    stream: Option<TcpStream>,
}

impl TcpTransport {
    /// Creates a transport targeting `host:port` (not yet connected).
    pub fn new(host: impl Into<String>, port: u16, connect_timeout: Duration) -> Self {
        Self {
            host: host.into(),
            port,
            connect_timeout,
            stream: None,
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = tokio::time::timeout(self.connect_timeout, TcpStream::connect(&addr))
            .await
            .map_err(|_| Ecr17Error::Transport {
                kind: std::io::ErrorKind::TimedOut,
                message: format!("connect to {addr} timed out"),
            })??;
        // Best-effort latency optimization for the request/response handshake. A failure
        // here is non-fatal (Nagle stays on), so we intentionally do not fail connect().
        let _ = stream.set_nodelay(true);
        self.stream = Some(stream);
        Ok(())
    }

    async fn disconnect(&mut self) {
        // Dropping the stream closes the socket.
        self.stream = None;
    }

    fn is_connected(&self) -> bool {
        let Some(stream) = self.stream.as_ref() else {
            return false;
        };
        // Non-destructive liveness probe: peek 1 byte without consuming it.
        let mut byte = [0u8; 1];
        let mut buf = ReadBuf::new(&mut byte);
        let mut cx = Context::from_waker(Waker::noop());
        match stream.poll_peek(&mut cx, &mut buf) {
            // Ready with 0 bytes == EOF (peer sent FIN) → dead; >0 bytes buffered → alive.
            Poll::Ready(Ok(n)) => n > 0,
            // A socket error → dead.
            Poll::Ready(Err(_)) => false,
            // No data available right now, but the connection is open → alive.
            Poll::Pending => true,
        }
    }

    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(Ecr17Error::Disconnected)?;
        stream.write_all(bytes).await?;
        stream.flush().await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        let stream = self.stream.as_mut().ok_or(Ecr17Error::Disconnected)?;
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            // EOF: the peer closed the connection.
            self.stream = None;
            return Err(Ecr17Error::Disconnected);
        }
        Ok(buf[..n].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn connect_send_recv_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 16];
            let n = sock.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"PING");
            sock.write_all(b"PONG").await.unwrap();
            sock.flush().await.unwrap();
            // keep the socket open briefly so the client's recv completes
            tokio::time::sleep(Duration::from_millis(50)).await;
        });

        let mut t = TcpTransport::new(addr.ip().to_string(), addr.port(), Duration::from_secs(2));
        assert!(!t.is_connected());
        t.connect().await.unwrap();
        assert!(t.is_connected());
        t.send(b"PING").await.unwrap();
        assert_eq!(t.recv().await.unwrap(), b"PONG");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn recv_errors_when_peer_closes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (sock, _) = listener.accept().await.unwrap();
            drop(sock); // close immediately
        });

        let mut t = TcpTransport::new(addr.ip().to_string(), addr.port(), Duration::from_secs(2));
        t.connect().await.unwrap();
        server.await.unwrap();
        assert_eq!(t.recv().await, Err(Ecr17Error::Disconnected));
        assert!(!t.is_connected());
    }

    #[tokio::test]
    async fn send_and_recv_error_when_not_connected() {
        let mut t = TcpTransport::new("127.0.0.1", 1, Duration::from_millis(100));
        assert_eq!(t.send(b"x").await, Err(Ecr17Error::Disconnected));
        assert_eq!(t.recv().await, Err(Ecr17Error::Disconnected));
    }

    // Opt-in integration test against a REAL Nexi terminal. Runs only when
    // ECR17_TEST_HOST is set, e.g.:
    //   ECR17_TEST_HOST=192.168.1.50 ECR17_TEST_TID=12345678 \
    //   cargo test -p ecr17-protocol --features tokio-transport -- --ignored real_terminal
    #[tokio::test]
    #[ignore = "requires a real terminal via ECR17_TEST_HOST"]
    async fn real_terminal_status() {
        let Ok(host) = std::env::var("ECR17_TEST_HOST") else {
            eprintln!("ECR17_TEST_HOST not set — skipping");
            return;
        };
        let port: u16 = std::env::var("ECR17_TEST_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10_000);
        let tid = std::env::var("ECR17_TEST_TID").unwrap_or_else(|_| "12345678".into());
        let crn = std::env::var("ECR17_TEST_CRN").unwrap_or_else(|_| "00000001".into());

        let transport = TcpTransport::new(host, port, Duration::from_secs(5));
        let config = crate::types::Ecr17Config {
            host: String::new(),
            port: Some(port),
            terminal_id: tid,
            cash_register_id: crn,
            lrc_mode: None,
            keep_alive: Some(true),
            auto_reconnect: Some(true),
            connection_timeout_ms: Some(5000),
            response_timeout_ms: Some(60_000),
            ack_timeout_ms: Some(2000),
            receipt_drain_ms: None,
            retry_count: Some(3),
            retry_delay_ms: Some(200),
            debug: Some(true),
        };
        let mut client = crate::client::Ecr17Client::new(transport, config);
        let status = client.status().await.expect("status");
        eprintln!("terminal status: {status:?}");
    }
}
