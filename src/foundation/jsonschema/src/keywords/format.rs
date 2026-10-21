//! Validator for `format` keyword.
use crate::LazyInstance;
use std::{
    borrow::Cow,
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
    sync::Arc,
};

use email_address::{EmailAddress, Options as EmailAddressOptions};
use json_codec::{Map, Value};
use strum::VariantArray;
use unicode_general_category::{get_general_category, GeneralCategory};
use uuid_simd::{parse_hyphenated, Out};

use crate::{
    compiler,
    error::ValidationError,
    evaluation::Annotations,
    keywords::{rfc3986, CompilationResult},
    paths::{LazyLocation, Location, RefTracker},
    types::JsonType,
    validator::{EvaluationResult, Validate, ValidationContext},
    Draft, Json, Node,
};

/// RFC 6570 Level 4 URI Template validator.
///
/// Single-pass parser with no allocations, early exit on invalid input.
/// Supports all operators (+, #, ., /, ;, ?, &, =, ,, !, @, |) and modifiers (:prefix, *explode).
#[must_use]
pub fn is_valid_uri_template(template: &str) -> bool {
    let bytes = template.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'{' {
            // Parse expression
            i += 1;
            if i >= len {
                return false; // Unclosed brace
            }

            // Optional operator
            if is_operator(bytes[i]) {
                i += 1;
                if i >= len {
                    return false;
                }
            }

            // Parse variable list (at least one varspec required)
            if !parse_varspec(bytes, &mut i) {
                return false;
            }

            // Parse additional varspecs separated by commas
            while i < len && bytes[i] == b',' {
                i += 1;
                if !parse_varspec(bytes, &mut i) {
                    return false;
                }
            }

            // Expect closing brace
            if i >= len || bytes[i] != b'}' {
                return false;
            }
            i += 1;
        } else if bytes[i] == b'}' {
            // Unmatched closing brace
            return false;
        } else {
            // Parse literal
            if !parse_literal(bytes, &mut i) {
                return false;
            }
        }
    }

    true
}

/// Check if byte is an RFC 6570 operator (Appendix A grammar).
/// Level 2: + (reserved expansion), # (fragment)
/// Level 3: . (label), / (path), ; (path-style param), ? (query), & (query continuation)
/// Reserved: = , ! @ | (reserved for future extensions per Section 2.2)
#[inline]
fn is_operator(b: u8) -> bool {
    matches!(
        b,
        b'+' | b'#' | b'.' | b'/' | b';' | b'?' | b'&' | b'=' | b',' | b'!' | b'@' | b'|'
    )
}

/// Parse a variable specification: varname \[ modifier \]
/// Returns false if invalid, updates index on success.
#[inline]
fn parse_varspec(bytes: &[u8], i: &mut usize) -> bool {
    let len = bytes.len();

    // Parse varname (required)
    if !parse_varname(bytes, i) {
        return false;
    }

    // Optional modifier
    if *i < len {
        match bytes[*i] {
            b':' => {
                // Prefix modifier: ":" max-length (1-9999)
                *i += 1;
                if *i >= len {
                    return false;
                }
                // First digit must be 1-9
                if !bytes[*i].is_ascii_digit() || bytes[*i] == b'0' {
                    return false;
                }
                *i += 1;
                // Up to 3 more digits (total max 4 digits for 1-9999)
                let mut digit_count = 1;
                while *i < len && bytes[*i].is_ascii_digit() && digit_count < 4 {
                    *i += 1;
                    digit_count += 1;
                }
            }
            b'*' => {
                // Explode modifier
                *i += 1;
            }
            _ => {}
        }
    }

    true
}

/// Parse a variable name: `varchar *( ["."] varchar )`
/// varchar = ALPHA / DIGIT / "_" / pct-encoded
#[inline]
fn parse_varname(bytes: &[u8], i: &mut usize) -> bool {
    let len = bytes.len();

    // Must have at least one varchar
    if !parse_varchar(bytes, i) {
        return false;
    }

    // Continue parsing [ "." ] varchar
    while *i < len {
        if bytes[*i] == b'.' {
            *i += 1;
            if !parse_varchar(bytes, i) {
                return false;
            }
        } else if is_varchar_start(bytes[*i]) || bytes[*i] == b'%' {
            if !parse_varchar(bytes, i) {
                return false;
            }
        } else {
            break;
        }
    }

    true
}

/// Parse one or more varchar characters.
#[inline]
fn parse_varchar(bytes: &[u8], i: &mut usize) -> bool {
    let len = bytes.len();
    let start = *i;

    while *i < len {
        if is_varchar_start(bytes[*i]) {
            *i += 1;
        } else if bytes[*i] == b'%' {
            // pct-encoded
            if *i + 2 >= len {
                return false;
            }
            if !is_hex_digit(bytes[*i + 1]) || !is_hex_digit(bytes[*i + 2]) {
                return false;
            }
            *i += 3;
        } else {
            break;
        }
    }

    *i > start
}

