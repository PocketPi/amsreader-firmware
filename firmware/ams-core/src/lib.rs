//! Core AMS reader logic ported from the C++ firmware (ESP32-only Rust build).

pub mod han;

/// CRC-16/X-25 (HDLC), matching `lib/AmsDecoder/src/crc.cpp` `crc16_x25`.
pub fn crc16_x25(data: &[u8]) -> u16 {
    let mut crc: u16 = u16::MAX;
    for &b in data {
        let mut d = u16::from(b);
        for _ in 0..8 {
            let bit = (crc & 1) ^ (d & 1);
            crc = if bit != 0 {
                (crc >> 1) ^ 0x8408
            } else {
                crc >> 1
            };
            d >>= 1;
        }
    }
    let inv = !crc;
    (inv << 8) | ((inv >> 8) & 0xff)
}

/// CRC-16 (Modbus polynomial 0xA001), matching `crc16` in `lib/AmsDecoder/src/crc.cpp`.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= u16::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::han;

    #[test]
    fn crc16_empty() {
        assert_eq!(crc16(&[]), 0);
    }

    #[test]
    fn crc16_x25_known_vector() {
        let data = [0x01, 0x02, 0x03];
        let a = crc16_x25(&data);
        let b = reference_crc16_x25_c(&data);
        assert_eq!(a, b);
    }

    #[test]
    fn han_unwrap_dsmr_sample() {
        let hex = include_str!("../../../frames/dsmr.raw").trim();
        let mut raw: Vec<u8> = Vec::with_capacity(hex.len() / 2);
        for chunk in hex.as_bytes().chunks(2) {
            let s = std::str::from_utf8(chunk).unwrap();
            raw.push(u8::from_str_radix(s, 16).unwrap());
        }
        let mut ctx = han::ParserContext::default();
        let mut hdlc = han::HdlcParser::new();
        let mut mbus = han::MbusParser::new();
        let mut gbt = han::GbtParser::new();
        match han::try_unwrap_han(&mut raw, &mut ctx, &mut hdlc, &mut mbus, &mut gbt) {
            han::UnwrapResult::Complete {
                frame_type,
                payload_len,
                ..
            } => {
                assert_eq!(frame_type, han::DataTag::Dsmr as u8);
                assert!(payload_len > 10);
                assert_eq!(raw[0], b'/');
            }
            o => panic!("expected Complete DSMR, got {:?}", o),
        }
    }

    fn reference_crc16_x25_c(p: &[u8]) -> u16 {
        let mut crc: u16 = u16::MAX;
        for &byte in p {
            let mut d = u16::from(byte);
            for _ in 0..8 {
                crc = if (crc & 1) ^ (d & 1) != 0 {
                    (crc >> 1) ^ 0x8408
                } else {
                    crc >> 1
                };
                d >>= 1;
            }
        }
        let inv = !crc;
        (inv << 8) | ((inv >> 8) & 0xff)
    }
}
