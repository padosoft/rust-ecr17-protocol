//! ECR17 application-message builders — the bytes that go between `STX` and `ETX`.
//!
//! Every field is fixed-width and validated: a value that overflows its field returns
//! [`Ecr17Error::FieldOverflow`] so a malformed frame is never produced. Ported from the
//! reference C++ `Ecr17Protocol`.
//!
//! `payment_type` is the single request digit: `'0'` auto, `'1'` debit, `'2'` credit,
//! `'3'` other (see [`crate::types::PaymentCardType`]).

use crate::error::{Ecr17Error, Result};

/// `'0'` (0x30) filler for reserved numeric fields.
const RESERVED: char = '0';
/// End-of-field marker for the privative TAG content (0x1B).
const FIELD_SEP: char = '\u{1B}';

/// Right-aligns `value` into a fixed-width field, padding on the left with `ch`.
fn left_pad(value: &str, size: usize, ch: char) -> Result<String> {
    if value.len() > size {
        return Err(Ecr17Error::FieldOverflow {
            value: value.to_string(),
            width: size,
        });
    }
    let mut s = String::with_capacity(size);
    for _ in 0..size - value.len() {
        s.push(ch);
    }
    s.push_str(value);
    Ok(s)
}

/// Left-aligns `value` into a fixed-width field, padding on the right with `ch`.
fn right_pad(value: &str, size: usize, ch: char) -> Result<String> {
    if value.len() > size {
        return Err(Ecr17Error::FieldOverflow {
            value: value.to_string(),
            width: size,
        });
    }
    let mut s = String::with_capacity(size);
    s.push_str(value);
    for _ in 0..size - value.len() {
        s.push(ch);
    }
    Ok(s)
}

/// The 8-byte, right-aligned, zero-filled amount field.
fn amount_field(amount_cents: i64) -> Result<String> {
    if amount_cents < 0 {
        return Err(Ecr17Error::NegativeAmount);
    }
    left_pad(&amount_cents.to_string(), 8, RESERVED)
}

fn flag(on: bool) -> char {
    if on {
        '1'
    } else {
        '0'
    }
}

/// Rejects a payment-type digit outside `'0'..'3'` so a malformed frame is never produced.
/// The normal path supplies this via [`crate::types::PaymentCardType::as_digit`].
fn validate_payment_type(payment_type: char) -> Result<()> {
    if matches!(payment_type, '0'..='3') {
        Ok(())
    } else {
        Err(Ecr17Error::InvalidPaymentType {
            value: payment_type,
        })
    }
}

// Shared 167-byte payment-family layout (codes 'P', 'X', 'p').
#[allow(clippy::too_many_arguments)]
fn build_payment_like(
    code: char,
    terminal_id: &str,
    cash_register_id: &str,
    amount_cents: i64,
    payment_type: char,
    card_already_present: bool,
    with_additional_data: bool,
    receipt_text: &str,
) -> Result<String> {
    validate_payment_type(payment_type)?;
    let mut m = String::with_capacity(167);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push(code); // 10 : message code
    m.push_str(&left_pad(cash_register_id, 8, RESERVED)?); // 11 : cash register id
    m.push(flag(with_additional_data)); // 19 : presence of additional GT data
    m.push_str("00"); // 20 : reserved
    m.push(flag(card_already_present)); // 22 : start-with-card-present
    m.push(payment_type); // 23 : payment type
    m.push_str(&amount_field(amount_cents)?); // 24 : amount (8)
                                              // 32 : receipt text (128) — RIGHT-aligned (leading spaces), per the Nexi reference
                                              // (`buildPaymentLike` uses leftPad here; the layout test asserts the text at the tail).
                                              // Do NOT switch to right_pad — that would misalign the field vs the terminal.
    m.push_str(&left_pad(receipt_text, 128, ' ')?);
    m.push_str("00000000"); // 160: reserved (8)
    Ok(m) // 167
}

