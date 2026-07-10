//! Request/result/config data model (serde), mirroring the reference `types/client.ts`
//! field-for-field. Enums serialize to the same JSON string unions as the TS API and
//! struct fields use `camelCase`, so the model maps cleanly onto the Tauri IPC. Absent
//! optional fields deserialize to `None`; a `None` serializes as JSON `null` (the TS side
//! treats `null`/absent equivalently for these optional fields).

use serde::{Deserialize, Serialize};

use crate::lrc::LrcMode;

// ---------------------------------------------------------------------------
// Enums (string unions)
// ---------------------------------------------------------------------------

/// Transport connection lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    /// Not connected.
    #[default]
    Disconnected,
    /// Connection in progress.
    Connecting,
    /// Connected and ready.
    Connected,
}

/// Normalized transaction outcome (mapped from the raw 2-digit result code):
/// `"00" -> ok`, `"01" -> ko`, `"05" -> cardNotPresent`, `"09" -> unknownTag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TransactionOutcome {
    /// Approved.
    Ok,
    /// Declined / failed.
    Ko,
    /// The card was not present when required.
    CardNotPresent,
    /// The requested TAG was unknown to the terminal.
    UnknownTag,
    /// Unrecognized result code.
    #[default]
    Unknown,
}

/// Card category reported by the terminal (response "Card type": 1/2/3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CardType {
    /// Debit card.
    Debit,
    /// Credit card.
    Credit,
    /// Other card product.
    Other,
    /// Unknown / not reported.
    #[default]
    Unknown,
}

/// How the card was read (response "Transaction type": ICC/MAG/MAN/CLM/CLI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TransactionEntryMode {
    /// Chip (ICC).
    Icc,
    /// Magnetic stripe.
    Mag,
    /// Manual PAN entry.
    Manual,
    /// Contactless magstripe.
    ClessMag,
    /// Contactless chip.
    ClessIcc,
    /// Unknown / not reported.
    #[default]
    Unknown,
}

/// Requested card handling for a payment (request "Payment type": 0..3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PaymentCardType {
    /// Let the terminal decide (`'0'`).
    #[default]
    Auto,
    /// Force debit (`'1'`).
    Debit,
    /// Force credit (`'2'`).
    Credit,
    /// Other (`'3'`).
    Other,
}

impl PaymentCardType {
    /// The single request digit (`'0'`..`'3'`) for this card handling.
    #[must_use]
    pub fn as_digit(self) -> char {
        match self {
            PaymentCardType::Auto => '0',
            PaymentCardType::Debit => '1',
            PaymentCardType::Credit => '2',
            PaymentCardType::Other => '3',
        }
    }
}

/// Tokenization service requested alongside a payment/preauth/verification (command `U`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenizationService {
    /// Recurring (`0REC`).
    Recurring,
    /// Unscheduled / one-click (`0COF`).
    UnscheduledOrOneClick,
}

impl TokenizationService {
    /// Whether this maps to the recurring (`0REC`) mapping vs one-click (`0COF`).
    #[must_use]
    pub fn is_recurring(self) -> bool {
        matches!(self, TokenizationService::Recurring)
    }
}

/// Tokenization additional-data request (`U`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizationRequest {
    /// Which tokenization mapping to request.
    pub service: TokenizationService,
    /// Unique contract code at merchant level, alphanumeric, up to 18 chars.
    pub contract_code: String,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Client/session configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ecr17Config {
    /// POS terminal host (IP address).
    pub host: String,
    /// TCP port (terminal default when omitted).
    pub port: Option<u16>,

    /// Terminal identifier (max 8 chars).
    pub terminal_id: String,
    /// Cash-register identifier (max 8 chars).
    pub cash_register_id: String,

    /// LRC framing mode.
    pub lrc_mode: Option<LrcMode>,

    /// Keep the socket open between transactions.
    pub keep_alive: Option<bool>,
    /// Reconnect automatically on a mid-session drop (financial ops are still never
    /// blindly replayed — recover via `send_last_result`).
    pub auto_reconnect: Option<bool>,

    /// Connection timeout (ms).
    pub connection_timeout_ms: Option<u32>,
    /// Application-response timeout (ms).
    pub response_timeout_ms: Option<u32>,
    /// ACK timeout (ms).
    pub ack_timeout_ms: Option<u32>,

    /// After a transaction result, keep forwarding `S` receipt lines until this many ms
    /// of silence. `0`/`None` = off. Set when ECR-printing is on.
    pub receipt_drain_ms: Option<u32>,

    /// Retransmission attempts on ACK timeout / NAK.
    pub retry_count: Option<u32>,
    /// Delay between retransmissions (ms).
    pub retry_delay_ms: Option<u32>,

    /// Verbose debug logging.
    pub debug: Option<bool>,
}

