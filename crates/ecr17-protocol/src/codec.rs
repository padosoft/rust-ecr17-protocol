//! ECR17 packet framing: encode/decode `STX payload ETX LRC` application frames,
//! `SOH message EOT` progress updates (no LRC), and the `ACK`/`NAK` control bytes.
//!
//! [`PacketCodec::decode`] treats the input buffer as **exactly one frame** (the LRC is
//! the final byte). Splitting a coalesced byte stream into individual frames is the
//! transport layer's responsibility, not the codec's.
//!
//! Port of the reference C++ `PacketCodec`.

use crate::lrc::LrcMode;

/// Start of an application frame.
pub const STX: u8 = 0x02;
/// End of the application payload (precedes the LRC).
pub const ETX: u8 = 0x03;
/// Start of a progress-update frame.
pub const SOH: u8 = 0x01;
/// End of a progress-update frame (its final byte; no LRC).
pub const EOT: u8 = 0x04;
/// Positive acknowledgement control byte.
pub const ACK: u8 = 0x06;
/// Negative acknowledgement control byte.
pub const NAK: u8 = 0x15;

/// The kind of frame [`PacketCodec::decode`] recognized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// `STX payload ETX LRC` application message.
    Application,
    /// `SOH message EOT` procedure progress update (no LRC).
    Progress,
    /// Positive acknowledgement.
    Ack,
    /// Negative acknowledgement.
    Nak,
    /// Unrecognized / malformed / truncated / coalesced buffer.
    Unknown,
}

/// A decoded frame. For [`PacketType::Application`], [`validLrc`](DecodedPacket::valid_lrc)
/// reports whether the received LRC matched the recomputed one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPacket {
    /// Recognized frame kind.
    pub packet_type: PacketType,
    /// Payload bytes (between the framing bytes); empty for control/unknown frames.
    pub payload: Vec<u8>,
    /// Whether the frame's LRC verified (always `true` for `ACK`/`NAK`/`PROGRESS`,
    /// always `false` for `UNKNOWN`).
    pub valid_lrc: bool,
}

impl DecodedPacket {
    fn unknown() -> Self {
        Self {
            packet_type: PacketType::Unknown,
            payload: Vec::new(),
            valid_lrc: false,
        }
    }

    fn control(packet_type: PacketType) -> Self {
        Self {
            packet_type,
            payload: Vec::new(),
            valid_lrc: true,
        }
    }
}

/// Frames and parses ECR17 packets for a given [`LrcMode`].
#[derive(Debug, Clone, Copy)]
pub struct PacketCodec {
    lrc_mode: LrcMode,
}

impl PacketCodec {
    /// Creates a codec that computes LRCs under `mode`.
    #[must_use]
    pub fn new(mode: LrcMode) -> Self {
        Self { lrc_mode: mode }
    }

    /// The configured LRC mode.
    #[must_use]
    pub fn lrc_mode(&self) -> LrcMode {
        self.lrc_mode
    }

    /// Encodes an application frame: `STX + payload + ETX + LRC`.
    #[must_use]
    pub fn encode_application(&self, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(payload.len() + 3);
        frame.push(STX);
        frame.extend_from_slice(payload);
        frame.push(ETX);
        frame.push(self.lrc_mode.compute(payload));
        frame
    }

    /// Encodes a control frame: `ctrl + ETX + LRC(ctrl)`.
    #[must_use]
    pub fn encode_control(&self, ctrl: u8) -> Vec<u8> {
        vec![ctrl, ETX, self.lrc_mode.compute(&[ctrl])]
    }

