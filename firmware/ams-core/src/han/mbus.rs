//! M-Bus wired frame (`MbusParser.cpp`).

use super::context::{ParseError, ParserContext};

const MBUS_START: u8 = 0x68;
const MBUS_END: u8 = 0x16;

pub struct MbusParser {
    last_sequence: u8,
    buf: Vec<u8>,
    pos: usize,
}

impl Default for MbusParser {
    fn default() -> Self {
        Self {
            last_sequence: 0,
            buf: Vec::new(),
            pos: 0,
        }
    }
}

impl MbusParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(&mut self, buf: &mut [u8], ctx: &mut ParserContext) -> Result<usize, ParseError> {
        if ctx.length < 4 || buf.len() < ctx.length {
            return Err(ParseError::Incomplete);
        }
        let frame = &buf[..ctx.length];
        if frame[0] != MBUS_START || frame[2] != MBUS_START {
            return Err(ParseError::BoundaryFlagMissing);
        }
        if frame[1] != frame[3] {
            return Err(ParseError::Fail); // MBUS_FRAME_LENGTH_NOT_EQUAL
        }
        let mut len = frame[1] as usize;
        let headersize = 4usize;
        let footersize = 2usize;

        if len == 0 {
            len = ctx.length - headersize - footersize;
        }
        if len < headersize {
            len += 256;
        }
        if headersize + footersize + len > ctx.length {
            return Err(ParseError::Incomplete);
        }
        let footer_off = len + headersize;
        if frame[footer_off + 1] != MBUS_END {
            return Err(ParseError::BoundaryFlagMissing);
        }
        let fcs = frame[footer_off];
        if checksum(&frame[headersize..headersize + len]) != fcs {
            return Err(ParseError::FooterChecksumError);
        }

        let mut ptr = headersize + 2;
        let mut plen = len;
        plen -= 2;

        if ptr >= frame.len() {
            return Err(ParseError::Fail);
        }
        let ci = frame[ptr];
        ptr += 3;
        plen -= 3;

        let sequence = ci & 0x0F;
        if (ci & 0x10) == 0 {
            if sequence == 0 {
                self.buf.clear();
                self.buf.reserve(1024);
                self.pos = 0;
            } else if self.buf.is_empty() || self.pos + plen > 1024 || sequence != self.last_sequence.wrapping_add(1) {
                self.reset();
                return Err(ParseError::Fail);
            }
            self.buf.extend_from_slice(&frame[ptr..ptr + plen]);
            self.pos += plen;
            self.last_sequence = sequence;
            return Err(ParseError::IntermediateSegment);
        }
        if sequence > 0 {
            if self.buf.is_empty() || self.pos + plen > 1024 || sequence != self.last_sequence.wrapping_add(1) {
                self.reset();
                return Err(ParseError::Fail);
            }
            self.buf.extend_from_slice(&frame[ptr..ptr + plen]);
            self.pos += plen;
            return Err(ParseError::FinalSegment);
        }
        self.reset();
        Ok(ptr)
    }

    /// After `FinalSegment`, merge reassembled data to start of `buf` (matches `write`).
    pub fn write(&mut self, buf: &mut [u8], ctx: &mut ParserContext) {
        if !self.buf.is_empty() {
            let n = self.pos;
            buf[..n].copy_from_slice(&self.buf[..n]);
            ctx.length = n;
            self.buf.clear();
            self.pos = 0;
        }
    }

    fn reset(&mut self) {
        self.last_sequence = 0;
        self.buf.clear();
        self.pos = 0;
    }
}

fn checksum(p: &[u8]) -> u8 {
    p.iter().fold(0u8, |a, &b| a.wrapping_add(b))
}