/// POS terminal status code (response `s`), `-1`..`6`.
pub type PosTerminalStatus = i32;

/// Human-readable message for a [`PosTerminalStatus`].
#[must_use]
pub fn pos_terminal_status_message(status: PosTerminalStatus) -> &'static str {
    match status {
        0 => "Terminal not configured",
        1 => "Terminal configured, no DLL",
        2 => "Terminal operative (after a DLL)",
        3 => "Terminal not aligned (first DLL requested)",
        4 => "KMPB/KPOS key corrupted (first DLL requested)",
        5 => "DLL solicited by GT pending",
        6 => "Remote SW updated request pending",
        _ => "Unknown",
    }
}

/// Terminal status response (`s`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PosStatusResponse {
    /// Terminal identifier echoed back.
    pub terminal_id: String,
    /// Terminal date/time as an **ISO 8601** string (e.g. `"2026-07-10T14:30:00"`),
    /// parsed by the `response` layer from the terminal's raw `DDMMYYhhmm`. A string
    /// (not a native datetime) keeps the library dependency-free; the frontend maps it
    /// with `new Date(...)`, preserving the RN API's date contract.
    pub terminal_date_time: String,
    /// Status code (`-1`..`6`); see [`pos_terminal_status_message`].
    pub status: PosTerminalStatus,
    /// Firmware/software release string.
    pub software_release: String,
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

/// Payment / extended-payment / pre-auth request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequest {
    /// Amount in cents.
    pub amount_cents: i64,
    /// Overrides the configured cash-register id when set.
    pub cash_register_id: Option<String>,
    /// Requested card handling.
    pub payment_type: Option<PaymentCardType>,
    /// Start with the card already inserted in the terminal.
    pub card_already_present: Option<bool>,
    /// Text printed at the end of the receipt (max 128 chars).
    pub receipt_text: Option<String>,
    /// Attach tokenization additional data (`U`) to this transaction.
    pub tokenization: Option<TokenizationRequest>,
}

/// Reversal request (`S`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReversalRequest {
    /// Overrides the configured cash-register id when set.
    pub cash_register_id: Option<String>,
    /// STAN of the transaction to reverse. When omitted (`None`), the client uses
    /// `"000000"`, which reverses the last payment with no STAN check.
    pub stan: Option<String>,
}

/// Pre-auth request (`p`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreAuthRequest {
    /// Amount in cents.
    pub amount_cents: i64,
    /// Overrides the configured cash-register id when set.
    pub cash_register_id: Option<String>,
    /// Requested card handling.
    pub payment_type: Option<PaymentCardType>,
    /// Start with the card already inserted.
    pub card_already_present: Option<bool>,
    /// Receipt text.
    pub receipt_text: Option<String>,
    /// Attach tokenization additional data (`U`).
    pub tokenization: Option<TokenizationRequest>,
}

/// Incremental pre-auth request (`i`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncrementalAuthRequest {
    /// Amount in cents.
    pub amount_cents: i64,
    /// Unique pre-authorization code from the original pre-auth response.
    pub original_pre_auth_code: String,
    /// Overrides the configured cash-register id when set.
    pub cash_register_id: Option<String>,
    /// Receipt text.
    pub receipt_text: Option<String>,
}

/// Pre-auth closure request (`c`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreAuthClosureRequest {
    /// Amount in cents.
    pub amount_cents: i64,
    /// Unique pre-authorization code from the original pre-auth response.
    pub original_pre_auth_code: String,
    /// Overrides the configured cash-register id when set.
    pub cash_register_id: Option<String>,
    /// Receipt text.
    pub receipt_text: Option<String>,
}

