//! Public parsing functions — the API surface of the page CLI parser.

use fotobuch::commands::page::{DstSwap, PageMoveCmd, PagesExpr, SlotExpr, Src, WeightAddress};

use super::lexer::tokenize;
use super::parser::Parser;
use super::tokens::ParseError;

/// Parse a `page move` raw argument string (joined from CLI args).
///
/// Examples:
/// - `"3:2 to 5"`
/// - `"3,4 to 5"`
/// - `"3:1..3,7 to 4+"`
/// - `"3:2 ~ 5:6"`
/// - `"3 ~ 5"`
/// - `"3 out"`
pub fn parse_move_cmd(raw: &str) -> Result<PageMoveCmd, ParseError> {
    let tokens = tokenize(raw)?;
    let mut parser = Parser::new(tokens);
    let cmd = parser.parse_move_cmd()?;
    if !parser.is_at_end() {
        let rest: Vec<_> = parser.tokens[parser.pos..].to_vec();
        return Err(ParseError::UnexpectedToken {
            got: format!("{rest:?}"),
            expected: "end of input",
        });
    }
    Ok(cmd)
}

/// Parse a `page split` address: `"PAGE:SLOT"`.
pub fn parse_split_addr(raw: &str) -> Result<(u32, u32), ParseError> {
    let tokens = tokenize(raw)?;
    let mut parser = Parser::new(tokens);
    let page = parser.expect_number("page number")?;
    match parser.advance() {
        Some(super::tokens::Token::Colon) => {}
        Some(t) => {
            return Err(ParseError::UnexpectedToken {
                got: format!("{t:?}"),
                expected: "':'",
            });
        }
        None => return Err(ParseError::UnexpectedEnd { expected: "':'" }),
    }
    let slot = parser.expect_number("slot number")?;
    Ok((page, slot))
}

/// Parse a `page combine` / `unplace` pages expression: `"3"`, `"3,5"`, `"3..5"`.
pub fn parse_pages_expr(raw: &str) -> Result<PagesExpr, ParseError> {
    let tokens = tokenize(raw)?;
    let mut parser = Parser::new(tokens);
    parser.parse_pages_expr()
}

/// Parse an `unplace` address: `"PAGE:SLOT_EXPR"`.
pub fn parse_unplace_addr(raw: &str) -> Result<(u32, SlotExpr), ParseError> {
    let tokens = tokenize(raw)?;
    let mut parser = Parser::new(tokens);
    let page = parser.expect_number("page number")?;
    match parser.advance() {
        Some(super::tokens::Token::Colon) => {}
        Some(t) => {
            return Err(ParseError::UnexpectedToken {
                got: format!("{t:?}"),
                expected: "':'",
            });
        }
        None => return Err(ParseError::UnexpectedEnd { expected: "':'" }),
    }
    let slots = parser.parse_slot_expr()?;
    Ok((page, slots))
}

/// Parse a `page info` address — same grammar as `src`.
pub fn parse_info_address(raw: &str) -> Result<Src, ParseError> {
    let tokens = tokenize(raw)?;
    let mut parser = Parser::new(tokens);
    parser.parse_src()
}

/// Parse a `page weight` address: `"PAGE"` or `"PAGE:SLOT_EXPR"`.
pub fn parse_weight_address(raw: &str) -> Result<WeightAddress, ParseError> {
    let tokens = tokenize(raw)?;
    let mut parser = Parser::new(tokens);
    let page = parser.expect_number("page number")?;
    if parser.is_at_end() {
        return Ok(WeightAddress::Page(page));
    }
    match parser.advance() {
        Some(super::tokens::Token::Colon) => {}
        Some(t) => {
            return Err(ParseError::UnexpectedToken {
                got: format!("{t:?}"),
                expected: "':'",
            });
        }
        None => return Err(ParseError::UnexpectedEnd { expected: "':'" }),
    }
    let slots = parser.parse_slot_expr()?;
    Ok(WeightAddress::Slots { page, slots })
}

/// Parse a `page pos` address: `"PAGE:SLOT_EXPR"` (e.g. `"4:2"`, `"4:2..5"`).
pub fn parse_pos_address(raw: &str) -> Result<(u32, SlotExpr), ParseError> {
    let tokens = tokenize(raw)?;
    let mut parser = Parser::new(tokens);
    let page = parser.expect_number("page number")?;
    match parser.advance() {
        Some(super::tokens::Token::Colon) => {}
        Some(t) => {
            return Err(ParseError::UnexpectedToken {
                got: format!("{t:?}"),
                expected: "':'",
            });
        }
        None => return Err(ParseError::UnexpectedEnd { expected: "':'" }),
    }
    let slots = parser.parse_slot_expr()?;
    Ok((page, slots))
}

/// Parse a `page swap` pair: two address strings each being `src` / `dst_swap`.
pub fn parse_swap_addrs(left: &str, right: &str) -> Result<(Src, DstSwap), ParseError> {
    let left_tokens = tokenize(left)?;
    let mut left_parser = Parser::new(left_tokens);
    let left_src = left_parser.parse_src()?;

    let right_tokens = tokenize(right)?;
    let mut right_parser = Parser::new(right_tokens);
    let right_dst = right_parser.parse_dst_swap()?;

    Ok((left_src, right_dst))
}
