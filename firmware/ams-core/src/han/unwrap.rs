//! Stack unwrap matching `PassiveMeterCommunicator::unwrapData`.

use super::context::{DataTag, ParseError, ParserContext};
use super::dlms;
use super::dsmr;
use super::gbt::GbtParser;
use super::hdlc::HdlcParser;
use super::llc;
use super::mbus::MbusParser;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnwrapResult {
    Incomplete,
    Intermediate,
    Complete {
        frame_type: u8,
        wire_len: usize,
        /// Index into the original buffer where `payload_len` bytes of application data start.
        payload_offset: usize,
        payload_len: usize,
    },
    Error(ParseError),
}

/// Receive buffer + first UART flush flag (matches `PassiveMeterCommunicator::loop`).
pub struct UnwrapState {
    pub buf: Vec<u8>,
    cap: usize,
    pub serial_flushed: bool,
}

impl UnwrapState {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            cap: capacity,
            serial_flushed: false,
        }
    }

    pub fn push(&mut self, b: u8) -> Result<(), ()> {
        if self.buf.len() >= self.cap {
            return Err(());
        }
        self.buf.push(b);
        Ok(())
    }
}

/// Try to unwrap `buf` in place. On success, payload is `buf[..payload_len]`.
pub fn try_unwrap_han(
    buf: &mut Vec<u8>,
    ctx: &mut ParserContext,
    hdlc: &mut HdlcParser,
    mbus: &mut MbusParser,
    gbt: &mut GbtParser,
) -> UnwrapResult {
    if buf.is_empty() {
        return UnwrapResult::Incomplete;
    }

    let wire_total = buf.len();
    let mut off = 0usize;
    ctx.length = buf.len();
    let mut last_tag = DataTag::None as u8;

    loop {
        if off >= buf.len() {
            return UnwrapResult::Error(ParseError::UnknownData);
        }

        let layer_start = off;
        let layer_end = layer_start + ctx.length;
        if layer_end > buf.len() {
            return UnwrapResult::Incomplete;
        }

        let tag = buf[layer_start];
        let cur_len_before = ctx.length;
        let mut do_ret = false;

        let slice = &mut buf[layer_start..layer_end];
        if slice.len() != ctx.length {
            return UnwrapResult::Incomplete;
        }

        let step: Result<i32, ParseError> = match tag {
            t if t == DataTag::Hdlc as u8 => match hdlc.parse(slice, ctx) {
                Ok(header) => Ok(header as i32),
                Err(ParseError::IntermediateSegment) => return UnwrapResult::Intermediate,
                Err(e) => return UnwrapResult::Error(e),
            },
            t if t == DataTag::Mbus as u8 => match mbus.parse(slice, ctx) {
                Ok(header) => Ok(header as i32),
                Err(ParseError::IntermediateSegment) => return UnwrapResult::Intermediate,
                Err(ParseError::FinalSegment) => {
                    mbus.write(slice, ctx);
                    Ok(0)
                }
                Err(e) => return UnwrapResult::Error(e),
            },
            t if t == DataTag::Gbt as u8 => match gbt.parse(slice, ctx) {
                Ok(advance) => Ok(advance as i32),
                Err(ParseError::IntermediateSegment) => return UnwrapResult::Intermediate,
                Err(e) => return UnwrapResult::Error(e),
            },
            t if t == DataTag::Gcm as u8 => return UnwrapResult::Error(ParseError::UnknownData),
            t if t == DataTag::Llc as u8 => llc::parse(slice, ctx).map(|n| n as i32),
            t if t == DataTag::Dlms as u8 => {
                do_ret = true;
                dlms::parse(slice, ctx).map(|n| n as i32)
            }
            t if t == DataTag::Dsmr as u8 => {
                let verified = last_tag != DataTag::None as u8;
                match dsmr::parse(slice, ctx.length, verified) {
                    Ok(end_line) => {
                        ctx.length = end_line;
                        do_ret = true;
                        Ok(0)
                    }
                    Err(ParseError::Incomplete) => return UnwrapResult::Incomplete,
                    Err(e) => return UnwrapResult::Error(e),
                }
            }
            t if t == DataTag::Snrm as u8 || t == DataTag::Aare as u8 || t == DataTag::Res as u8 => {
                do_ret = true;
                Ok(0)
            }
            _ => return UnwrapResult::Error(ParseError::UnknownData),
        };

        let res = match step {
            Ok(v) => v,
            Err(ParseError::Incomplete) => return UnwrapResult::Incomplete,
            Err(e) => return UnwrapResult::Error(e),
        };

        last_tag = tag;

        if ctx.length > cur_len_before {
            ctx.frame_type = 0;
            ctx.length = 0;
            return UnwrapResult::Error(ParseError::Fail);
        }

        if do_ret {
            return UnwrapResult::Complete {
                frame_type: tag,
                wire_len: wire_total,
                payload_offset: layer_start,
                payload_len: ctx.length,
            };
        }

        let advance = match tag {
            t if t == DataTag::Gbt as u8 => 0usize,
            _ => res as usize,
        };

        off = layer_start + advance;
        ctx.length = layer_end.saturating_sub(off);

        if off > buf.len() {
            return UnwrapResult::Error(ParseError::Fail);
        }
    }
}