// Shared 176-byte pre-auth integration/closure layout (codes 'i', 'c').
fn build_pre_auth_follow_up(
    code: char,
    terminal_id: &str,
    cash_register_id: &str,
    amount_cents: i64,
    original_pre_auth_code: &str,
    with_additional_data: bool,
    receipt_text: &str,
) -> Result<String> {
    let mut m = String::with_capacity(176);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push(code); // 10 : message code
    m.push_str(&left_pad(cash_register_id, 8, RESERVED)?); // 11 : cash register id
    m.push(flag(with_additional_data)); // 19 : presence of additional GT data
    m.push_str("0000"); // 20 : reserved (4)
    m.push_str(&amount_field(amount_cents)?); // 24 : amount (8)
    m.push_str(&left_pad(receipt_text, 128, ' ')?); // 32 : receipt text (128, right-aligned; see build_payment_like)
    m.push_str(&left_pad(original_pre_auth_code, 9, RESERVED)?); // 160: original pre-auth code (9)
    m.push_str("00000000"); // 169: reserved (8)
    Ok(m) // 176
}

// Shared 26-byte session command layout (codes 'C', 'T').
fn build_session_command(
    code: char,
    terminal_id: &str,
    cash_register_id: &str,
    with_additional_data: bool,
) -> Result<String> {
    let mut m = String::with_capacity(26);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push(code); // 10 : message code
    m.push_str(&left_pad(cash_register_id, 8, RESERVED)?); // 11 : cash register id
    m.push(flag(with_additional_data)); // 19 : presence of additional GT data
    m.push_str("0000000"); // 20 : reserved (7)
    Ok(m) // 26
}

/// Payment `'P'` (167 bytes).
#[allow(clippy::too_many_arguments)]
pub fn build_payment(
    terminal_id: &str,
    cash_register_id: &str,
    amount_cents: i64,
    payment_type: char,
    card_already_present: bool,
    with_additional_data: bool,
    receipt_text: &str,
) -> Result<String> {
    build_payment_like(
        'P',
        terminal_id,
        cash_register_id,
        amount_cents,
        payment_type,
        card_already_present,
        with_additional_data,
        receipt_text,
    )
}

/// Extended payment `'X'` (167 bytes).
#[allow(clippy::too_many_arguments)]
pub fn build_extended_payment(
    terminal_id: &str,
    cash_register_id: &str,
    amount_cents: i64,
    payment_type: char,
    card_already_present: bool,
    with_additional_data: bool,
    receipt_text: &str,
) -> Result<String> {
    build_payment_like(
        'X',
        terminal_id,
        cash_register_id,
        amount_cents,
        payment_type,
        card_already_present,
        with_additional_data,
        receipt_text,
    )
}

/// Pre-auth `'p'` (167 bytes).
#[allow(clippy::too_many_arguments)]
pub fn build_pre_auth(
    terminal_id: &str,
    cash_register_id: &str,
    amount_cents: i64,
    payment_type: char,
    card_already_present: bool,
    with_additional_data: bool,
    receipt_text: &str,
) -> Result<String> {
    build_payment_like(
        'p',
        terminal_id,
        cash_register_id,
        amount_cents,
        payment_type,
        card_already_present,
        with_additional_data,
        receipt_text,
    )
}

/// Incremental pre-auth `'i'` (176 bytes).
pub fn build_incremental(
    terminal_id: &str,
    cash_register_id: &str,
    amount_cents: i64,
    original_pre_auth_code: &str,
    with_additional_data: bool,
    receipt_text: &str,
) -> Result<String> {
    build_pre_auth_follow_up(
        'i',
        terminal_id,
        cash_register_id,
        amount_cents,
        original_pre_auth_code,
        with_additional_data,
        receipt_text,
    )
}

/// Pre-auth closure `'c'` (176 bytes).
pub fn build_pre_auth_closure(
    terminal_id: &str,
    cash_register_id: &str,
    amount_cents: i64,
    original_pre_auth_code: &str,
    with_additional_data: bool,
    receipt_text: &str,
) -> Result<String> {
    build_pre_auth_follow_up(
        'c',
        terminal_id,
        cash_register_id,
        amount_cents,
        original_pre_auth_code,
        with_additional_data,
        receipt_text,
    )
}