/// Check if byte can start a varchar (ALPHA / DIGIT / "_").
#[inline]
fn is_varchar_start(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Check if byte is a hexadecimal digit.
#[inline]
fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

/// Parse a literal character or percent-encoded sequence.
/// Returns false if invalid character found.
#[inline]
fn parse_literal(bytes: &[u8], i: &mut usize) -> bool {
    let len = bytes.len();
    let b = bytes[*i];

    if b == b'%' {
        // pct-encoded
        if *i + 2 >= len {
            return false;
        }
        if !is_hex_digit(bytes[*i + 1]) || !is_hex_digit(bytes[*i + 2]) {
            return false;
        }
        *i += 3;
        true
    } else if is_literal_char(b) {
        *i += 1;
        true
    } else {
        false
    }
}

/// Check if byte is a valid literal character per RFC 6570 (with errata 6937, which
/// admits the apostrophe).
/// Excludes: CTL (0x00-0x1F, 0x7F), space (0x20), and: `" < > % \ ^ { | }`
#[inline]
fn is_literal_char(b: u8) -> bool {
    !matches!(
        b,
        0x00..=0x20 | b'"' | b'<' | b'>' | b'%' | b'\\' | b'^' | b'`' | b'{' | b'|' | b'}' | 0x7F
    )
}

#[must_use]
pub fn is_valid_json_pointer(pointer: &str) -> bool {
    if pointer.is_empty() {
        // An empty string is a valid JSON Pointer
        return true;
    }

    let mut chars = pointer.chars();

    // The first character must be a '/'
    if chars.next() != Some('/') {
        return false;
    }
    is_valid_json_pointer_impl(chars)
}

#[must_use]
pub fn is_valid_relative_json_pointer(s: &str) -> bool {
    let mut chars = s.chars();

    // Parse the non-negative integer part
    match chars.next() {
        Some('0') => {
            // If it starts with '0', it must be followed by '#' or '/'
            match chars.next() {
                Some('#') => chars.next().is_none(),
                Some('/') => is_valid_json_pointer_impl(chars),
                None => true,
                _ => false,
            }
        }
        Some(c) if c.is_ascii_digit() => {
            // Parse the rest of the integer
            while let Some(c) = chars.next() {
                match c {
                    '#' => return chars.next().is_none(),
                    '/' => return is_valid_json_pointer_impl(chars),
                    c if c.is_ascii_digit() => {}
                    _ => return false,
                }
            }
            // Valid if it's just a number
            true
        }
        _ => false,
    }
}

#[inline]
fn is_valid_json_pointer_impl<I: Iterator<Item = char>>(chars: I) -> bool {
    let mut escaped = false;
    for c in chars {
        match c {
            // '/' is only allowed as a separator between reference tokens
            '/' if !escaped => escaped = false,
            '~' if !escaped => escaped = true,
            '0' | '1' if escaped => escaped = false,
            // These ranges cover all allowed unescaped characters
            '\x00'..='\x2E' | '\x30'..='\x7D' | '\x7F'..='\u{10FFFF}' if !escaped => {}
            // Any other character or combination is invalid
            _ => return false,
        }
    }
    // If we end in an escaped state, it's invalid
    !escaped
}

#[must_use]
pub fn is_valid_date(date: &str) -> bool {
    if date.len() != 10 {
        return false;
    }

    let bytes = date.as_bytes();

    // Check format: YYYY-MM-DD
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }

    // Parse year (YYYY)
    let Some(year) = parse_four_digits(&bytes[0..4]) else {
        return false;
    };

    // Parse month (MM)
    let Some(month) = parse_two_digits(&bytes[5..7]) else {
        return false;
    };
    if !(1..=12).contains(&month) {
        return false;
    }

    // Parse day (DD)
    let Some(day) = parse_two_digits(&bytes[8..10]) else {
        return false;
    };
    if day == 0 {
        return false;
    }

    // Check day validity
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => day <= 31,
        4 | 6 | 9 | 11 => day <= 30,
        2 => {
            if is_leap_year(year) {
                day <= 29
            } else {
                day <= 28
            }
        }
        _ => unreachable!("Month value is checked above"),
    }
}

#[inline]
fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[inline]
fn parse_four_digits(bytes: &[u8]) -> Option<u16> {
    // Little-endian layout: bytes[0] lands in the lowest byte of the u32.
    // Check if all bytes are ASCII digits
    let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let sub = value.wrapping_sub(0x3030_3030);
    if sub.wrapping_add(0x0606_0606) & 0xF0F0_F0F0 != 0 {
        return None;
    }
    let val = (sub & 0x0F0F_0F0F).wrapping_mul(2561) >> 8;
    Some(((val & 0x00FF_00FF).wrapping_mul(6_553_601) >> 16) as u16)
}

#[inline]
fn parse_two_digits(bytes: &[u8]) -> Option<u8> {
    // Little-endian layout: bytes[0] lands in the low byte of the u16.
    // Check if all bytes are ASCII digits
    let value = u16::from_le_bytes([bytes[0], bytes[1]]);
    let sub = value.wrapping_sub(0x3030);
    if sub.wrapping_add(0x0606) & 0xF0F0 != 0 {
        return None;
    }
    Some(((sub & 0x0F0F).wrapping_mul(2561) >> 8) as u8)
}

macro_rules! handle_offset {
    ($sign:tt, $i:ident, $bytes:expr, $hour:expr, $minute:expr, $second:expr) => {{
        if $bytes.len() - $i != 6 {
            return false;
        }
        $i += 1;
        if $bytes[$i + 2] != b':' {
            return false;
        }
        let Some(offset_hh) = parse_two_digits(&$bytes[$i..$i + 2]) else {
            return false;
        };
        let Some(offset_mm) = parse_two_digits(&$bytes[$i + 3..$i + 5]) else {
            return false;
        };
        if offset_hh > 23 || offset_mm > 59 {
            return false;
        }

        if $second == 60 {
            let mut utc_hh = i16::from($hour);
            let mut utc_mm = i16::from($minute);
            let offset_hh = i16::from(offset_hh);
            let offset_mm = i16::from(offset_mm);

            // Apply offset based on the sign (+ or -)
            utc_hh $sign offset_hh;
            utc_mm $sign offset_mm;

            // Adjust for minute overflow/underflow
            utc_hh += utc_mm / 60;
            utc_mm %= 60;
            if utc_mm < 0 {
                utc_mm += 60;
                utc_hh -= 1;
            }

            // Adjust for hour overflow/underflow
            utc_hh = (utc_hh + 24) % 24;
            utc_hh == 23 && utc_mm == 59
        } else {
            true
        }
    }};
}

#[must_use]
pub fn is_valid_time(time: &str) -> bool {
    let bytes = time.as_bytes();
    let len = bytes.len();

    if len < 9 {
        // Minimum valid time is "HH:MM:SSZ"
        return false;
    }

    // Check HH:MM:SS format
    if bytes[2] != b':' || bytes[5] != b':' {
        return false;
    }

    // Parse hour (HH)
    let Some(hour) = parse_two_digits(&bytes[..2]) else {
        return false;
    };
    // Parse minute (MM)
    let Some(minute) = parse_two_digits(&bytes[3..5]) else {
        return false;
    };
    // Parse second (SS)
    let Some(second) = parse_two_digits(&bytes[6..8]) else {
        return false;
    };

    if hour > 23 || minute > 59 || second > 60 {
        return false;
    }

    let mut i = 8;

    // Check fractional seconds
    if i < len && bytes[i] == b'.' {
        i += 1;
        let mut has_digit = false;
        while i < len && bytes[i].is_ascii_digit() {
            has_digit = true;
            i += 1;
        }
        if !has_digit {
            return false;
        }
    }

    // Check offset
    if i == len {
        return false;
    }

    match bytes[i] {
        b'Z' | b'z' => i == len - 1 && (second != 60 || (hour == 23 && minute == 59)),
        b'+' => handle_offset!(-=, i, bytes, hour, minute, second),
        b'-' => handle_offset!(+=, i, bytes, hour, minute, second),
        _ => false,
    }
}

#[must_use]
pub fn is_valid_datetime(datetime: &str) -> bool {
    // Find the position of 'T' or 't' separator
    let Some(t_pos) = datetime.bytes().position(|b| b == b'T' || b == b't') else {
        return false;
    };

    // Split the string into date and time parts
    let (date_part, time_part) = datetime.split_at(t_pos);

    is_valid_date(date_part) && is_valid_time(&time_part[1..])
}

