//! Parsers for ECR17 terminal *response* application messages.
//!
//! Each parser takes the application payload (the bytes between `STX` and `ETX`, i.e.
//! [`crate::codec::DecodedPacket::payload`] as text) and returns a plain raw struct with
//! string fields at the exact 1-based spec offsets. The `client` layer maps these raw
//! structs onto the typed [`crate::types`] results (enum/amount/date conversions).
//!
//! Parsing is **defensive**: a field starting beyond the payload comes back empty (and a
//! partial field is clamped) rather than panicking, so a short/truncated response degrades
//! gracefully. Port of the reference C++ `Ecr17Response`.

use crate::types::TransactionOutcome;

/// 1-based field extractor. Returns `""` if the field starts beyond the payload; clamps the
/// length to whatever bytes are actually present. Char-boundary safe (ECR17 fixed fields
/// are ASCII, so this never trims mid-character).
fn at(p: &str, pos1: usize, len: usize) -> &str {
    if pos1 == 0 || pos1 > p.len() {
        return "";
    }
    let i = pos1 - 1;
    let end = (i + len).min(p.len());
    p.get(i..end).unwrap_or("")
}

fn trim_right(s: &str) -> String {
    s.trim_end_matches(' ').to_string()
}

/// Extracts the value of a Nexi VAS XML param: `<p k="KEY">value</p>`.
fn xml_value(xml: &str, key: &str) -> String {
    let needle = format!("\"{key}\">");
    let Some(start) = xml.find(&needle) else {
        return String::new();
    };
    let from = start + needle.len();
    let rest = &xml[from..];
    let value = match rest.find('<') {
        Some(end) => &rest[..end],
        None => rest,
    };
    value.trim().to_string()
}

/// Maps the raw 2-digit result code to a [`TransactionOutcome`].
#[must_use]
pub fn outcome_from_code(code: &str) -> TransactionOutcome {
    match code {
        "00" => TransactionOutcome::Ok,
        "01" => TransactionOutcome::Ko,
        "05" => TransactionOutcome::CardNotPresent,
        "09" => TransactionOutcome::UnknownTag,
        _ => TransactionOutcome::Unknown,
    }
}

/// Optional DCC / currency-exchange block (parsed from a `'V'` response). Named `DccInfo`
/// to mirror the reference; the `client` maps it to [`crate::types::CurrencyExchange`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DccInfo {
    /// Whether DCC was applied (flag byte `== "1"`).
    pub applied: bool,
    /// Rate (8 digits, 4 decimals), raw.
    pub rate: String,
    /// Currency code (alpha-3), raw.
    pub currency_code: String,
    /// Converted amount (12 digits), raw.
    pub amount: String,
    /// Decimal precision, raw.
    pub precision: String,
}

/// Raw payment-family response (`'E'` without DCC, `'V'` with DCC). Reused for reversal /
/// card verification / pre-auth closure which share the layout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaymentResponse {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code (`"00"`/`"01"`/`"05"`/`"09"`).
    pub result_code: String,
    /// PAN (positive), raw.
    pub pan: String,
    /// Transaction/entry type (positive), raw `"ICC"`/`"MAG"`/…
    pub transaction_type: String,
    /// Authorization code (positive), raw.
    pub auth_code: String,
    /// Host date/time (positive), raw `DDDHHMM`.
    pub host_date_time: String,
    /// Error description (negative), raw.
    pub error_description: String,
    /// Card type (common), raw `"1"`/`"2"`/`"3"`.
    pub card_type: String,
    /// Acquirer id (common), raw.
    pub acquirer_id: String,
    /// STAN (common), raw.
    pub stan: String,
    /// Online id (common), raw.
    pub online_id: String,
    /// DCC block (only when a `'V'` response carried one).
    pub currency: DccInfo,
}

/// Raw status response (`'s'`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusResponse {
    /// Terminal id, raw.
    pub terminal_id: String,
    /// Date/time, raw `DDMMYYhhmm`.
    pub date_time_raw: String,
    /// Status code `0..=6`, or `-1` if unknown/missing.
    pub status: i32,
    /// Firmware/software release, raw.
    pub software_release: String,
}

/// Raw totals response (`'T'`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TotalsResponse {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code.
    pub result_code: String,
    /// POS total (16 digits, cents), raw.
    pub pos_total: String,
}

/// Raw close-session response (`'C'`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloseResponse {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code.
    pub result_code: String,
    /// POS total (positive, 16 digits), raw.
    pub pos_total: String,
    /// Host total (positive, 16 digits), raw.
    pub host_total: String,
    /// Error description (negative), raw.
    pub error_description: String,
    /// Action code (negative), raw.
    pub action_code: String,
}