/// Card verification `'H'` (39 bytes).
pub fn build_card_verification(
    terminal_id: &str,
    cash_register_id: &str,
    payment_type: char,
    with_additional_data: bool,
) -> Result<String> {
    validate_payment_type(payment_type)?;
    let mut m = String::with_capacity(39);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push('H'); // 10 : message code
    m.push_str(&left_pad(cash_register_id, 8, RESERVED)?); // 11 : cash register id
    m.push(flag(with_additional_data)); // 19 : presence of additional GT data
    m.push_str("00"); // 20 : reserved (2)
    m.push('0'); // 22 : standard card verification
    m.push(payment_type); // 23 : payment type
    m.push_str("0000000000000000"); // 24 : reserved (16)
    Ok(m) // 39
}

/// Close session `'C'` (26 bytes).
pub fn build_close_session(
    terminal_id: &str,
    cash_register_id: &str,
    with_additional_data: bool,
) -> Result<String> {
    build_session_command('C', terminal_id, cash_register_id, with_additional_data)
}

/// Totals `'T'` (26 bytes).
pub fn build_totals(
    terminal_id: &str,
    cash_register_id: &str,
    with_additional_data: bool,
) -> Result<String> {
    build_session_command('T', terminal_id, cash_register_id, with_additional_data)
}

/// Send last result `'G'` (22 bytes).
pub fn build_send_last_result(
    terminal_id: &str,
    cash_register_id: &str,
    with_additional_data: bool,
) -> Result<String> {
    let mut m = String::with_capacity(22);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push('G'); // 10 : message code
    m.push_str(&left_pad(cash_register_id, 8, RESERVED)?); // 11 : cash register id
    m.push(flag(with_additional_data)); // 19 : presence of additional GT data
    m.push_str("000"); // 20 : reserved (3)
    Ok(m) // 22
}

/// Enable/disable ECR printing `'E'` (11 bytes).
pub fn build_enable_ecr_print(terminal_id: &str, enabled: bool) -> Result<String> {
    let mut m = String::with_capacity(11);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push('E'); // 10 : message code
    m.push(flag(enabled)); // 11 : enable(1)/disable(0) printing on ECR
    Ok(m) // 11
}

/// Reprint ticket `'R'` (22 bytes).
pub fn build_reprint(terminal_id: &str, to_ecr: bool, ticket_type: char) -> Result<String> {
    let mut m = String::with_capacity(22);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push('R'); // 10 : message code
    m.push(flag(to_ecr)); // 11 : 1 = send receipt to ECR, 0 = print on terminal
    m.push(ticket_type); // 12 : ticket type flag
    m.push_str("0000000000"); // 13 : reserved (10)
    Ok(m) // 22
}

/// Status `'s'` (10 bytes; lowercase code per spec).
pub fn build_status(terminal_id: &str) -> Result<String> {
    let mut m = String::with_capacity(10);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push('s'); // 10 : message code (lowercase per spec)
    Ok(m) // 10
}

/// Reversal `'S'` (26 bytes); `stan = "000000"` reverses the last payment with no STAN check.
pub fn build_reversal(terminal_id: &str, cash_register_id: &str, stan: &str) -> Result<String> {
    let mut m = String::with_capacity(26);
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push('S'); // 10 : message code
    m.push_str(&left_pad(cash_register_id, 8, RESERVED)?); // 11 : cash register id
    m.push_str(&left_pad(stan, 6, RESERVED)?); // 19 : STAN ("000000" = no check)
    m.push(RESERVED); // 25 : presence of additional GT data
    m.push(RESERVED); // 26 : reserved
    Ok(m) // 26
}

