//! First DLMS/COSEM data unit (`DlmsParser.cpp` + `Cosem`).

use super::context::{ParseError, ParserContext};

const COSEM_NULL: u8 = 0x00;
const COSEM_OCTET_STRING: u8 = 0x09;
const COSEM_DATE_TIME: u8 = 0x19;
const COSEM_KAMSTRUP_BUG: u8 = 0x0C;

/// Parse leading notification; sets `ctx.timestamp` when a datetime is present.
/// Returns bytes consumed from `buf` (matching C++ `len`).
pub fn parse(buf: &[u8], ctx: &mut ParserContext) -> Result<usize, ParseError> {
    if ctx.length < 6 || buf.len() < ctx.length {
        return Err(ParseError::Incomplete);
    }
    let slice = &buf[..ctx.length];
    let mut ptr = 1usize;
    ptr += 4;
    if ptr >= slice.len() {
        return Err(ParseError::Incomplete);
    }

    let ty = slice[ptr];
    match ty {
        COSEM_OCTET_STRING => {
            let len_b = slice.get(ptr + 1).copied().unwrap_or(0);
            if len_b == 0x0C {
                if ptr + 2 + 14 > slice.len() {
                    return Err(ParseError::Incomplete);
                }
                let dt = &slice[ptr + 2..ptr + 2 + 14];
                ctx.timestamp = decode_cosem_datetime_octets(dt);
                let consumed = 5 + 14;
                ctx.length -= consumed;
                return Ok(consumed);
            }
            return Err(ParseError::UnknownData);
        }
        COSEM_NULL => {
            let consumed = 5 + 1;
            ctx.length -= consumed;
            ctx.timestamp = 0;
            return Ok(consumed);
        }
        COSEM_DATE_TIME => {
            if ptr + 13 > slice.len() {
                return Err(ParseError::Incomplete);
            }
            let dt = &slice[ptr..ptr + 13];
            ctx.timestamp = decode_cosem_datetime_struct(dt);
            let consumed = 5 + 13;
            ctx.length -= consumed;
            return Ok(consumed);
        }
        COSEM_KAMSTRUP_BUG => {
            if ptr + 13 > slice.len() {
                return Err(ParseError::Incomplete);
            }
            let dt = &slice[ptr..ptr + 13];
            ctx.timestamp = decode_cosem_datetime_struct(dt);
            let consumed = 5 + 13;
            ctx.length -= consumed;
            return Ok(consumed);
        }
        _ => Err(ParseError::UnknownData),
    }
}

fn decode_cosem_datetime_octets(_b: &[u8]) -> i64 {
    // Full decode needs calendar; keep 0 until we pull in time crate on device.
    0
}

fn decode_cosem_datetime_struct(b: &[u8]) -> i64 {
    if b.len() < 13 {
        return 0;
    }
    let year = u16::from_be_bytes([b[1], b[2]]);
    if year < 1970 {
        return 0;
    }
    let _month = b[3];
    let _day = b[4];
    let hour = b[6] as i64;
    let minute = b[7] as i64;
    let second = b[8] as i64;
    let deviation = i16::from_be_bytes([b[11], b[12]]);
    let days = days_since_1970(year, b[3], b[4]);
    let mut t = days * 86400 + hour * 3600 + minute * 60 + second;
    if (-720..=720).contains(&deviation) {
        t += (deviation as i64) * 60;
    }
    t
}

fn days_since_1970(year: u16, month: u8, day: u8) -> i64 {
    let mut d: i64 = 0;
    for y in 1970..year {
        d += if is_leap(y) { 366 } else { 365 };
    }
    let dim = [31u8, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        let mut days = dim[(m - 1) as usize] as i64;
        if m == 2 && is_leap(year) {
            days = 29;
        }
        d += days;
    }
    d += (day as i64) - 1;
    d
}

fn is_leap(y: u16) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