fn parse_email(email: &str, options: Option<&EmailAddressOptions>) -> Option<EmailAddress> {
    if let Some(opts) = options {
        EmailAddress::parse_with_options(email, *opts)
    } else {
        EmailAddress::from_str(email)
    }
    .ok()
}

const IPV6_TAG: &str = "IPv6:";

fn validate_email_domain<F>(domain: &str, is_valid_hostname_impl: &F) -> bool
where
    F: Fn(&str) -> bool,
{
    if let Some(domain) = domain.strip_prefix('[').and_then(|d| d.strip_suffix(']')) {
        // RFC 5321 tags are case-insensitive
        if domain
            .as_bytes()
            .get(..IPV6_TAG.len())
            .is_some_and(|tag| tag.eq_ignore_ascii_case(IPV6_TAG.as_bytes()))
        {
            domain[IPV6_TAG.len()..].parse::<Ipv6Addr>().is_ok()
        } else {
            domain.parse::<Ipv4Addr>().is_ok()
        }
    } else {
        is_valid_hostname_impl(domain)
    }
}

fn is_valid_email_impl<F>(
    email: &str,
    is_valid_hostname_impl: F,
    options: Option<&EmailAddressOptions>,
    allow_non_ascii_local_part: bool,
) -> bool
where
    F: Fn(&str) -> bool,
{
    // `email_address` 0.2.9 rejects an empty quoted local part, which RFC 5321 allows.
    if let Some(domain) = email.strip_prefix("\"\"@") {
        return is_valid_email_impl(
            &format!("\"a\"@{domain}"),
            is_valid_hostname_impl,
            options,
            allow_non_ascii_local_part,
        );
    }
    if let Some(parsed) = parse_email(email, options) {
        if !allow_non_ascii_local_part && !parsed.local_part().is_ascii() {
            return false;
        }
        return validate_email_domain(parsed.domain(), &is_valid_hostname_impl);
    }
    // `email_address` 0.2.9 rejects non-ASCII in quoted local parts, which `idn-email`
    // must accept. Mask non-ASCII to pass the structural check, then validate the real
    // domain (after the last `@`, which a local part never holds unquoted).
    if !allow_non_ascii_local_part || email.is_ascii() {
        return false;
    }
    let masked: String = email
        .chars()
        .map(|c| if c.is_ascii() { c } else { 'a' })
        .collect();
    if parse_email(&masked, options).is_none() {
        return false;
    }
    let mut parts = email.rsplitn(2, '@');
    let domain = parts.next().unwrap_or_default();
    if parts.next().is_none() {
        return false;
    }
    validate_email_domain(domain, &is_valid_hostname_impl)
}

pub(crate) fn is_valid_email(email: &str, options: Option<&EmailAddressOptions>) -> bool {
    is_valid_email_impl(email, is_valid_hostname, options, false)
}

#[cfg(feature = "idna")]
pub(crate) fn is_valid_idn_email(email: &str, options: Option<&EmailAddressOptions>) -> bool {
    is_valid_email_impl(email, is_valid_idn_hostname, options, true)
}

const VALID_HOSTNAME_CHARS: [bool; 256] = {
    let mut table = [false; 256];
    let mut byte: u8 = 0;
    while byte < 255 {
        table[byte as usize] = matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-');
        byte += 1;
    }
    // Handle byte 255 separately to avoid overflow
    table[255] = matches!(255u8, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-');
    table
};

#[inline]
fn is_punycode_label(label: &[u8]) -> bool {
    label.len() >= 4 && label[0] == b'x' && label[1] == b'n' && label[2] == b'-' && label[3] == b'-'
}

#[inline]
fn validate_hostname_label(label: &[u8]) -> bool {
    !label.is_empty() && label.len() <= 63 && label[0] != b'-' && *label.last().unwrap() != b'-'
}

fn is_valid_ascii_hostname(hostname: &str) -> bool {
    let hostname_bytes = hostname.as_bytes();
    let len = hostname_bytes.len();
    if len == 0 || len > 253 || hostname_bytes[len - 1] == b'.' {
        return false;
    }

    let mut label_start = 0;
    let mut i = 0;
    while i < len {
        if hostname_bytes[i] == b'.' {
            if !validate_hostname_label(&hostname_bytes[label_start..i]) {
                return false;
            }
            label_start = i + 1;
        } else if !VALID_HOSTNAME_CHARS[hostname_bytes[i] as usize] {
            return false;
        }
        i += 1;
    }

    validate_hostname_label(&hostname_bytes[label_start..])
}

#[must_use]
pub fn is_valid_hostname_rfc1034(hostname: &str) -> bool {
    is_valid_ascii_hostname(hostname)
}

// RFC 3492 parameters for the Punycode variant used by IDNA:
// https://www.rfc-editor.org/rfc/rfc3492#section-5
const PUNYCODE_BASE: u32 = 36;
const PUNYCODE_TMIN: u32 = 1;
const PUNYCODE_TMAX: u32 = 26;
const PUNYCODE_SKEW: u32 = 38;
const PUNYCODE_DAMP: u32 = 700;
const PUNYCODE_INITIAL_BIAS: u32 = 72;
const PUNYCODE_INITIAL_N: u32 = 128;

fn punycode_digit(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte) - u32::from(b'0') + 26),
        b'A'..=b'Z' => Some(u32::from(byte) - u32::from(b'A')),
        b'a'..=b'z' => Some(u32::from(byte) - u32::from(b'a')),
        _ => None,
    }
}

// RFC 3492, section 6.1: https://www.rfc-editor.org/rfc/rfc3492#section-6.1
fn punycode_adapt(mut delta: u32, count: u32, first: bool) -> u32 {
    delta /= if first { PUNYCODE_DAMP } else { 2 };
    delta += delta / count;
    let mut k = 0;
    while delta > ((PUNYCODE_BASE - PUNYCODE_TMIN) * PUNYCODE_TMAX) / 2 {
        delta /= PUNYCODE_BASE - PUNYCODE_TMIN;
        k += PUNYCODE_BASE;
    }
    k + ((PUNYCODE_BASE - PUNYCODE_TMIN + 1) * delta) / (delta + PUNYCODE_SKEW)
}