/// VAS `'K'` (variable, length-prefixed XML, max 1024 bytes).
pub fn build_vas(terminal_id: &str, ecr_id: &str, xml_request: &str) -> Result<String> {
    if xml_request.len() > 1024 {
        return Err(Ecr17Error::VasTooLong);
    }
    let mut m = String::new();
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push('K'); // 10 : message code
    m.push_str(&left_pad(ecr_id, 8, RESERVED)?); // 11 : ECR identifier
    m.push_str("000"); // 19 : reserved (3)
    m.push(RESERVED); // 22 : reserved (1)
    m.push_str(&left_pad(&xml_request.len().to_string(), 4, RESERVED)?); // 23 : VAS length (4)
    m.push_str(xml_request); // 27 : VAS request (XML)
    Ok(m)
}

/// Additional data for GT / tokenization `'U'` (variable).
///
/// `tag_content` is the privative TAG content (1..=100 chars), terminated with `0x1B`
/// by this builder. Use [`format_tokenization_tag`] to produce it. `iso_field` and
/// `tag_number` default (in the reference) to `"62"` and `"DF8D01"`.
pub fn build_additional_tags(
    terminal_id: &str,
    tag_content: &str,
    iso_field: &str,
    tag_number: &str,
) -> Result<String> {
    if tag_content.is_empty() || tag_content.len() > 100 {
        return Err(Ecr17Error::TagContentInvalid);
    }
    let mut m = String::new();
    m.push_str(&left_pad(terminal_id, 8, RESERVED)?); // 1  : terminal id
    m.push(RESERVED); // 9  : reserved
    m.push('U'); // 10 : message code
    m.push_str("000000"); // 11 : payment type (6) -> standard payment
    m.push_str(&left_pad(iso_field, 2, RESERVED)?); // 17 : ISO field number (e.g. "62")
    m.push_str(&right_pad(tag_number, 8, ' ')?); // 19 : TAG number, left-justified, blank-filled
    m.push(RESERVED); // 27 : reserved (1)
    m.push_str("0000"); // 28 : exclusive TAG index bytemap (none to send to GT)
    m.push_str("00000"); // 32 : reserved (5)
    m.push_str(tag_content); // 37 : privative TAG content (1..=100)
    m.push(FIELD_SEP); //      end-of-field (0x1B)
    Ok(m)
}

