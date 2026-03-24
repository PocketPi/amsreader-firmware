//! General Block Transfer (`GbtParser.cpp`).

use super::context::{ParseError, ParserContext};

const GBT_TAG: u8 = 0xE0;

pub struct GbtParser {
    last_sequence: u16,
    buf: Vec<u8>,
    pos: usize,
}

impl Default for GbtParser {
    fn default() -> Self {
        Self {
            last_sequence: 0,
            buf: Vec::new(),
            pos: 0,
        }
    }
}

impl GbtParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// On success returns **0** (C++ `DATA_PARSE_OK`); `ctx.length` becomes merged payload size.
    pub fn parse(&mut self, buf: &mut [u8], ctx: &mut ParserContext) -> Result<usize, ParseError> {
        if ctx.length < 8 || buf.len() < ctx.length {
            return Err(ParseError::Incomplete);
        }
        let frame_len = ctx.length;
        let frame = &mut buf[..frame_len];
        if frame[0] != GBT_TAG {
            return Err(ParseError::BoundaryFlagMissing);
        }
        let control = frame[1];
        let sequence = u16::from_be_bytes([frame[2], frame[3]]);
        let size = frame[7] as usize;
        let header = 8usize;
        if header + size > frame_len {
            return Err(ParseError::Incomplete);
        }

        if sequence == 1 {
            self.buf.clear();
            self.buf.reserve(1024);
            self.pos = 0;
        } else if self.last_sequence != sequence.wrapping_sub(1) {
            self.reset();
            return Err(ParseError::Fail);
        }

        let payload = &frame[header..header + size];
        if self.pos + size > 1024 {
            self.reset();
            return Err(ParseError::Fail);
        }
        self.buf.extend_from_slice(payload);
        self.pos += size;
        self.last_sequence = sequence;

        if (control & 0x80) == 0 {
            return Err(ParseError::IntermediateSegment);
        }

        let total = self.pos;
        if total > frame_len {
            self.reset();
            return Err(ParseError::Fail);
        }
        frame[..total].copy_from_slice(&self.buf[..total]);
        ctx.length = total;
        self.reset();
        Ok(0)
    }

    fn reset(&mut self) {
        self.last_sequence = 0;
        self.buf.clear();
        self.pos = 0;
    }
}