/// Decode a Punycode payload, the part of an A-label after the `xn--` prefix.
///
/// RFC 3492, section 6.2: <https://www.rfc-editor.org/rfc/rfc3492#section-6.2>
fn decode_punycode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    // The delimiter is consumed only when it is preceded by basic code points; a leading
    // `-` is part of the encoded portion and makes the label invalid.
    let (basic, encoded) = match bytes.iter().rposition(|&byte| byte == b'-') {
        Some(position) if position > 0 => (&bytes[..position], &bytes[position + 1..]),
        _ => (&bytes[..0], bytes),
    };
    if !basic.is_ascii() {
        return None;
    }
    let mut output: Vec<char> = basic.iter().map(|&byte| char::from(byte)).collect();

    let mut code_point = PUNYCODE_INITIAL_N;
    let mut index: u32 = 0;
    let mut bias = PUNYCODE_INITIAL_BIAS;
    let mut position = 0;

    while position < encoded.len() {
        let previous = index;
        let mut weight: u32 = 1;
        let mut k = PUNYCODE_BASE;
        loop {
            let digit = punycode_digit(*encoded.get(position)?)?;
            position += 1;
            index = index.checked_add(digit.checked_mul(weight)?)?;
            let threshold = if k <= bias {
                PUNYCODE_TMIN
            } else if k >= bias + PUNYCODE_TMAX {
                PUNYCODE_TMAX
            } else {
                k - bias
            };
            if digit < threshold {
                break;
            }
            weight = weight.checked_mul(PUNYCODE_BASE - threshold)?;
            k += PUNYCODE_BASE;
        }

        let count = u32::try_from(output.len()).ok()? + 1;
        bias = punycode_adapt(index - previous, count, previous == 0);
        code_point = code_point.checked_add(index / count)?;
        index %= count;
        output.insert(index as usize, char::from_u32(code_point)?);
        index += 1;
    }

    Some(output.into_iter().collect())
}

/// # Panics
///
/// Panics if a punycode label contains non-UTF-8 bytes, which cannot happen
/// because the label has already been validated as ASCII.
#[must_use]
pub fn is_valid_hostname(hostname: &str) -> bool {
    if !is_valid_ascii_hostname(hostname) {
        return false;
    }

    for label in hostname.as_bytes().split(|&b| b == b'.') {
        // Per RFC 5891, labels with hyphens in 3rd & 4th positions must be valid A-labels.
        if label.len() >= 4 && label[2] == b'-' && label[3] == b'-' && !is_punycode_label(label) {
            return false;
        }

        if is_punycode_label(label) {
            let payload = std::str::from_utf8(&label[4..]).expect("ASCII label already validated");
            let Some(decoded) = decode_punycode(payload) else {
                return false;
            };
            if !validate_unicode_label(&decoded) {
                return false;
            }
        }
    }

    true
}

// RFC 5892 derives the PVALID property primarily from these general categories.
fn is_idna_pvalid_category(category: GeneralCategory) -> bool {
    matches!(
        category,
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::DecimalNumber
    )
}

fn validate_unicode_label(label: &str) -> bool {
    let mut chars = label.chars().peekable();
    if let Some(&first) = chars.peek() {
        let category = get_general_category(first);
        if matches!(
            category,
            GeneralCategory::SpacingMark
                | GeneralCategory::NonspacingMark
                | GeneralCategory::EnclosingMark
        ) {
            return false;
        }
    }
    let mut previous = None;
    let mut has_katakana_middle_dot = false;
    let mut has_hiragana_katakana_han = false;
    let mut has_arabic_indic_digits = false;
    let mut has_extended_arabic_indic_digits = false;

    while let Some(current) = chars.next() {
        match current {
            // ZERO WIDTH JOINER
            // https://www.rfc-editor.org/rfc/rfc5892#appendix-A.2
            '\u{200D}'
                if !previous.is_some_and(|prev| {
                    matches!(
                        prev,
                        '\u{094D}'
                            | '\u{09CD}'
                            | '\u{0A4D}'
                            | '\u{0ACD}'
                            | '\u{0B4D}'
                            | '\u{0BCD}'
                            | '\u{0C4D}'
                            | '\u{0CCD}'
                            | '\u{0D4D}'
                            | '\u{0DCA}'
                            | '\u{0E3A}'
                            | '\u{0F84}'
                            | '\u{1039}'
                            | '\u{1714}'
                            | '\u{1734}'
                            | '\u{17D2}'
                            | '\u{1A60}'
                            | '\u{1B44}'
                            | '\u{1BAA}'
                            | '\u{1BF2}'
                            | '\u{1BF3}'
                            | '\u{2D7F}'
                            | '\u{A806}'
                            | '\u{A8C4}'
                            | '\u{A953}'
                            | '\u{ABED}'
                            | '\u{10A3F}'
                            | '\u{11046}'
                            | '\u{1107F}'
                            | '\u{110B9}'
                            | '\u{11133}'
                            | '\u{111C0}'
                            | '\u{11235}'
                            | '\u{112EA}'
                            | '\u{1134D}'
                            | '\u{11442}'
                            | '\u{114C2}'
                            | '\u{115BF}'
                            | '\u{1163F}'
                            | '\u{116B6}'
                            | '\u{1172B}'
                            | '\u{11839}'
                            | '\u{119E0}'
                            | '\u{11A34}'
                            | '\u{11A47}'
                            | '\u{11A99}'
                            | '\u{11C3F}'
                            | '\u{11D44}'
                            | '\u{11D45}'
                            | '\u{11D97}'
                    )
                }) =>
            {
                return false;
            }
            // MIDDLE DOT
            // https://www.rfc-editor.org/rfc/rfc5892#appendix-A.3
            '\u{00B7}' if previous != Some('l') || chars.peek() != Some(&'l') => return false,
            // Greek KERAIA
            // https://www.rfc-editor.org/rfc/rfc5892#appendix-A.4
            '\u{0375}'
                if !chars
                    .peek()
                    .is_some_and(|next| ('\u{0370}'..='\u{03FF}').contains(next)) =>
            {
                return false
            }
            // Hebrew GERESH and GERSHAYIM
            // https://www.rfc-editor.org/rfc/rfc5892#appendix-A.5
            // https://www.rfc-editor.org/rfc/rfc5892#appendix-A.6
            '\u{05F3}' | '\u{05F4}'
                if !previous.is_some_and(|prev| ('\u{0590}'..='\u{05FF}').contains(&prev)) =>
            {
                return false
            }
            // KATAKANA MIDDLE DOT
            '\u{30FB}' => has_katakana_middle_dot = true,
            // Hiragana, Katakana, or Han
            // https://www.rfc-editor.org/rfc/rfc5892#appendix-A.7
            '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' | '\u{4E00}'..='\u{9FFF}' => {
                has_hiragana_katakana_han = true;
            }
            // ARABIC-INDIC DIGITS
            // https://www.rfc-editor.org/rfc/rfc5892#appendix-A.8
            '\u{0660}'..='\u{0669}' => has_arabic_indic_digits = true,
            // EXTENDED ARABIC-INDIC DIGITS
            // https://www.rfc-editor.org/rfc/rfc5892#appendix-A.9
            '\u{06F0}'..='\u{06F9}' => has_extended_arabic_indic_digits = true,
            // DISALLOWED
            '\u{0640}' | '\u{07FA}' | '\u{302E}' | '\u{302F}' | '\u{3031}' | '\u{3032}'
            | '\u{3033}' | '\u{3034}' | '\u{3035}' | '\u{303B}' => return false,
            // Contextual joiners/punctuation already validated above, plus the RFC 5892
            // PVALID exceptions whose general category would otherwise be disallowed.
            '\u{200C}' | '\u{200D}' | '\u{00B7}' | '\u{0375}' | '\u{05F3}' | '\u{05F4}'
            | '\u{06FD}' | '\u{06FE}' | '\u{0F0B}' | '\u{3007}' => {}
            // Per RFC 5892 a code point is PVALID only when it is a letter, a combining
            // mark, or a decimal digit; any other decoded code point is disallowed.
            other if !other.is_ascii() && !is_idna_pvalid_category(get_general_category(other)) => {
                return false;
            }

            _ => {}
        }
        previous = Some(current);
    }

    if (has_katakana_middle_dot && !has_hiragana_katakana_han)
        || (has_arabic_indic_digits && has_extended_arabic_indic_digits)
    {
        return false;
    }

    true
}

