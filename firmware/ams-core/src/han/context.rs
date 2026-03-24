/// First byte of a protocol layer (matches `DataParser.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DataTag {
    None = 0x00,
    Hdlc = 0x7E,
    Llc = 0xE6,
    Dlms = 0x0F,
    Dsmr = 0x2F,
    Mbus = 0x68,
    Gbt = 0xE0,
    Gcm = 0xDB,
    Snrm = 0x81,
    Aarq = 0x60,
    Aare = 0x61,
    Res = 0xC4,
}

impl TryFrom<u8> for DataTag {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x7E => Ok(DataTag::Hdlc),
            0xE6 => Ok(DataTag::Llc),
            0x0F => Ok(DataTag::Dlms),
            0x2F => Ok(DataTag::Dsmr),
            0x68 => Ok(DataTag::Mbus),
            0xE0 => Ok(DataTag::Gbt),
            0xDB => Ok(DataTag::Gcm),
            0x81 => Ok(DataTag::Snrm),
            0x60 => Ok(DataTag::Aarq),
            0x61 => Ok(DataTag::Aare),
            0xC4 => Ok(DataTag::Res),
            _ => Err(()),
        }
    }
}

/// Parse status (matches `DataParser.h` negative codes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    Fail = -1,
    Incomplete = -2,
    BoundaryFlagMissing = -3,
    HeaderChecksumError = -4,
    FooterChecksumError = -5,
    IntermediateSegment = -6,
    FinalSegment = -7,
    UnknownData = -9,
}

impl ParseError {
    pub const MBUS_FRAME_LENGTH_NOT_EQUAL: i8 = -41;

    pub fn as_i16(self) -> i16 {
        self as i8 as i16
    }
}

/// Working context while unwrapping (matches `DataParserContext`).
#[derive(Clone, Debug, Default)]
pub struct ParserContext {
    pub frame_type: u8,
    pub length: usize,
    /// Unix timestamp from DLMS COSEM datetime when present.
    pub timestamp: i64,
    pub system_title: [u8; 8],
}
