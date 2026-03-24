//! Core AMS reader logic ported from the C++ firmware (ESP32-only Rust build).

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
