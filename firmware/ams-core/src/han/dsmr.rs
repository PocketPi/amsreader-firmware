//! DSMR / IEC 62056-21 text telegram (`DsmrParser.cpp`), plaintext only (no GCM).

use super::context::ParseError;
use crate::crc16;

/// Find `!` + CRC hex + CRLF; verify CRC. Returns end index of telegram (exclusive of trailing CRLF after CRC line).
pub fn parse(buf: &mut [u8], ctx_len: usize, verified: bool) -> Result<usize, ParseError> {
    if ctx_len > buf.len() {
        return Err(ParseError::Incomplete);
    }
    let slice = &buf[..ctx_len];
    if slice.is_empty() || slice[0] != b'/' {
        return Err(ParseError::BoundaryFlagMissing);
    }

    let mut crc_pos = 0usize;
    let mut last = 0u8;
    let mut end_line = None;
    for (pos, &b) in slice.iter().enumerate() {
        if pos > 0 && b == b'!' {
            crc_pos = pos + 1;
        }
        if crc_pos > 0 && b == 0x0A && last == 0x0D {
            end_line = Some(pos);
            break;
        }
        last = b;
    }
    let end_line = match end_line {
        Some(e) => e,
        None => return Err(ParseError::Incomplete),
    };

    if !verified && crc_pos > 0 && crc_pos + 4 <= slice.len() {
        let crc_calc = crc16(&slice[..crc_pos]);
        let hex = &slice[crc_pos..crc_pos + 4];
        let parsed = parse_hex_u16(hex);
        if let Some(crc) = parsed {
            if crc > 0 && crc != crc_calc {
                return Err(ParseError::FooterChecksumError);
            }
        }
    }

    Ok(end_line)
}

fn parse_hex_u16(h: &[u8]) -> Option<u16> {
    if h.len() != 4 {
        return None;
    }
    let a = from_hex(h[0])?;
    let b = from_hex(h[1])?;
    let c = from_hex(h[2])?;
    let d = from_hex(h[3])?;
    Some(((a as u16) << 12) | ((b as u16) << 8) | ((c as u16) << 4) | (d as u16))
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}