#[cfg(feature = "idna")]
#[must_use]
pub fn is_valid_idn_hostname(hostname: &str) -> bool {
    use idna::uts46::{AsciiDenyList, DnsLength, Hyphens, Uts46};

    let Ok(ascii_hostname) = Uts46::new().to_ascii(
        hostname.as_bytes(),
        AsciiDenyList::STD3,
        // Prohibit hyphens in the first, third, fourth, and last position in the label
        Hyphens::Check,
        DnsLength::Verify,
    ) else {
        return false;
    };

    if !is_valid_hostname(&ascii_hostname) {
        return false;
    }

    let (unicode_hostname, _) = Uts46::new().to_unicode(
        ascii_hostname.as_bytes(),
        AsciiDenyList::EMPTY,
        Hyphens::Allow,
    );

    unicode_hostname
        .split('.')
        .all(|label| !label.is_empty() && validate_unicode_label(label))
}

#[inline]
fn unit_index(units: &[u8], unit: u8) -> Option<usize> {
    units.iter().position(|&u| u == unit)
}

#[must_use]
pub fn is_valid_duration(duration: &str) -> bool {
    let bytes = duration.as_bytes();
    let len = bytes.len();

    if len < 2 || bytes[0] != b'P' {
        return false;
    }

    let mut i = 1;
    let mut has_component = false;
    let mut has_time = false;
    let mut last_date_unit = 0;
    let mut last_time_unit = 0;
    let mut has_weeks = false;
    let mut has_time_component = false;
    let mut seen_units = 0u8;

    let date_units = *b"YMWD";
    let time_units = *b"HMS";

    while i < len {
        if bytes[i] == b'T' {
            // RFC 3339 Appendix A: `dur-week` stands alone, it never takes a time part.
            if has_time || has_weeks {
                return false;
            }
            has_time = true;
            i += 1;
            continue;
        }

        let start = i;
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }

        if i == start || i == len {
            return false;
        }

        let unit = bytes[i];

        if !has_time {
            if let Some(idx) = unit_index(&date_units, unit) {
                if unit == b'W' {
                    if has_component {
                        return false;
                    }
                    has_weeks = true;
                } else if has_weeks {
                    return false;
                }
                if idx < last_date_unit || (seen_units & (1 << idx) != 0) {
                    return false;
                }
                seen_units |= 1 << idx;
                last_date_unit = idx;
            } else {
                return false;
            }
        } else if let Some(idx) = unit_index(&time_units, unit) {
            if idx < last_time_unit || (seen_units & (1 << (idx + 4)) != 0) {
                return false;
            }
            seen_units |= 1 << (idx + 4);
            last_time_unit = idx;
            has_time_component = true;
        } else {
            return false;
        }

        has_component = true;
        i += 1;
    }

    if !has_component || (has_time && !has_time_component) {
        return false;
    }

    // RFC 3339 ABNF: dur-year = Y [dur-month], dur-month = M [dur-day]
    // So Y+D without M is invalid
    let has_date_y = seen_units & (1 << 0) != 0; // Y is index 0 in date_units
    let has_date_m = seen_units & (1 << 1) != 0; // M is index 1 in date_units
    let has_date_d = seen_units & (1 << 3) != 0; // D is index 3 in date_units
    if has_date_y && has_date_d && !has_date_m {
        return false;
    }

    // RFC 3339 ABNF: dur-hour = H [dur-minute], dur-minute = M [dur-second]
    // So H+S without M is invalid
    let has_time_h = seen_units & (1 << 4) != 0; // H is index 0 in time_units, stored at +4
    let has_time_m = seen_units & (1 << 5) != 0; // M is index 1 in time_units, stored at +5
    let has_time_s = seen_units & (1 << 6) != 0; // S is index 2 in time_units, stored at +6
    if has_time_h && has_time_s && !has_time_m {
        return false;
    }

    true
}

#[must_use]
pub fn is_valid_ipv4(ip: &str) -> bool {
    Ipv4Addr::from_str(ip).is_ok()
}

#[must_use]
pub fn is_valid_ipv6(ip: &str) -> bool {
    Ipv6Addr::from_str(ip).is_ok()
}

#[must_use]
pub fn is_valid_iri(iri: &str) -> bool {
    referencing::Iri::parse(iri).is_ok()
}

#[must_use]
pub fn is_valid_iri_reference(iri_reference: &str) -> bool {
    referencing::IriRef::parse(iri_reference).is_ok()
}

#[must_use]
pub fn is_valid_uri(uri: &str) -> bool {
    rfc3986::check(uri, true).unwrap_or_else(|| referencing::Uri::parse(uri).is_ok())
}

#[must_use]
pub fn is_valid_uri_reference(uri_reference: &str) -> bool {
    rfc3986::check(uri_reference, false)
        .unwrap_or_else(|| referencing::UriRef::parse(uri_reference).is_ok())
}

#[must_use]
pub fn is_valid_uuid(uuid: &str) -> bool {
    let mut out = [0; 16];
    parse_hyphenated(uuid.as_bytes(), Out::from_mut(&mut out)).is_ok()
}

/// Validate that a string is a valid ECMAScript regular expression.
#[must_use]
pub fn is_valid_regex(pattern: &str) -> bool {
    const CACHE_SIZE: usize = 16;
    thread_local! {
        static CACHE: std::cell::RefCell<(Vec<(String, bool)>, usize)> =
            const { std::cell::RefCell::new((Vec::new(), 0)) };
    }
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let (entries, next) = &mut *cache;
        if let Some((_, valid)) = entries.iter().find(|(candidate, _)| candidate == pattern) {
            return *valid;
        }
        let valid = jsonschema_regex::is_valid_ecma_regex(pattern);
        if entries.len() < CACHE_SIZE {
            entries.push((pattern.to_owned(), valid));
        } else {
            entries[*next] = (pattern.to_owned(), valid);
            *next = (*next + 1) % CACHE_SIZE;
        }
        valid
    })
}