/// Raw pre-auth response (`'e'`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreAuthResponse {
    /// Normalized outcome.
    pub outcome: TransactionOutcome,
    /// Raw result code.
    pub result_code: String,
    /// PAN, raw.
    pub pan: String,
    /// Transaction/entry type, raw.
    pub transaction_type: String,
    /// Authorization code, raw.
    pub auth_code: String,
    /// Pre-authorized amount (8 digits, cents), raw.
    pub pre_authorized_amount: String,
    /// Pre-auth code (9 digits), raw.
    pub pre_auth_code: String,
    /// Action code, raw.
    pub action_code: String,
    /// Host date/time, raw.
    pub host_date_time: String,
    /// Error description, raw.
    pub error_description: String,
    /// Card type, raw.
    pub card_type: String,
    /// Acquirer id, raw.
    pub acquirer_id: String,
    /// STAN, raw.
    pub stan: String,
    /// Online id, raw.
    pub online_id: String,
}

/// Raw VAS response (`'K'`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VasResponse {
    /// `RESPID` parsed from XML (`"0"` = OK); empty if absent.
    pub response_id: String,
    /// `RESPMSG`.
    pub response_message: String,
    /// `ORDER_ID`.
    pub order_id: String,
    /// Concatenation flag (`"1"` → more messages follow).
    pub more_messages: bool,
    /// 3-digit sequence id.
    pub id_message: String,
    /// The XML body of this message.
    pub raw_xml: String,
}

/// Parses a payment-family response (`'E'` plain / `'V'` DCC).
#[must_use]
pub fn parse_payment(p: &str) -> PaymentResponse {
    let mut r = PaymentResponse::default();
    let dcc = at(p, 10, 1) == "V"; // message code 'E' (plain) or 'V' (DCC)

    r.result_code = at(p, 11, 2).to_string();
    r.outcome = outcome_from_code(&r.result_code);

    if r.outcome == TransactionOutcome::Ko {
        r.error_description = trim_right(at(p, 13, 24));
    } else {
        r.pan = at(p, 13, 19).to_string();
        r.transaction_type = trim_right(at(p, 32, 3));
        r.auth_code = trim_right(at(p, 35, 6));
        r.host_date_time = at(p, 41, 7).to_string();
    }

    // Common to any response.
    r.card_type = at(p, 48, 1).to_string();
    r.acquirer_id = trim_right(at(p, 49, 11));
    r.stan = at(p, 60, 6).to_string();
    r.online_id = at(p, 66, 6).to_string();

    if dcc {
        r.currency.applied = at(p, 83, 1) == "1";
        r.currency.rate = at(p, 84, 8).to_string();
        r.currency.currency_code = trim_right(at(p, 92, 3));
        r.currency.amount = at(p, 95, 12).to_string();
        r.currency.precision = at(p, 107, 1).to_string();
    }
    r
}

/// Parses a status response (`'s'`).
#[must_use]
pub fn parse_status(p: &str) -> StatusResponse {
    let s = at(p, 31, 1);
    let status = s
        .bytes()
        .next()
        .filter(u8::is_ascii_digit)
        .map_or(-1, |b| i32::from(b - b'0'));
    StatusResponse {
        terminal_id: at(p, 1, 8).to_string(),
        date_time_raw: at(p, 21, 10).to_string(),
        status,
        software_release: trim_right(at(p, 32, p.len())),
    }
}

/// Parses a totals response (`'T'`).
#[must_use]
pub fn parse_totals(p: &str) -> TotalsResponse {
    let result_code = at(p, 11, 2).to_string();
    TotalsResponse {
        outcome: outcome_from_code(&result_code),
        result_code,
        pos_total: at(p, 13, 16).to_string(),
    }
}

/// Parses a close-session response (`'C'`).
#[must_use]
pub fn parse_close(p: &str) -> CloseResponse {
    let mut r = CloseResponse {
        result_code: at(p, 11, 2).to_string(),
        ..Default::default()
    };
    r.outcome = outcome_from_code(&r.result_code);
    if r.outcome == TransactionOutcome::Ok {
        r.pos_total = at(p, 13, 16).to_string();
        r.host_total = at(p, 29, 16).to_string();
    } else {
        r.error_description = trim_right(at(p, 13, 19));
        r.action_code = at(p, 32, 3).to_string();
    }
    r
}