/// Card-verification request (`H`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CardVerificationRequest {
    /// Overrides the configured cash-register id when set.
    pub cash_register_id: Option<String>,
    /// Requested card handling.
    pub payment_type: Option<PaymentCardType>,
    /// Attach tokenization additional data (`U`).
    pub tokenization: Option<TokenizationRequest>,
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// DCC / currency-exchange block (only meaningful when `applied == true`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyExchange {
    /// Whether DCC was applied.
    pub applied: bool,
    /// Exchange rate.
    pub rate: Option<f64>,
    /// Currency code.
    pub currency_code: Option<String>,
    /// Converted amount in cents.
    pub amount_cents: Option<i64>,
    /// Decimal precision.
    pub precision: Option<i32>,
}

/// Payment / extended-payment / closure result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResult {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code (`"00"`/`"01"`/`"05"`/`"09"`).
    pub result_code: String,
    /// Masked PAN.
    pub pan: Option<String>,
    /// Entry mode.
    pub entry_mode: Option<TransactionEntryMode>,
    /// Authorization code.
    pub auth_code: Option<String>,
    /// Raw host date/time (`DDDHHMM` as received).
    pub host_date_time: Option<String>,
    /// Card type.
    pub card_type: Option<CardType>,
    /// Acquirer id.
    pub acquirer_id: Option<String>,
    /// STAN.
    pub stan: Option<String>,
    /// Online id.
    pub online_id: Option<String>,
    /// Error description (when declined).
    pub error_description: Option<String>,
    /// DCC block.
    pub currency_exchange: Option<CurrencyExchange>,
}

/// Reversal result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReversalResult {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code.
    pub result_code: String,
    /// Masked PAN.
    pub pan: Option<String>,
    /// Entry mode.
    pub entry_mode: Option<TransactionEntryMode>,
    /// Raw host date/time.
    pub host_date_time: Option<String>,
    /// Card type.
    pub card_type: Option<CardType>,
    /// Acquirer id.
    pub acquirer_id: Option<String>,
    /// STAN.
    pub stan: Option<String>,
    /// Online id.
    pub online_id: Option<String>,
    /// Action code.
    pub action_code: Option<String>,
    /// Error description.
    pub error_description: Option<String>,
}

/// Pre-auth / incremental result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreAuthResult {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code.
    pub result_code: String,
    /// Masked PAN.
    pub pan: Option<String>,
    /// Entry mode.
    pub entry_mode: Option<TransactionEntryMode>,
    /// Authorization code.
    pub auth_code: Option<String>,
    /// Pre-authorized amount in cents.
    pub pre_authorized_amount_cents: Option<i64>,
    /// Pre-auth code (for follow-up incremental/closure).
    pub pre_auth_code: Option<String>,
    /// Action code.
    pub action_code: Option<String>,
    /// Raw host date/time.
    pub host_date_time: Option<String>,
    /// Card type.
    pub card_type: Option<CardType>,
    /// Acquirer id.
    pub acquirer_id: Option<String>,
    /// STAN.
    pub stan: Option<String>,
    /// Online id.
    pub online_id: Option<String>,
    /// Error description.
    pub error_description: Option<String>,
}

/// Card-verification result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CardVerificationResult {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code.
    pub result_code: String,
    /// Masked PAN.
    pub pan: Option<String>,
    /// Entry mode.
    pub entry_mode: Option<TransactionEntryMode>,
    /// Authorization code.
    pub auth_code: Option<String>,
    /// Raw host date/time.
    pub host_date_time: Option<String>,
    /// Card type.
    pub card_type: Option<CardType>,
    /// Acquirer id.
    pub acquirer_id: Option<String>,
    /// STAN.
    pub stan: Option<String>,
    /// Online id.
    pub online_id: Option<String>,
    /// Action code.
    pub action_code: Option<String>,
    /// Error description.
    pub error_description: Option<String>,
}

/// Totals result (`T`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TotalsResult {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code.
    pub result_code: String,
    /// POS total in cents.
    pub pos_total_cents: i64,
}