/// Implements `evaluate()` for format validators that have an `annotation: Arc<Value>` field.
///
/// Per spec §7.2.1 and §7.2.2, the format value MUST be collected as an annotation
/// regardless of whether the assertion passes or fails.
macro_rules! impl_format_evaluate {
    () => {
        fn evaluate(
            &self,
            instance: &F::Node<'_>,
            location: &LazyLocation,
            tracker: Option<&RefTracker>,
            ctx: &mut ValidationContext,
        ) -> EvaluationResult {
            if !instance.is_string() {
                return EvaluationResult::valid_empty();
            }
            let mut collected = Vec::new();
            Validate::<F>::collect_errors(self, instance, location, tracker, ctx, &mut collected);
            let errors: Vec<_> = collected
                .iter()
                .map(crate::evaluation::ErrorDescription::from_validation_error)
                .collect();
            let mut result = if errors.is_empty() {
                EvaluationResult::valid_empty()
            } else {
                EvaluationResult::invalid_empty(errors)
            };
            result.annotate(Annotations::from_arc(Arc::clone(&self.annotation)));
            result
        }
    };
}

macro_rules! format_validators {
    ($($(#[$meta:meta])* ($validator:ident, $format:expr, $validation_fn:ident)),+ $(,)?) => {
        $(
            $(#[$meta])*
            struct $validator {
                location: Location,
                annotation: Arc<Value>,
            }

            $(#[$meta])*
            impl $validator {
                pub(crate) fn compile<'a, F: Json>(ctx: &compiler::Context<F>) -> CompilationResult<'a, F> {
                    let location = ctx.location().join("format");
                    let annotation = Arc::new(Value::String($format.to_owned()));
                    Ok(Box::new($validator { location, annotation }))
                }
            }

            $(#[$meta])*
            impl<F: Json> Validate<F> for $validator {
                fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
                    if let Some(item) = instance.as_string() {
                        $validation_fn(&item)
                    } else {
                        true
                    }
                }

                fn validate<'i>(
                    &self,
                    instance: &F::Node<'i>,
                    location: &LazyLocation,
                    tracker: Option<&RefTracker>,
                    ctx: &mut ValidationContext,
                ) -> Result<(), ValidationError<'i>> {
                    if instance.is_string() {
                        if !Validate::<F>::is_valid(self, instance, ctx) {
                            return Err(ValidationError::format(
                                self.location.clone(),
                                crate::paths::capture_evaluation_path(tracker, &self.location),
                                location.into(),
                                instance.lazy_value(),
                                $format,
                            ));
                        }
                    }
                    Ok(())
                }

                impl_format_evaluate!();
            }
        )+
    };
}
format_validators!(
    (DateValidator, "date", is_valid_date),
    (DateTimeValidator, "date-time", is_valid_datetime),
    (DurationValidator, "duration", is_valid_duration),
    (
        HostnameValidatorDraft4,
        "hostname",
        is_valid_hostname_rfc1034
    ),
    (HostnameValidator, "hostname", is_valid_hostname),
    #[cfg(feature = "idna")]
    (IdnHostnameValidator, "idn-hostname", is_valid_idn_hostname),
    (IpV4Validator, "ipv4", is_valid_ipv4),
    (IpV6Validator, "ipv6", is_valid_ipv6),
    (IriValidator, "iri", is_valid_iri),
    (
        IriReferenceValidator,
        "iri-reference",
        is_valid_iri_reference
    ),
    (JsonPointerValidator, "json-pointer", is_valid_json_pointer),
    (
        RelativeJsonPointerValidator,
        "relative-json-pointer",
        is_valid_relative_json_pointer
    ),
    (TimeValidator, "time", is_valid_time),
    (UriValidator, "uri", is_valid_uri),
    (
        UriReferenceValidator,
        "uri-reference",
        is_valid_uri_reference
    ),
    (UriTemplateValidator, "uri-template", is_valid_uri_template),
    (UuidValidator, "uuid", is_valid_uuid),
);

// Custom RegexValidator that caches ECMA regex transformation results in ValidationContext
struct RegexValidator {
    location: Location,
    annotation: Arc<Value>,
}

impl RegexValidator {
    pub(crate) fn compile<'a, F: Json>(ctx: &compiler::Context<F>) -> CompilationResult<'a, F> {
        let location = ctx.location().join("format");
        let annotation = Arc::new(Value::String("regex".to_owned()));
        Ok(Box::new(RegexValidator {
            location,
            annotation,
        }))
    }
}

impl<F: Json> Validate<F> for RegexValidator {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            ctx.is_valid_ecma_regex(&item)
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if instance.is_string() && !Validate::<F>::is_valid(self, instance, ctx) {
            return Err(ValidationError::format(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                "regex",
            ));
        }
        Ok(())
    }

    impl_format_evaluate!();
}

// Custom EmailValidator that supports email options
struct EmailValidator {
    location: Location,
    annotation: Arc<Value>,
    email_options: Option<EmailAddressOptions>,
}

impl EmailValidator {
    pub(crate) fn compile<'a, F: Json>(ctx: &compiler::Context<F>) -> CompilationResult<'a, F> {
        let location = ctx.location().join("format");
        let annotation = Arc::new(Value::String("email".to_owned()));
        let email_options = ctx.config().email_options().copied();
        Ok(Box::new(EmailValidator {
            location,
            annotation,
            email_options,
        }))
    }
}

impl<F: Json> Validate<F> for EmailValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            is_valid_email(&item, self.email_options.as_ref())
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if instance.is_string() && !Validate::<F>::is_valid(self, instance, ctx) {
            return Err(ValidationError::format(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                "email",
            ));
        }
        Ok(())
    }

    impl_format_evaluate!();
}

// Custom IdnEmailValidator that supports email options
#[cfg(feature = "idna")]
struct IdnEmailValidator {
    location: Location,
    annotation: Arc<Value>,
    email_options: Option<EmailAddressOptions>,
}

#[cfg(feature = "idna")]
impl IdnEmailValidator {
    pub(crate) fn compile<'a, F: Json>(ctx: &compiler::Context<F>) -> CompilationResult<'a, F> {
        let location = ctx.location().join("format");
        let annotation = Arc::new(Value::String("idn-email".to_owned()));
        let email_options = ctx.config().email_options().copied();
        Ok(Box::new(IdnEmailValidator {
            location,
            annotation,
            email_options,
        }))
    }
}

#[cfg(feature = "idna")]
impl<F: Json> Validate<F> for IdnEmailValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            is_valid_idn_email(&item, self.email_options.as_ref())
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if instance.is_string() && !Validate::<F>::is_valid(self, instance, ctx) {
            return Err(ValidationError::format(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                "idn-email",
            ));
        }
        Ok(())
    }

    impl_format_evaluate!();
}