/// Parses a pre-auth response (`'e'`).
#[must_use]
pub fn parse_pre_auth(p: &str) -> PreAuthResponse {
    let mut r = PreAuthResponse {
        result_code: at(p, 11, 2).to_string(),
        ..Default::default()
    };
    r.outcome = outcome_from_code(&r.result_code);
    if r.outcome == TransactionOutcome::Ko {
        r.error_description = trim_right(at(p, 13, 24));
        r.action_code = at(p, 37, 3).to_string();
    } else {
        r.pan = at(p, 13, 19).to_string();
        r.transaction_type = trim_right(at(p, 32, 3));
        r.auth_code = trim_right(at(p, 35, 6));
        r.pre_authorized_amount = at(p, 41, 8).to_string();
        r.pre_auth_code = at(p, 49, 9).to_string();
        r.action_code = at(p, 58, 3).to_string();
        r.host_date_time = at(p, 61, 7).to_string();
    }
    // In the OK layout pre_authorized_amount occupies positions 41-48, so position 48 is
    // the amount's last digit, NOT a card type. Only read card_type for the KO layout.
    if r.outcome == TransactionOutcome::Ko {
        r.card_type = at(p, 48, 1).to_string();
    }
    r.acquirer_id = trim_right(at(p, 72, 11));
    r.stan = at(p, 83, 6).to_string();
    r.online_id = at(p, 89, 6).to_string();
    r
}

