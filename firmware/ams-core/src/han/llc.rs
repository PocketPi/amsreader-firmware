//! LLC header strip (`LlcParser.cpp`).

use super::context::{ParseError, ParserContext};

pub fn parse(_buf: &mut [u8], ctx: &mut ParserContext) -> Result<usize, ParseError> {
    if ctx.length < 3 {
        return Err(ParseError::Incomplete);
    }
    ctx.length -= 3;
    Ok(3)
}