struct CustomFormatValidator {
    location: Location,
    annotation: Arc<Value>,
    format_name: String,
    check: Arc<dyn Format>,
}
impl CustomFormatValidator {
    pub(crate) fn compile<'a, F: Json>(
        ctx: &compiler::Context<F>,
        format_name: String,
        check: Arc<dyn Format>,
    ) -> CompilationResult<'a, F> {
        let location = ctx.location().join("format");
        let annotation = Arc::new(Value::String(format_name.clone()));
        Ok(Box::new(CustomFormatValidator {
            location,
            annotation,
            format_name,
            check,
        }))
    }
}

impl<F: Json> Validate<F> for CustomFormatValidator {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if Validate::<F>::is_valid(self, instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::format(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                self.format_name.clone(),
            ))
        }
    }

    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            self.check.is_valid(&item)
        } else {
            true
        }
    }

    impl_format_evaluate!();
}

/// Format annotation-only validator used when format assertion is disabled.
///
/// Always validates successfully but emits the required annotation per spec §7.2.1.
struct AnnotationOnlyFormatValidator {
    annotation: Arc<Value>,
}

impl<F: Json> Validate<F> for AnnotationOnlyFormatValidator {
    fn is_valid(&self, _instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        true
    }

    fn validate<'i>(
        &self,
        _instance: &F::Node<'i>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        Ok(())
    }

    fn collect_errors<'i>(
        &self,
        _instance: &F::Node<'i>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
        _errors: &mut Vec<ValidationError<'i>>,
    ) {
    }

    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if !instance.is_string() {
            return EvaluationResult::valid_empty();
        }
        let mut result = EvaluationResult::valid_empty();
        result.annotate(Annotations::from_arc(Arc::clone(&self.annotation)));
        result
    }
}

pub(crate) trait Format: Send + Sync + 'static {
    fn is_valid(&self, value: &str) -> bool;
}

impl<F> Format for F
where
    F: Fn(&str) -> bool + Send + Sync + 'static,
{
    #[inline]
    fn is_valid(&self, value: &str) -> bool {
        self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, VariantArray)]
pub(crate) enum BuiltinFormat {
    Date,
    DateTime,
    Duration,
    Email,
    Hostname,
    HostnameDraft4,
    #[cfg(feature = "idna")]
    IdnEmail,
    #[cfg(feature = "idna")]
    IdnHostname,
    Ipv4,
    Ipv6,
    Iri,
    IriReference,
    JsonPointer,
    Regex,
    RelativeJsonPointer,
    Time,
    Uri,
    UriReference,
    UriTemplate,
    Uuid,
}

pub(crate) fn builtin_format(draft: Draft, format: &str) -> Option<BuiltinFormat> {
    match format {
        "date" => Some(BuiltinFormat::Date),
        "date-time" => Some(BuiltinFormat::DateTime),
        "duration" if draft >= Draft::Draft201909 => Some(BuiltinFormat::Duration),
        "email" => Some(BuiltinFormat::Email),
        "hostname" if matches!(draft, Draft::Draft4 | Draft::Draft6) => {
            Some(BuiltinFormat::HostnameDraft4)
        }
        "hostname" => Some(BuiltinFormat::Hostname),
        #[cfg(feature = "idna")]
        "idn-email" => Some(BuiltinFormat::IdnEmail),
        #[cfg(feature = "idna")]
        "idn-hostname" if draft >= Draft::Draft7 => Some(BuiltinFormat::IdnHostname),
        "ipv4" => Some(BuiltinFormat::Ipv4),
        "ipv6" => Some(BuiltinFormat::Ipv6),
        "iri" if draft >= Draft::Draft7 => Some(BuiltinFormat::Iri),
        "iri-reference" if draft >= Draft::Draft7 => Some(BuiltinFormat::IriReference),
        "json-pointer" if draft >= Draft::Draft6 => Some(BuiltinFormat::JsonPointer),
        "regex" => Some(BuiltinFormat::Regex),
        "relative-json-pointer" if draft >= Draft::Draft7 => {
            Some(BuiltinFormat::RelativeJsonPointer)
        }
        "time" => Some(BuiltinFormat::Time),
        "uri" => Some(BuiltinFormat::Uri),
        "uri-reference" if draft >= Draft::Draft6 => Some(BuiltinFormat::UriReference),
        "uri-template" if draft >= Draft::Draft6 => Some(BuiltinFormat::UriTemplate),
        "uuid" if draft >= Draft::Draft201909 => Some(BuiltinFormat::Uuid),
        _ => None,
    }
}