/// Close-session result (`C`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CloseSessionResult {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code.
    pub result_code: String,
    /// POS total in cents.
    pub pos_total_cents: Option<i64>,
    /// Host total in cents.
    pub host_total_cents: Option<i64>,
    /// Action code.
    pub action_code: Option<String>,
    /// Error description.
    pub error_description: Option<String>,
}

/// VAS result (`K`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VasResult {
    /// Response id (`RESPID`; `"0"` = OK).
    pub response_id: String,
    /// Response message (`RESPMSG`).
    pub response_message: String,
    /// Order id when present.
    pub order_id: Option<String>,
    /// Full concatenated XML response.
    pub raw_xml: String,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Progress-update message shown on the ECR display during a procedure (`SOH` frame).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    /// Display message.
    pub message: String,
}

/// A single receipt line streamed by the terminal (`S` message) when ECR printing is on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLine {
    /// Receipt line text.
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_serialize_to_ts_string_unions() {
        assert_eq!(
            serde_json::to_string(&ConnectionState::Disconnected).unwrap(),
            "\"disconnected\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionOutcome::CardNotPresent).unwrap(),
            "\"cardNotPresent\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionOutcome::UnknownTag).unwrap(),
            "\"unknownTag\""
        );
        assert_eq!(
            serde_json::to_string(&CardType::Debit).unwrap(),
            "\"debit\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionEntryMode::ClessMag).unwrap(),
            "\"clessMag\""
        );
        assert_eq!(
            serde_json::to_string(&PaymentCardType::Auto).unwrap(),
            "\"auto\""
        );
        assert_eq!(
            serde_json::to_string(&TokenizationService::UnscheduledOrOneClick).unwrap(),
            "\"unscheduledOrOneClick\""
        );
    }

    #[test]
    fn payment_card_type_digit_mapping() {
        assert_eq!(PaymentCardType::Auto.as_digit(), '0');
        assert_eq!(PaymentCardType::Debit.as_digit(), '1');
        assert_eq!(PaymentCardType::Credit.as_digit(), '2');
        assert_eq!(PaymentCardType::Other.as_digit(), '3');
    }

    #[test]
    fn request_deserializes_camel_case_and_defaults_missing_options() {
        // Only the required field present -> optionals default to None.
        let r: PaymentRequest = serde_json::from_str(r#"{"amountCents":650}"#).unwrap();
        assert_eq!(r.amount_cents, 650);
        assert_eq!(r.payment_type, None);
        assert_eq!(r.card_already_present, None);

        let full: PaymentRequest = serde_json::from_str(
            r#"{"amountCents":100,"paymentType":"credit","cardAlreadyPresent":true,"receiptText":"x"}"#,
        )
        .unwrap();
        assert_eq!(full.payment_type, Some(PaymentCardType::Credit));
        assert_eq!(full.card_already_present, Some(true));
        assert_eq!(full.receipt_text.as_deref(), Some("x"));
    }

    #[test]
    fn tokenization_request_round_trip() {
        let t = TokenizationRequest {
            service: TokenizationService::Recurring,
            contract_code: "ABC".into(),
        };
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"{"service":"recurring","contractCode":"ABC"}"#);
        assert_eq!(
            serde_json::from_str::<TokenizationRequest>(&json).unwrap(),
            t
        );
        assert!(t.service.is_recurring());
    }

    #[test]
    fn status_message_lookup() {
        assert_eq!(
            pos_terminal_status_message(2),
            "Terminal operative (after a DLL)"
        );
        assert_eq!(pos_terminal_status_message(-1), "Unknown");
        assert_eq!(pos_terminal_status_message(99), "Unknown");
    }

    #[test]
    fn config_round_trip_camel_case() {
        let json =
            r#"{"host":"10.0.0.5","terminalId":"12345678","cashRegisterId":"1","lrcMode":"stx"}"#;
        let cfg: Ecr17Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.host, "10.0.0.5");
        assert_eq!(cfg.lrc_mode, Some(LrcMode::Stx));
        assert_eq!(cfg.port, None);
    }
}