    /// Decodes exactly one frame from `data`.
    ///
    /// `ACK`/`NAK` are recognized by their **lead byte** only: on the wire a control
    /// frame is `ctrl + ETX + LRC` (what [`encode_control`](Self::encode_control)
    /// produces and what the session frames), so `[ACK, ETX, LRC]` decodes to
    /// [`PacketType::Ack`]. This leniency is required — the session hands `decode` the
    /// full 3-byte control frame, and requiring a bare 1-byte `ACK` would make every
    /// transaction's ACK handshake fail. See `docs/LESSON.md`.
    ///
    /// For `STX`/`SOH` frames the buffer must be exactly one frame: more than one frame,
    /// trailing bytes after the LRC, a missing LRC, or a progress frame not terminated by
    /// `EOT` all decode to [`PacketType::Unknown`].
    #[must_use]
    pub fn decode(&self, data: &[u8]) -> DecodedPacket {
        let Some(&first) = data.first() else {
            return DecodedPacket::unknown();
        };

        match first {
            ACK => return DecodedPacket::control(PacketType::Ack),
            NAK => return DecodedPacket::control(PacketType::Nak),
            _ => {}
        }

        if first == SOH {
            // Progress update: SOH + message + EOT (no LRC). Need at least SOH and the
            // trailing EOT, and the final byte MUST be EOT — reject garbage/truncation.
            if data.len() < 2 || *data.last().unwrap() != EOT {
                return DecodedPacket::unknown();
            }
            return DecodedPacket {
                packet_type: PacketType::Progress,
                payload: data[1..data.len() - 1].to_vec(),
                valid_lrc: true,
            };
        }

        if first == STX {
            let Some(etx_index) = data.iter().position(|&b| b == ETX) else {
                return DecodedPacket::unknown();
            };
            // A well-formed frame is exactly STX + payload + ETX + LRC, so the LRC must
            // be the final byte. Reject truncation (no LRC) and any trailing bytes (e.g.
            // a coalesced read holding a second frame) — framing a byte stream is the
            // transport's job.
            if etx_index + 2 != data.len() {
                return DecodedPacket::unknown();
            }
            let payload = &data[1..etx_index];
            let rx_lrc = data[etx_index + 1];
            let calc_lrc = self.lrc_mode.compute(payload);
            return DecodedPacket {
                packet_type: PacketType::Application,
                payload: payload.to_vec(),
                valid_lrc: rx_lrc == calc_lrc,
            };
        }

        DecodedPacket::unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_application_frames_stx_payload_etx_lrc() {
        let codec = PacketCodec::new(LrcMode::Std);
        let frame = codec.encode_application(b"AB");
        assert_eq!(frame, vec![STX, b'A', b'B', ETX, 0x7C]); // 0x7F ^ 'A' ^ 'B'
    }

    #[test]
    fn application_round_trip_all_modes() {
        for mode in [
            LrcMode::Stx,
            LrcMode::Std,
            LrcMode::Noext,
            LrcMode::StxNoext,
        ] {
            let codec = PacketCodec::new(mode);
            let payload = b"123456780P0000065000";
            let decoded = codec.decode(&codec.encode_application(payload));
            assert_eq!(decoded.packet_type, PacketType::Application);
            assert_eq!(decoded.payload, payload);
            assert!(decoded.valid_lrc);
        }
    }

    #[test]
    fn application_detects_corrupted_lrc() {
        let codec = PacketCodec::new(LrcMode::Std);
        let mut frame = codec.encode_application(b"HELLO");
        *frame.last_mut().unwrap() ^= 0xFF; // corrupt the LRC byte
        let decoded = codec.decode(&frame);
        assert_eq!(decoded.packet_type, PacketType::Application);
        assert_eq!(decoded.payload, b"HELLO");
        assert!(!decoded.valid_lrc);
    }

    #[test]
    fn encode_control_frames_ctrl_etx_lrc() {
        let codec = PacketCodec::new(LrcMode::Std);
        let frame = codec.encode_control(ACK);
        assert_eq!(frame.len(), 3);
        assert_eq!(frame[0], ACK);
        assert_eq!(frame[1], ETX);
    }

    #[test]
    fn decode_ack() {
        let decoded = PacketCodec::new(LrcMode::Std).decode(&[ACK]);
        assert_eq!(decoded.packet_type, PacketType::Ack);
        assert!(decoded.valid_lrc);
    }

    #[test]
    fn decode_nak() {
        let decoded = PacketCodec::new(LrcMode::Std).decode(&[NAK]);
        assert_eq!(decoded.packet_type, PacketType::Nak);
        assert!(decoded.valid_lrc);
    }

    // 💰 Money-critical invariant: the real 3-byte control frame `ctrl + ETX + LRC`
    // (what encode_control produces and what the session frames) MUST decode as
    // ACK/NAK by lead byte. Requiring a bare 1-byte control frame here would make every
    // transaction's ACK handshake fail. Do NOT "tighten" this to data.len() == 1.
    #[test]
    fn decode_full_control_frame_from_encode_control() {
        let codec = PacketCodec::new(LrcMode::Std);
        assert_eq!(
            codec.decode(&codec.encode_control(ACK)).packet_type,
            PacketType::Ack
        );
        assert_eq!(
            codec.decode(&codec.encode_control(NAK)).packet_type,
            PacketType::Nak
        );
    }

    #[test]
    fn decode_empty_is_unknown() {
        let decoded = PacketCodec::new(LrcMode::Std).decode(&[]);
        assert_eq!(decoded.packet_type, PacketType::Unknown);
        assert!(!decoded.valid_lrc);
    }

    // Regression: a lone SOH byte must not build an inverted slice range -> panic.
    #[test]
    fn decode_lone_soh_is_unknown_not_panic() {
        let decoded = PacketCodec::new(LrcMode::Std).decode(&[SOH]);
        assert_eq!(decoded.packet_type, PacketType::Unknown);
        assert!(!decoded.valid_lrc);
    }

    #[test]
    fn decode_progress_update() {
        let codec = PacketCodec::new(LrcMode::Std);
        let msg = b"ELABORAZIONE...     "; // 20 chars per spec
        let mut frame = vec![SOH];
        frame.extend_from_slice(msg);
        frame.push(EOT);
        let decoded = codec.decode(&frame);
        assert_eq!(decoded.packet_type, PacketType::Progress);
        assert_eq!(decoded.payload, msg);
    }

    #[test]
    fn decode_stx_without_etx_is_unknown() {
        let decoded = PacketCodec::new(LrcMode::Std).decode(&[STX, b'A', b'B']);
        assert_eq!(decoded.packet_type, PacketType::Unknown);
        assert!(!decoded.valid_lrc);
    }

    // Regression: ETX present but no trailing LRC must not read past the end nor
    // mistake ETX for the LRC.
    #[test]
    fn decode_stx_with_etx_but_no_lrc_is_unknown() {
        let decoded = PacketCodec::new(LrcMode::Std).decode(&[STX, b'A', ETX]);
        assert_eq!(decoded.packet_type, PacketType::Unknown);
        assert!(!decoded.valid_lrc);
    }

    #[test]
    fn decode_unknown_lead_byte() {
        let decoded = PacketCodec::new(LrcMode::Std).decode(&[0x99, 0x00]);
        assert_eq!(decoded.packet_type, PacketType::Unknown);
        assert!(!decoded.valid_lrc);
    }

    // Regression: trailing bytes after a complete frame's LRC must not be accepted.
    #[test]
    fn decode_stx_with_trailing_bytes_after_lrc_is_unknown() {
        let codec = PacketCodec::new(LrcMode::Std);
        let mut frame = codec.encode_application(b"AB"); // STX A B ETX LRC
        frame.push(0x00); // stray trailing byte
        let decoded = codec.decode(&frame);
        assert_eq!(decoded.packet_type, PacketType::Unknown);
        assert!(!decoded.valid_lrc);
    }

    // Regression: a coalesced read holding two frames must not be reported as one
    // valid APPLICATION packet (framing a byte stream is the transport's job).
    #[test]
    fn decode_coalesced_frames_is_unknown() {
        let codec = PacketCodec::new(LrcMode::Std);
        let mut first = codec.encode_application(b"AB");
        first.extend_from_slice(&codec.encode_application(b"CD"));
        let decoded = codec.decode(&first);
        assert_eq!(decoded.packet_type, PacketType::Unknown);
        assert!(!decoded.valid_lrc);
    }

    // Regression: SOH frame not terminated by EOT must not be accepted as PROGRESS.
    #[test]
    fn decode_soh_without_eot_is_unknown() {
        let codec = PacketCodec::new(LrcMode::Std);
        let mut frame = vec![SOH];
        frame.extend_from_slice(b"ELABORAZIONE...     ");
        frame.push(0xFF); // wrong terminator
        let decoded = codec.decode(&frame);
        assert_eq!(decoded.packet_type, PacketType::Unknown);
        assert!(!decoded.valid_lrc);
    }
}