/// Parses a VAS response (`'K'`).
#[must_use]
pub fn parse_vas(p: &str) -> VasResponse {
    let raw_xml = at(p, 27, p.len()).to_string();
    VasResponse {
        response_id: xml_value(&raw_xml, "RESPID"),
        response_message: xml_value(&raw_xml, "RESPMSG"),
        order_id: xml_value(&raw_xml, "ORDER_ID"),
        more_messages: at(p, 15, 1) == "1",
        id_message: at(p, 16, 3).to_string(),
        raw_xml,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Left-justified field, right-padded with spaces to `width` (alpha fields).
    fn a(value: &str, width: usize) -> String {
        let mut s = value.to_string();
        s.push_str(&" ".repeat(width.saturating_sub(value.len())));
        s
    }

    // Right-justified numeric field, left-padded with '0' to `width`.
    fn n(value: &str, width: usize) -> String {
        format!("{}{}", "0".repeat(width - value.len()), value)
    }

    #[test]
    fn payment_positive() {
        let p = format!(
            "{}0E00{}{}{}2111520{}{}{}{}",
            a("12345678", 8),
            n("4111111111", 19),
            a("ICC", 3),
            a("ABC123", 6),
            "2",
            a("ACQ", 11),
            n("42", 6),
            n("99", 6)
        );
        let r = parse_payment(&p);
        assert_eq!(r.outcome, TransactionOutcome::Ok);
        assert_eq!(r.result_code, "00");
        assert_eq!(r.pan, n("4111111111", 19));
        assert_eq!(r.transaction_type, "ICC");
        assert_eq!(r.auth_code, "ABC123");
        assert_eq!(r.host_date_time, "2111520");
        assert_eq!(r.card_type, "2");
        assert_eq!(r.acquirer_id, "ACQ");
        assert_eq!(r.stan, "000042");
        assert_eq!(r.online_id, "000099");
        assert!(!r.currency.applied);
    }

    #[test]
    fn payment_negative() {
        let p = format!(
            "{}0E01{}{}3{}{}{}",
            a("12345678", 8),
            a("CARTA RIFIUTATA", 24),
            n("", 11), // reserved 37-47
            a("AC2", 11),
            n("7", 6),
            n("3", 6)
        );
        let r = parse_payment(&p);
        assert_eq!(r.outcome, TransactionOutcome::Ko);
        assert_eq!(r.result_code, "01");
        assert_eq!(r.error_description, "CARTA RIFIUTATA");
        assert_eq!(r.card_type, "3");
        assert_eq!(r.stan, "000007");
    }

    #[test]
    fn payment_with_currency_exchange() {
        let base = format!(
            "{}0V00{}{}{}2111520{}{}{}{}",
            a("12345678", 8),
            n("4111111111", 19),
            a("ICC", 3),
            a("ABC123", 6),
            "2",
            a("ACQ", 11),
            n("42", 6),
            n("99", 6)
        );
        // actionCode(3) origAmount(8) flag(1) rate(8) ccy(3) amount(12) precision(1)
        let p = format!(
            "{base}000{}1{}USD{}2",
            n("650", 8),
            n("12345", 8),
            n("650", 12)
        );
        let r = parse_payment(&p);
        assert_eq!(r.outcome, TransactionOutcome::Ok);
        assert!(r.currency.applied);
        assert_eq!(r.currency.rate, "00012345");
        assert_eq!(r.currency.currency_code, "USD");
        assert_eq!(r.currency.amount, "000000000650");
        assert_eq!(r.currency.precision, "2");
    }

    #[test]
    fn status() {
        let p = format!("{}0s{}0102251530{}V1.2.3", a("12345678", 8), n("", 10), "2");
        let r = parse_status(&p);
        assert_eq!(r.terminal_id, "12345678");
        assert_eq!(r.date_time_raw, "0102251530");
        assert_eq!(r.status, 2);
        assert_eq!(r.software_release, "V1.2.3");
    }

    #[test]
    fn totals() {
        let p = format!("{}0T00{}{}", a("12345678", 8), n("123456", 16), n("", 6));
        let r = parse_totals(&p);
        assert_eq!(r.outcome, TransactionOutcome::Ok);
        assert_eq!(r.pos_total, n("123456", 16));
    }

    #[test]
    fn close_positive() {
        let p = format!("{}0C00{}{}", a("12345678", 8), n("1000", 16), n("1000", 16));
        let r = parse_close(&p);
        assert_eq!(r.outcome, TransactionOutcome::Ok);
        assert_eq!(r.pos_total, n("1000", 16));
        assert_eq!(r.host_total, n("1000", 16));
    }

    #[test]
    fn close_negative() {
        let p = format!("{}0C01{}100", a("12345678", 8), a("SBILANCIO", 19));
        let r = parse_close(&p);
        assert_eq!(r.outcome, TransactionOutcome::Ko);
        assert_eq!(r.error_description, "SBILANCIO");
        assert_eq!(r.action_code, "100");
    }

    #[test]
    fn pre_auth_positive() {
        let p = format!(
            "{}0e00{}{}{}{}{}000{}",
            a("12345678", 8),
            n("4111111111", 19),
            a("CLI", 3),
            a("AUTH01", 6),
            n("50000", 8),
            n("123", 9),
            "2111520"
        );
        let r = parse_pre_auth(&p);
        assert_eq!(r.outcome, TransactionOutcome::Ok);
        assert_eq!(r.transaction_type, "CLI");
        assert_eq!(r.auth_code, "AUTH01");
        assert_eq!(r.pre_authorized_amount, "00050000");
        assert_eq!(r.pre_auth_code, "000000123");
        assert_eq!(r.host_date_time, "2111520");
    }

    // Regression: on an approved pre-auth the amount field occupies positions 41-48, so
    // its last digit sits exactly where card_type would be read. An amount ending in
    // 1/2/3 must NOT be surfaced as debit/credit/other.
    #[test]
    fn pre_auth_positive_does_not_leak_amount_digit_as_card_type() {
        let p = format!(
            "{}0e00{}{}{}{}{}000{}",
            a("12345678", 8),
            n("4111111111", 19),
            a("CLI", 3),
            a("AUTH01", 6),
            n("50001", 8),
            n("123", 9),
            "2111520"
        );
        let r = parse_pre_auth(&p);
        assert_eq!(r.outcome, TransactionOutcome::Ok);
        assert_eq!(r.pre_authorized_amount, "00050001"); // ends in '1'
        assert_eq!(r.card_type, ""); // must stay empty, not "1"
    }

    #[test]
    fn vas() {
        let xml = "<ecrres><p k=\"RESPID\">0</p><p k=\"RESPMSG\">OK-APPROVED</p>\
                   <p k=\"ORDER_ID\">ABC123</p></ecrres>";
        // header(10) reserved(4) concatFlag(1) idMessage(3) filler-to-pos27(8) xml
        let p = format!("{}0K{}0001{}{}", a("12345678", 8), n("", 4), n("", 8), xml);
        let r = parse_vas(&p);
        assert!(!r.more_messages);
        assert_eq!(r.id_message, "001");
        assert_eq!(r.response_id, "0");
        assert_eq!(r.response_message, "OK-APPROVED");
        assert_eq!(r.order_id, "ABC123");
        assert_eq!(r.raw_xml, xml);
    }

    #[test]
    fn defensive_on_short_or_empty_payload() {
        let r = parse_payment("");
        assert_eq!(r.outcome, TransactionOutcome::Unknown);
        assert_eq!(r.result_code, "");
        assert_eq!(r.pan, "");

        let s = parse_status("123"); // truncated, must not panic
        assert_eq!(s.status, -1);
    }

    #[test]
    fn outcome_mapping() {
        assert_eq!(outcome_from_code("00"), TransactionOutcome::Ok);
        assert_eq!(outcome_from_code("01"), TransactionOutcome::Ko);
        assert_eq!(outcome_from_code("05"), TransactionOutcome::CardNotPresent);
        assert_eq!(outcome_from_code("09"), TransactionOutcome::UnknownTag);
        assert_eq!(outcome_from_code("zz"), TransactionOutcome::Unknown);
    }
}