impl BuiltinFormat {
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::DateTime => "date-time",
            Self::Duration => "duration",
            Self::Email => "email",
            Self::Hostname | Self::HostnameDraft4 => "hostname",
            #[cfg(feature = "idna")]
            Self::IdnEmail => "idn-email",
            #[cfg(feature = "idna")]
            Self::IdnHostname => "idn-hostname",
            Self::Ipv4 => "ipv4",
            Self::Ipv6 => "ipv6",
            Self::Iri => "iri",
            Self::IriReference => "iri-reference",
            Self::JsonPointer => "json-pointer",
            Self::Regex => "regex",
            Self::RelativeJsonPointer => "relative-json-pointer",
            Self::Time => "time",
            Self::Uri => "uri",
            Self::UriReference => "uri-reference",
            Self::UriTemplate => "uri-template",
            Self::Uuid => "uuid",
        }
    }

    /// The lengths this format admits, or `None` where it takes any length.
    ///
    /// A window narrower than the strings `is_valid` accepts folds a satisfiable schema to `false`,
    /// so every bound below comes from a length check in the matching `is_valid_*`. Those count
    /// bytes, which equals the character length a window is read against only for an ASCII-only
    /// format - so a format taking non-ASCII has no window.
    #[must_use]
    pub(crate) const fn length_window(self) -> Option<(u64, u64)> {
        match self {
            // `YYYY-MM-DD`.
            Self::Date => Some((10, 10)),
            // A shortest date, `T`, and a shortest time.
            Self::DateTime => Some((20, u64::MAX)),
            // `HH:MM:SSZ`, with no ceiling on fractional seconds.
            Self::Time => Some((9, u64::MAX)),
            // `P` and one component, as in `P1D`; two characters never parse.
            Self::Duration => Some((3, u64::MAX)),
            // The empty name and anything over 253 are turned down.
            Self::Hostname | Self::HostnameDraft4 => Some((1, 253)),
            // Eight-four-four-four-twelve hex digits plus four hyphens.
            Self::Uuid => Some((36, 36)),
            // `0.0.0.0` through `255.255.255.255`.
            Self::Ipv4 => Some((7, 15)),
            // `::` up to `0000:0000:0000:0000:0000:0000:255.255.255.255`.
            Self::Ipv6 => Some((2, 45)),
            // `email`, `idn-email` and `idn-hostname` take non-ASCII; the rest take any
            // length, empty included.
            #[cfg(feature = "idna")]
            Self::IdnEmail | Self::IdnHostname => None,
            Self::Email
            | Self::Iri
            | Self::IriReference
            | Self::JsonPointer
            | Self::Regex
            | Self::RelativeJsonPointer
            | Self::Uri
            | Self::UriReference
            | Self::UriTemplate => None,
        }
    }

    /// A string this format accepts. The test below checks every one against `is_valid`.
    #[must_use]
    pub(crate) const fn example(self) -> &'static str {
        match self {
            Self::Date => "2020-01-01",
            Self::DateTime => "2020-01-01T00:00:00Z",
            Self::Duration => "P1D",
            Self::Email => "a@b.co",
            #[cfg(feature = "idna")]
            Self::IdnEmail => "a@b.co",
            Self::Hostname | Self::HostnameDraft4 => "example.com",
            #[cfg(feature = "idna")]
            Self::IdnHostname => "example.com",
            Self::Ipv4 => "127.0.0.1",
            Self::Ipv6 => "::1",
            Self::Iri | Self::IriReference | Self::Uri | Self::UriReference => "http://example.com",
            Self::JsonPointer => "/a",
            Self::Regex => "a",
            Self::RelativeJsonPointer => "0",
            Self::Time => "00:00:00Z",
            Self::UriTemplate => "http://example.com/{id}",
            Self::Uuid => "00000000-0000-4000-8000-000000000000",
        }
    }

    pub(crate) fn is_valid(self, text: &str) -> bool {
        match self {
            Self::Date => is_valid_date(text),
            Self::DateTime => is_valid_datetime(text),
            Self::Duration => is_valid_duration(text),
            Self::Email => is_valid_email(text, None),
            Self::Hostname => is_valid_hostname(text),
            Self::HostnameDraft4 => is_valid_hostname_rfc1034(text),
            #[cfg(feature = "idna")]
            Self::IdnEmail => is_valid_idn_email(text, None),
            #[cfg(feature = "idna")]
            Self::IdnHostname => is_valid_idn_hostname(text),
            Self::Ipv4 => is_valid_ipv4(text),
            Self::Ipv6 => is_valid_ipv6(text),
            Self::Iri => is_valid_iri(text),
            Self::IriReference => is_valid_iri_reference(text),
            Self::JsonPointer => is_valid_json_pointer(text),
            Self::Regex => is_valid_regex(text),
            Self::RelativeJsonPointer => is_valid_relative_json_pointer(text),
            Self::Time => is_valid_time(text),
            Self::Uri => is_valid_uri(text),
            Self::UriReference => is_valid_uri_reference(text),
            Self::UriTemplate => is_valid_uri_template(text),
            Self::Uuid => is_valid_uuid(text),
        }
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    if let Value::String(format) = schema {
        if ctx.validates_formats_by_default() {
            // Format validation is enabled: each specific validator carries its own annotation
            if let Some((name, func)) = ctx.get_format(format) {
                return Some(CustomFormatValidator::compile(
                    ctx,
                    name.clone(),
                    func.clone(),
                ));
            }
            let draft = ctx.draft();
            match builtin_format(draft, format) {
                Some(BuiltinFormat::Date) => Some(DateValidator::compile(ctx)),
                Some(BuiltinFormat::DateTime) => Some(DateTimeValidator::compile(ctx)),
                Some(BuiltinFormat::Duration) => Some(DurationValidator::compile(ctx)),
                Some(BuiltinFormat::Email) => Some(EmailValidator::compile(ctx)),
                Some(BuiltinFormat::Hostname) => Some(HostnameValidator::compile(ctx)),
                Some(BuiltinFormat::HostnameDraft4) => Some(HostnameValidatorDraft4::compile(ctx)),
                #[cfg(feature = "idna")]
                Some(BuiltinFormat::IdnEmail) => Some(IdnEmailValidator::compile(ctx)),
                #[cfg(feature = "idna")]
                Some(BuiltinFormat::IdnHostname) => Some(IdnHostnameValidator::compile(ctx)),
                Some(BuiltinFormat::Ipv4) => Some(IpV4Validator::compile(ctx)),
                Some(BuiltinFormat::Ipv6) => Some(IpV6Validator::compile(ctx)),
                Some(BuiltinFormat::Iri) => Some(IriValidator::compile(ctx)),
                Some(BuiltinFormat::IriReference) => Some(IriReferenceValidator::compile(ctx)),
                Some(BuiltinFormat::JsonPointer) => Some(JsonPointerValidator::compile(ctx)),
                Some(BuiltinFormat::Regex) => Some(RegexValidator::compile(ctx)),
                Some(BuiltinFormat::RelativeJsonPointer) => {
                    Some(RelativeJsonPointerValidator::compile(ctx))
                }
                Some(BuiltinFormat::Time) => Some(TimeValidator::compile(ctx)),
                Some(BuiltinFormat::Uri) => Some(UriValidator::compile(ctx)),
                Some(BuiltinFormat::UriReference) => Some(UriReferenceValidator::compile(ctx)),
                Some(BuiltinFormat::UriTemplate) => Some(UriTemplateValidator::compile(ctx)),
                Some(BuiltinFormat::Uuid) => Some(UuidValidator::compile(ctx)),
                None => {
                    if ctx.are_unknown_formats_ignored() {
                        None
                    } else {
                        let message = if ctx.asserts_formats_by_dialect() {
                            format!(
                                "Unknown format: '{format}'. The meta-schema asserts formats, so unrecognized ones cannot be ignored. Register a check for it or disable format validation"
                            )
                        } else {
                            format!(
                                "Unknown format: '{format}'. Adjust configuration to ignore unrecognized formats"
                            )
                        };
                        let location = ctx.location().join("format");
                        Some(Err(ValidationError::compile_error(
                            location.clone(),
                            location,
                            Location::new(),
                            LazyInstance::Ready(Cow::Borrowed(schema)),
                            message,
                        )))
                    }
                }
            }
        } else {
            // Format validation disabled: annotation-only per spec §7.2.1
            Some(Ok(Box::new(AnnotationOnlyFormatValidator {
                annotation: Arc::new(Value::String(format.clone())),
            })))
        }
    } else {
        let location = ctx.location().join("format");
        Some(Err(ValidationError::single_type_error(
            location.clone(),
            location,
            Location::new(),
            LazyInstance::Ready(Cow::Borrowed(schema)),
            JsonType::String,
        )))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/format/tests_1773.rs"));
