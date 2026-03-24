//! HDLC format type 3 (`HdlcParser.cpp`).

use super::context::{ParseError, ParserContext};
use crate::crc16_x25;

const HDLC_FLAG: u8 = 0x7E;
const LLC_TAG: u8 = 0xE6;

/// State for HDLC multi-segment reassembly (matches `HDLCParser` class).
pub struct HdlcParser {
    last_sequence: u8,
    segment_buf: Vec<u8>,
}

impl Default for HdlcParser {
    fn default() -> Self {
        Self {
            last_sequence: 0,
            segment_buf: Vec::new(),
        }
    }
}

impl HdlcParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// On success returns **header length** (bytes to skip before inner payload starts), matching
    /// `return ptr - d` in C++. Updates `ctx.length` to inner payload length (one segment).
    pub fn parse(&mut self, buf: &mut [u8], ctx: &mut ParserContext) -> Result<usize, ParseError> {
        if ctx.length < 3 || buf.len() < ctx.length {
            return Err(ParseError::Incomplete);
        }

        let frame_len = ctx.length;
        let frame = &mut buf[..frame_len];

        if frame[0] != HDLC_FLAG {
            return Err(ParseError::BoundaryFlagMissing);
        }

        let format = u16::from_be_bytes([frame[1], frame[2]]);
        if (format & 0xF000) != 0xA000 {
            return Err(ParseError::UnknownData);
        }

        let len = ((format & 0x7FF) as usize) + 2;
        if len > frame_len {
            return Err(ParseError::Incomplete);
        }

        let footer_idx = len - 3;
        if frame[footer_idx + 2] != HDLC_FLAG {
            return Err(ParseError::BoundaryFlagMissing);
        }

        let fcs = u16::from_be_bytes([frame[footer_idx], frame[footer_idx + 1]]);
        if fcs > 0 {
            let crc_region = &frame[1..footer_idx];
            if crc16_x25(crc_region) != fcs {
                return Err(ParseError::FooterChecksumError);
            }
        }

        let mut ptr = 3usize;
        while ptr < frame.len() && (frame[ptr] & 0x01) == 0 {
            ptr += 1;
        }
        if ptr >= frame.len() {
            return Err(ParseError::Fail);
        }
        ptr += 1;
        while ptr < frame.len() && (frame[ptr] & 0x01) == 0 {
            ptr += 1;
        }
        if ptr >= frame.len() {
            return Err(ParseError::Fail);
        }
        ptr += 1;

        if ptr + 3 > footer_idx {
            return Err(ParseError::Incomplete);
        }

        let hcs = u16::from_be_bytes([frame[ptr + 1], frame[ptr + 2]]);
        if hcs > 0 {
            let hdr_for_crc = &frame[1..=ptr];
            if crc16_x25(hdr_for_crc) != hcs {
                return Err(ParseError::HeaderChecksumError);
            }
        }
        ptr += 3;

        let header_len = ptr;
        let rem = frame_len.saturating_sub(header_len);
        let inner_remaining = if rem > 1 { rem - 3 } else { rem };

        let segmented = (format & 0x0800) != 0;

        if segmented {
            let mut p = ptr;
            let mut seg_len = inner_remaining;
            if p < frame_len && frame[p] == LLC_TAG {
                p += 3;
                seg_len = seg_len.saturating_sub(3);
            }
            if seg_len > 1024 || self.segment_buf.len() + seg_len > 1024 {
                self.reset_seg();
                return Err(ParseError::Fail);
            }
            if self.last_sequence == 0 {
                self.segment_buf.clear();
            }
            self.segment_buf.extend_from_slice(&frame[p..p + seg_len]);
            self.last_sequence = self.last_sequence.wrapping_add(1);
            ctx.length = seg_len;
            return Err(ParseError::IntermediateSegment);
        }

        if self.last_sequence > 0 {
            let mut p = ptr;
            let mut seg_len = inner_remaining;
            if p < frame_len && frame[p] == LLC_TAG {
                p += 3;
                seg_len = seg_len.saturating_sub(3);
            }
            if self.segment_buf.len() + seg_len > 1024 {
                self.reset_seg();
                return Err(ParseError::Fail);
            }
            self.segment_buf.extend_from_slice(&frame[p..p + seg_len]);
            let total = self.segment_buf.len();
            if total > frame_len {
                self.reset_seg();
                return Err(ParseError::Fail);
            }
            frame[..total].copy_from_slice(&self.segment_buf[..total]);
            self.segment_buf.clear();
            self.last_sequence = 0;
            ctx.length = total;
            return Ok(header_len);
        }

        self.reset_seg();
        ctx.length = inner_remaining;
        Ok(header_len)
    }

    fn reset_seg(&mut self) {
        self.last_sequence = 0;
        self.segment_buf.clear();
    }
}
