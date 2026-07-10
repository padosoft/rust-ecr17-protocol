//! LRC checksum and the [`LrcMode`] framing selector.
//!
//! The ECR17 application frame is `STX payload ETX LRC`. The LRC starts from the base
//! `0x7F` and XOR-folds the payload; which of the framing bytes (`STX`/`ETX`) are also
//! folded is selected by [`LrcMode`], because it varies by terminal/firmware.
//!
//! Port of the reference C++ `Lcr`.

use serde::{Deserialize, Serialize};

/// LRC base value (`0x7F`).
pub const BASE: u8 = 0x7F;

const STX: u8 = 0x02;
const ETX: u8 = 0x03;

/// Which framing bytes are folded into the LRC (base [`BASE`]):
///
/// - [`LrcMode::Stx`] — `STX + payload + ETX`
/// - [`LrcMode::Std`] — payload only
/// - [`LrcMode::Noext`] — `payload + ETX`
/// - [`LrcMode::StxNoext`] — `STX + payload`
///
/// Serializes to the wire/JSON string union `"stx" | "std" | "noext" | "stx_noext"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LrcMode {
    /// `STX + payload + ETX`.
    Stx,
    /// payload only.
    #[default]
    Std,
    /// `payload + ETX`.
    Noext,
    /// `STX + payload`.
    StxNoext,
}

impl LrcMode {
    /// Computes the LRC of `payload` under this mode.
    ///
    /// ```
    /// use ecr17_protocol::lrc::LrcMode;
    /// assert_eq!(LrcMode::Std.compute(b""), 0x7F);
    /// assert_eq!(LrcMode::Stx.compute(b""), 0x7E); // 0x7F ^ STX ^ ETX
    /// ```
    #[must_use]
    pub fn compute(self, payload: &[u8]) -> u8 {
        let mut lrc = BASE;
        if matches!(self, LrcMode::Stx | LrcMode::StxNoext) {
            lrc ^= STX;
        }
        for &b in payload {
            lrc ^= b;
        }
        if matches!(self, LrcMode::Stx | LrcMode::Noext) {
            lrc ^= ETX;
        }
        lrc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference implementation kept intentionally independent from the production
    // code, so the tests assert against first principles rather than a copy.
    fn reference(payload: &[u8], mode: LrcMode) -> u8 {
        let mut lrc = 0x7Fu8;
        if mode == LrcMode::Stx || mode == LrcMode::StxNoext {
            lrc ^= 0x02;
        }
        for &b in payload {
            lrc ^= b;
        }
        if mode == LrcMode::Stx || mode == LrcMode::Noext {
            lrc ^= 0x03;
        }
        lrc
    }

    #[test]
    fn empty_payload_std_is_base() {
        assert_eq!(LrcMode::Std.compute(&[]), BASE);
    }

    #[test]
    fn empty_payload_stx_folds_stx_and_etx() {
        // 0x7F ^ 0x02 ^ 0x03 == 0x7E
        assert_eq!(LrcMode::Stx.compute(&[]), 0x7E);
    }

    #[test]
    fn known_vector_all_modes() {
        let payload = *b"A"; // 0x41
        assert_eq!(LrcMode::Std.compute(&payload), 0x3E);
        assert_eq!(LrcMode::Stx.compute(&payload), 0x3F);
        assert_eq!(LrcMode::Noext.compute(&payload), 0x3D);
        assert_eq!(LrcMode::StxNoext.compute(&payload), 0x3C);
    }

    #[test]
    fn matches_reference_for_every_mode() {
        let payload = [0x00, 0x7F, 0x55, 0xAA, b'Z', 0x10];
        for mode in [
            LrcMode::Stx,
            LrcMode::Std,
            LrcMode::Noext,
            LrcMode::StxNoext,
        ] {
            assert_eq!(mode.compute(&payload), reference(&payload, mode));
        }
    }

    #[test]
    fn str_and_bytes_agree() {
        let payload = "12345678P0";
        for mode in [
            LrcMode::Stx,
            LrcMode::Std,
            LrcMode::Noext,
            LrcMode::StxNoext,
        ] {
            assert_eq!(
                mode.compute(payload.as_bytes()),
                mode.compute(&payload.bytes().collect::<Vec<_>>())
            );
        }
    }

    #[test]
    fn serde_snake_case_round_trip() {
        assert_eq!(
            serde_json::to_string(&LrcMode::StxNoext).unwrap(),
            "\"stx_noext\""
        );
        assert_eq!(serde_json::to_string(&LrcMode::Std).unwrap(), "\"std\"");
        let m: LrcMode = serde_json::from_str("\"noext\"").unwrap();
        assert_eq!(m, LrcMode::Noext);
    }
}