/// Formats the TAG 5 content for tokenization (Intesa-style mapping):
///   `"0COF0TRK<contract>|0FNZ03"` (unscheduled/one-click) or
///   `"0REC0TRK<contract>|0FNZ03"` (recurring). `recurring` selects `0REC` vs `0COF`.
pub fn format_tokenization_tag(recurring: bool, contract_code: &str) -> Result<String> {
    if contract_code.is_empty() || contract_code.len() > 18 {
        return Err(Ecr17Error::ContractCodeInvalid);
    }
    let service = if recurring { "0REC" } else { "0COF" };
    Ok(format!("{service}0TRK{contract_code}|0FNZ03"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: &str = "12345678"; // terminal id
    const C: &str = "87654321"; // cash register id

    // --- test_protocol.cpp -------------------------------------------------

    #[test]
    fn status_message_layout() {
        let m = build_status("42").unwrap();
        assert_eq!(m.len(), 10);
        assert_eq!(&m[0..8], "00000042"); // terminal id, left-padded
        assert_eq!(m.as_bytes()[8], b'0'); // reserved
        assert_eq!(m.as_bytes()[9], b's'); // lowercase message code per spec
    }

    #[test]
    fn status_message_keeps_full_width_id() {
        assert_eq!(build_status("12345678").unwrap(), "123456780s");
    }

    #[test]
    fn payment_message_is_167_bytes() {
        assert_eq!(
            build_payment("1", "2", 650, '0', false, false, "")
                .unwrap()
                .len(),
            167
        );
    }

    #[test]
    fn payment_message_field_layout() {
        let m = build_payment("12345678", "87654321", 650, '0', false, false, "").unwrap();
        assert_eq!(m.len(), 167);
        assert_eq!(&m[0..8], "12345678"); // terminal id
        assert_eq!(m.as_bytes()[8], b'0'); // reserved
        assert_eq!(m.as_bytes()[9], b'P'); // message code
        assert_eq!(&m[10..18], "87654321"); // cash register id
        assert_eq!(m.as_bytes()[18], b'0'); // presence of additional data
        assert_eq!(&m[19..21], "00"); // reserved
        assert_eq!(m.as_bytes()[21], b'0'); // start-with-card
        assert_eq!(m.as_bytes()[22], b'0'); // payment type
        assert_eq!(&m[23..31], "00000650"); // amount, right aligned
        assert_eq!(&m[31..159], &" ".repeat(128)); // text field
        assert_eq!(&m[159..167], "00000000"); // trailing reserved
    }

    #[test]
    fn payment_message_amount_max_fits() {
        let m = build_payment("1", "2", 99_999_999, '0', false, false, "").unwrap();
        assert_eq!(&m[23..31], "99999999");
    }

    #[test]
    fn payment_rejects_negative_amount() {
        assert_eq!(
            build_payment("1", "2", -1, '0', false, false, ""),
            Err(Ecr17Error::NegativeAmount)
        );
    }

    #[test]
    fn payment_rejects_amount_overflowing_field() {
        // 9 digits does not fit the 8-byte amount field.
        assert!(matches!(
            build_payment("1", "2", 100_000_000, '0', false, false, ""),
            Err(Ecr17Error::FieldOverflow { .. })
        ));
    }

    #[test]
    fn payment_rejects_oversized_terminal_id() {
        assert!(matches!(
            build_payment("123456789", "2", 650, '0', false, false, ""),
            Err(Ecr17Error::FieldOverflow { .. })
        ));
    }

    #[test]
    fn status_rejects_oversized_terminal_id() {
        assert!(matches!(
            build_status("123456789"),
            Err(Ecr17Error::FieldOverflow { .. })
        ));
    }

    #[test]
    fn reversal_message_layout() {
        let m = build_reversal("12345678", "87654321", "000123").unwrap();
        assert_eq!(m.len(), 26);
        assert_eq!(&m[0..8], "12345678");
        assert_eq!(m.as_bytes()[8], b'0');
        assert_eq!(m.as_bytes()[9], b'S');
        assert_eq!(&m[10..18], "87654321");
        assert_eq!(&m[18..24], "000123");
        assert_eq!(m.as_bytes()[24], b'0');
        assert_eq!(m.as_bytes()[25], b'0');
    }

    #[test]
    fn reversal_default_stan_is_no_check() {
        let m = build_reversal("12345678", "87654321", "000000").unwrap();
        assert_eq!(&m[18..24], "000000");
    }

    #[test]
    fn reversal_rejects_oversized_stan() {
        assert!(matches!(
            build_reversal("1", "2", "1234567"),
            Err(Ecr17Error::FieldOverflow { .. })
        ));
    }

    // --- test_protocol_commands.cpp ---------------------------------------

    #[test]
    fn extended_payment_layout_and_flags() {
        let m = build_extended_payment(T, C, 650, '2', true, true, "ABC").unwrap();
        assert_eq!(m.len(), 167);
        assert_eq!(&m[0..8], T);
        assert_eq!(m.as_bytes()[8], b'0');
        assert_eq!(m.as_bytes()[9], b'X');
        assert_eq!(&m[10..18], C);
        assert_eq!(m.as_bytes()[18], b'1'); // withAdditionalData
        assert_eq!(&m[19..21], "00");
        assert_eq!(m.as_bytes()[21], b'1'); // cardAlreadyPresent
        assert_eq!(m.as_bytes()[22], b'2'); // payment type
        assert_eq!(&m[23..31], "00000650"); // amount
        assert_eq!(&m[31..156], &" ".repeat(125)); // text left-padding
        assert_eq!(&m[156..159], "ABC"); // text right-aligned
        assert_eq!(&m[159..167], "00000000");
    }

    // Locks the intentional RIGHT-alignment of the 128-byte receipt-text field (leading
    // spaces, text at the tail) — matches the Nexi reference; do not "fix" to left-align.
    #[test]
    fn payment_receipt_text_is_right_aligned() {
        let m = build_payment(T, C, 650, '0', false, false, "ABC").unwrap();
        assert_eq!(&m[31..156], &" ".repeat(125)); // 125 leading spaces
        assert_eq!(&m[156..159], "ABC"); // text right-aligned at the tail
    }

    #[test]
    fn pre_auth_uses_code_lower_p() {
        let m = build_pre_auth(T, C, 1000, '0', false, false, "").unwrap();
        assert_eq!(m.len(), 167);
        assert_eq!(m.as_bytes()[9], b'p');
        assert_eq!(&m[23..31], "00001000");
    }

    #[test]
    fn payment_defaults_match_basic_layout() {
        let m = build_payment(T, C, 650, '0', false, false, "").unwrap();
        assert_eq!(m.len(), 167);
        assert_eq!(m.as_bytes()[9], b'P');
        assert_eq!(m.as_bytes()[18], b'0'); // no additional data by default
        assert_eq!(m.as_bytes()[21], b'0'); // card not present by default
        assert_eq!(m.as_bytes()[22], b'0'); // auto payment type by default
        assert_eq!(&m[31..159], &" ".repeat(128));
    }

    #[test]
    fn incremental_layout() {
        let m = build_incremental(T, C, 1000, "123456789", false, "").unwrap();
        assert_eq!(m.len(), 176);
        assert_eq!(m.as_bytes()[9], b'i');
        assert_eq!(&m[19..23], "0000");
        assert_eq!(&m[23..31], "00001000");
        assert_eq!(&m[159..168], "123456789"); // original pre-auth code
        assert_eq!(&m[168..176], "00000000");
    }

    #[test]
    fn pre_auth_closure_layout() {
        let m = build_pre_auth_closure(T, C, 500, "000000042", false, "").unwrap();
        assert_eq!(m.len(), 176);
        assert_eq!(m.as_bytes()[9], b'c');
        assert_eq!(&m[159..168], "000000042");
    }

    #[test]
    fn card_verification_layout() {
        let m = build_card_verification(T, C, '1', false).unwrap();
        assert_eq!(m.len(), 39);
        assert_eq!(m.as_bytes()[9], b'H');
        assert_eq!(&m[10..18], C);
        assert_eq!(m.as_bytes()[18], b'0'); // no additional data
        assert_eq!(&m[19..21], "00");
        assert_eq!(m.as_bytes()[21], b'0'); // standard verification
        assert_eq!(m.as_bytes()[22], b'1'); // payment type
        assert_eq!(&m[23..39], &"0".repeat(16));
    }

    #[test]
    fn close_session_layout() {
        let m = build_close_session(T, C, false).unwrap();
        assert_eq!(m.len(), 26);
        assert_eq!(m.as_bytes()[9], b'C');
        assert_eq!(&m[10..18], C);
        assert_eq!(m.as_bytes()[18], b'0');
        assert_eq!(&m[19..26], &"0".repeat(7));
    }

    #[test]
    fn totals_layout() {
        let m = build_totals(T, C, false).unwrap();
        assert_eq!(m.len(), 26);
        assert_eq!(m.as_bytes()[9], b'T');
    }

    #[test]
    fn send_last_result_layout() {
        let m = build_send_last_result(T, C, false).unwrap();
        assert_eq!(m.len(), 22);
        assert_eq!(m.as_bytes()[9], b'G');
        assert_eq!(&m[19..22], "000");
    }

    #[test]
    fn enable_ecr_print_layout() {
        assert_eq!(build_enable_ecr_print(T, true).unwrap(), "123456780E1");
        assert_eq!(build_enable_ecr_print(T, false).unwrap(), "123456780E0");
    }

    #[test]
    fn reprint_layout() {
        let m = build_reprint(T, true, '0').unwrap();
        assert_eq!(m.len(), 22);
        assert_eq!(m.as_bytes()[9], b'R');
        assert_eq!(m.as_bytes()[10], b'1'); // send to ECR
        assert_eq!(m.as_bytes()[11], b'0'); // ticket type default
        assert_eq!(&m[12..22], &"0".repeat(10));
    }

    #[test]
    fn vas_layout_and_length_prefix() {
        let m = build_vas(T, C, "<x/>").unwrap();
        assert_eq!(m.len(), 30);
        assert_eq!(m.as_bytes()[9], b'K');
        assert_eq!(&m[10..18], C);
        assert_eq!(&m[18..21], "000");
        assert_eq!(m.as_bytes()[21], b'0');
        assert_eq!(&m[22..26], "0004"); // length of "<x/>"
        assert_eq!(&m[26..], "<x/>");
    }

    #[test]
    fn vas_rejects_oversized_request() {
        assert_eq!(
            build_vas(T, C, &"x".repeat(1025)),
            Err(Ecr17Error::VasTooLong)
        );
    }

    #[test]
    fn additional_tags_layout() {
        let content = "0COF0TRK123|0FNZ03"; // 18 chars
        let m = build_additional_tags(T, content, "62", "DF8D01").unwrap();
        assert_eq!(m.len(), 36 + content.len() + 1);
        assert_eq!(m.as_bytes()[9], b'U');
        assert_eq!(&m[10..16], "000000");
        assert_eq!(&m[16..18], "62");
        assert_eq!(&m[18..26], "DF8D01  "); // left-justified, blank-filled
        assert_eq!(m.as_bytes()[26], b'0');
        assert_eq!(&m[27..31], "0000");
        assert_eq!(&m[31..36], "00000");
        assert_eq!(&m[36..36 + content.len()], content);
        assert_eq!(m.as_bytes()[m.len() - 1], 0x1B);
    }

    #[test]
    fn additional_tags_rejects_bad_content() {
        assert_eq!(
            build_additional_tags(T, "", "62", "DF8D01"),
            Err(Ecr17Error::TagContentInvalid)
        );
        assert_eq!(
            build_additional_tags(T, &"x".repeat(101), "62", "DF8D01"),
            Err(Ecr17Error::TagContentInvalid)
        );
    }

    #[test]
    fn tokenization_tag_format() {
        assert_eq!(
            format_tokenization_tag(false, "1666354841608").unwrap(),
            "0COF0TRK1666354841608|0FNZ03"
        );
        assert_eq!(
            format_tokenization_tag(true, "ABC").unwrap(),
            "0REC0TRKABC|0FNZ03"
        );
        assert_eq!(
            format_tokenization_tag(false, ""),
            Err(Ecr17Error::ContractCodeInvalid)
        );
        assert_eq!(
            format_tokenization_tag(false, &"x".repeat(19)),
            Err(Ecr17Error::ContractCodeInvalid)
        );
    }

    #[test]
    fn incremental_rejects_oversized_pre_auth_code() {
        assert!(matches!(
            build_incremental(T, C, 100, "1234567890", false, ""), // 10 digits > 9-byte field
            Err(Ecr17Error::FieldOverflow { .. })
        ));
    }

    #[test]
    fn builders_reject_invalid_payment_type() {
        assert_eq!(
            build_payment(T, C, 100, '9', false, false, ""),
            Err(Ecr17Error::InvalidPaymentType { value: '9' })
        );
        assert_eq!(
            build_card_verification(T, C, 'x', false),
            Err(Ecr17Error::InvalidPaymentType { value: 'x' })
        );
        // All valid digits are accepted.
        for d in ['0', '1', '2', '3'] {
            assert!(build_payment(T, C, 100, d, false, false, "").is_ok());
            assert!(build_card_verification(T, C, d, false).is_ok());
        }
    }

    #[test]
    fn pre_auth_rejects_negative_amount() {
        assert_eq!(
            build_pre_auth(T, C, -1, '0', false, false, ""),
            Err(Ecr17Error::NegativeAmount)
        );
    }
}
