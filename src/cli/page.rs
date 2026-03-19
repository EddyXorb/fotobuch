//! CLI parser for `fotobuch page` and `fotobuch unplace` commands.
//!
//! # Architecture
//!
//! Lexer → Parser → (lib) Validator + Executor
//!
//! This module handles **syntax only** — it never touches the project state.
//! Semantic validation (pages exist, slots exist, …) is done in `commands::page`.

use anyhow::{Context, Result};
use std::path::PathBuf;

use fotobuch::commands::page::{
    self as page_cmd, DstMove, DstSwap, PageMoveCmd, PagesExpr, SlotExpr, Src,
};

// ── Tokens ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(u32),
    Comma,
    Range,  // ".."
    Colon,  // ":"
    Arrow,  // "->"
    Swap,   // "<>"
    Plus,   // "+"
}

// ── ParseError ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken { got: String, expected: &'static str },
    MissingOperator,
    MissingDestination,
    InvalidNumber(String),
    UnexpectedEnd { expected: &'static str },
    UnexpectedChar(char),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedToken { got, expected } => {
                write!(f, "unexpected '{got}', expected {expected}")
            }
            Self::MissingOperator => write!(f, "missing operator ('->' or '<>')"),
            Self::MissingDestination => write!(f, "missing destination after operator"),
            Self::InvalidNumber(s) => write!(f, "invalid number: '{s}'"),
            Self::UnexpectedEnd { expected } => {
                write!(f, "unexpected end of input, expected {expected}")
            }
            Self::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
        }
    }
}

// ── Lexer ─────────────────────────────────────────────────────────────────────

/// Tokenize a raw string into a list of [`Token`]s.
///
/// Whitespace is ignored. `->` and `<>` are recognised as two-character tokens.
pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' => {
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '.' => {
                if i + 1 < chars.len() && chars[i + 1] == '.' {
                    tokens.push(Token::Range);
                    i += 2;
                } else {
                    return Err(ParseError::UnexpectedChar('.'));
                }
            }
            '-' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Arrow);
                    i += 2;
                } else {
                    return Err(ParseError::UnexpectedChar('-'));
                }
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Swap);
                    i += 2;
                } else {
                    return Err(ParseError::UnexpectedChar('<'));
                }
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n: u32 = s
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber(s.clone()))?;
                tokens.push(Token::Number(n));
            }
            c => {
                return Err(ParseError::UnexpectedChar(c));
            }
        }
    }

    Ok(tokens)
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    fn expect_number(&mut self, ctx: &'static str) -> Result<u32, ParseError> {
        match self.advance() {
            Some(Token::Number(n)) => Ok(n),
            Some(t) => Err(ParseError::UnexpectedToken {
                got: format!("{t:?}"),
                expected: ctx,
            }),
            None => Err(ParseError::UnexpectedEnd { expected: ctx }),
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Parse `pages_expr`: `NUMBER` | `NUMBER,..,NUMBER` | `NUMBER..NUMBER`
    ///
    /// This is ambiguous with `src` (which may add `:slot_expr` after the first number).
    /// We parse just the pages part and let `parse_src` handle the colon.
    fn parse_pages_expr(&mut self) -> Result<PagesExpr, ParseError> {
        let first = self.expect_number("page number")?;

        match self.peek() {
            Some(Token::Range) => {
                self.advance(); // consume ".."
                let last = self.expect_number("page number after '..'")?;
                Ok(PagesExpr::from_range(first, last))
            }
            Some(Token::Comma) => {
                let mut pages = vec![first];
                while let Some(Token::Comma) = self.peek() {
                    self.advance(); // consume ","
                    let n = self.expect_number("page number after ','")?;
                    pages.push(n);
                }
                Ok(PagesExpr::from_list(pages))
            }
            _ => Ok(PagesExpr::single(first)),
        }
    }

    /// Parse `slot_expr`: `slot_item ("," slot_item)*`
    /// `slot_item`: `NUMBER | NUMBER ".." NUMBER`
    fn parse_slot_expr(&mut self) -> Result<SlotExpr, ParseError> {
        let mut slots = Vec::new();

        loop {
            let first = self.expect_number("slot number")?;
            if let Some(Token::Range) = self.peek() {
                self.advance(); // consume ".."
                let last = self.expect_number("slot number after '..'")?;
                for s in first..=last {
                    slots.push(s);
                }
            } else {
                slots.push(first);
            }
            if let Some(Token::Comma) = self.peek() {
                self.advance(); // consume ","
            } else {
                break;
            }
        }

        Ok(SlotExpr::from_list(slots))
    }

    /// Parse `src`: `pages_expr | page ":" slot_expr`
    ///
    /// We first parse the page number, then check for `:` (slots) or `,`/`..` (pages).
    fn parse_src(&mut self) -> Result<Src, ParseError> {
        let first = self.expect_number("page number")?;

        match self.peek().cloned() {
            Some(Token::Colon) => {
                self.advance(); // consume ":"
                let slots = self.parse_slot_expr()?;
                Ok(Src::Slots { page: first, slots })
            }
            Some(Token::Range) => {
                self.advance(); // consume ".."
                let last = self.expect_number("page number after '..'")?;
                Ok(Src::Pages(PagesExpr::from_range(first, last)))
            }
            Some(Token::Comma) => {
                let mut pages = vec![first];
                while let Some(Token::Comma) = self.peek() {
                    self.advance(); // consume ","
                    let n = self.expect_number("page number after ','")?;
                    pages.push(n);
                }
                Ok(Src::Pages(PagesExpr::from_list(pages)))
            }
            _ => Ok(Src::Pages(PagesExpr::single(first))),
        }
    }

    /// Parse `dst_move`: `page | page "+"`
    fn parse_dst_move(&mut self) -> Result<DstMove, ParseError> {
        let page = self.expect_number("destination page")?;
        if let Some(Token::Plus) = self.peek() {
            self.advance(); // consume "+"
            Ok(DstMove::NewPageAfter(page))
        } else {
            Ok(DstMove::Page(page))
        }
    }

    /// Parse `dst_swap`: `pages_expr | page ":" slot_expr`
    fn parse_dst_swap(&mut self) -> Result<DstSwap, ParseError> {
        let first = self.expect_number("destination page")?;

        match self.peek().cloned() {
            Some(Token::Colon) => {
                self.advance(); // consume ":"
                let slots = self.parse_slot_expr()?;
                Ok(DstSwap::Slots { page: first, slots })
            }
            Some(Token::Range) => {
                self.advance(); // consume ".."
                let last = self.expect_number("page number after '..'")?;
                Ok(DstSwap::Pages(PagesExpr::from_range(first, last)))
            }
            Some(Token::Comma) => {
                let mut pages = vec![first];
                while let Some(Token::Comma) = self.peek() {
                    self.advance(); // consume ","
                    let n = self.expect_number("page number after ','")?;
                    pages.push(n);
                }
                Ok(DstSwap::Pages(PagesExpr::from_list(pages)))
            }
            _ => Ok(DstSwap::Pages(PagesExpr::single(first))),
        }
    }

    /// Parse a full `page move` expression.
    fn parse_move_cmd(&mut self) -> Result<PageMoveCmd, ParseError> {
        let src = self.parse_src()?;

        match self.advance() {
            Some(Token::Arrow) => {
                if self.is_at_end() {
                    return Err(ParseError::MissingDestination);
                }
                let dst = self.parse_dst_move()?;
                Ok(PageMoveCmd::Move { src, dst })
            }
            Some(Token::Swap) => {
                if self.is_at_end() {
                    return Err(ParseError::MissingDestination);
                }
                let right = self.parse_dst_swap()?;
                Ok(PageMoveCmd::Swap { left: src, right })
            }
            Some(t) => Err(ParseError::UnexpectedToken {
                got: format!("{t:?}"),
                expected: "'->' or '<>'",
            }),
            None => Err(ParseError::MissingOperator),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a `page move` raw argument string (joined from CLI args).
///
/// Examples:
/// - `"3:2 -> 5"`
/// - `"3,4 -> 5"`
/// - `"3:1..3,7 -> 4+"`
/// - `"3:2 <> 5:6"`
/// - `"3 <> 5"`
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
        Some(Token::Colon) => {}
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
        Some(Token::Colon) => {}
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

/// Parse a `page swap` pair: two `"PAGE:SLOT_EXPR"` strings.
pub fn parse_swap_addrs(left: &str, right: &str) -> Result<(Src, DstSwap), ParseError> {
    let left_tokens = tokenize(left)?;
    let mut left_parser = Parser::new(left_tokens);
    let left_src = left_parser.parse_src()?;

    let right_tokens = tokenize(right)?;
    let mut right_parser = Parser::new(right_tokens);
    let right_dst = right_parser.parse_dst_swap()?;

    Ok((left_src, right_dst))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

// ── CLI handlers ──────────────────────────────────────────────────────────────

fn project_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to determine current directory")
}

/// Handler for `fotobuch unplace <address>`.
pub fn handle_unplace(address: &str) -> Result<()> {
    let (page, slots) = parse_unplace_addr(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;
    let result = page_cmd::execute_unplace(&project_root()?, page, slots)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if result.pages_modified.is_empty() {
        println!("Nothing to unplace.");
    } else {
        println!("Unplaced photos from page {}.", page);
    }
    Ok(())
}

/// Handler for `fotobuch page move <args...>`.
pub fn handle_move(args: &[String]) -> Result<()> {
    let raw = args.join(" ");
    let cmd = parse_move_cmd(&raw)
        .map_err(|e| anyhow::anyhow!("Invalid move expression '{}': {}", raw, e))?;
    let result = page_cmd::execute_move(&project_root()?, cmd)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Moved photos. Modified pages: {}",
        format_page_list(&result.pages_modified)
    );
    if !result.pages_inserted.is_empty() {
        println!(
            "Inserted new pages: {}",
            format_page_list(&result.pages_inserted)
        );
    }
    Ok(())
}

/// Handler for `fotobuch page split <address>`.
pub fn handle_split(address: &str) -> Result<()> {
    let (page, slot) = parse_split_addr(address)
        .map_err(|e| anyhow::anyhow!("Invalid split address '{}': {}", address, e))?;
    let result = page_cmd::execute_split(&project_root()?, page, slot)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Split page {} at slot {}. New page inserted after page {}.",
        page,
        slot,
        result.pages_inserted.first().copied().unwrap_or(0)
    );
    Ok(())
}

/// Handler for `fotobuch page combine <pages>`.
pub fn handle_combine(pages_str: &str) -> Result<()> {
    let pages = parse_pages_expr(pages_str)
        .map_err(|e| anyhow::anyhow!("Invalid pages expression '{}': {}", pages_str, e))?;
    let result = page_cmd::execute_combine(&project_root()?, pages)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Combined onto page {}. Deleted pages: {}",
        result.pages_modified.first().copied().unwrap_or(0),
        format_page_list(&result.pages_deleted)
    );
    Ok(())
}

/// Handler for `fotobuch page swap <left> <right>`.
pub fn handle_swap(left: &str, right: &str) -> Result<()> {
    let (left_src, right_dst) = parse_swap_addrs(left, right)
        .map_err(|e| anyhow::anyhow!("Invalid swap addresses '{}' '{}': {}", left, right, e))?;
    let cmd = PageMoveCmd::Swap {
        left: left_src,
        right: right_dst,
    };
    let result = page_cmd::execute_move(&project_root()?, cmd)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Swapped photos. Modified pages: {}",
        format_page_list(&result.pages_modified)
    );
    Ok(())
}

fn format_page_list(pages: &[u32]) -> String {
    let list: Vec<String> = pages.iter().map(|p| p.to_string()).collect();
    list.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use fotobuch::commands::page::*;

    // ── tokenize ──────────────────────────────────────────────────────────────

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("3:2 -> 5").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Number(3),
                Token::Colon,
                Token::Number(2),
                Token::Arrow,
                Token::Number(5),
            ]
        );
    }

    #[test]
    fn test_tokenize_swap() {
        let tokens = tokenize("3:1..3 <> 5:2..4").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Number(3),
                Token::Colon,
                Token::Number(1),
                Token::Range,
                Token::Number(3),
                Token::Swap,
                Token::Number(5),
                Token::Colon,
                Token::Number(2),
                Token::Range,
                Token::Number(4),
            ]
        );
    }

    #[test]
    fn test_tokenize_new_page() {
        let tokens = tokenize("3:2 -> 4+").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Number(3),
                Token::Colon,
                Token::Number(2),
                Token::Arrow,
                Token::Number(4),
                Token::Plus,
            ]
        );
    }

    #[test]
    fn test_tokenize_comma_list() {
        let tokens = tokenize("3,4 -> 5").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Number(3),
                Token::Comma,
                Token::Number(4),
                Token::Arrow,
                Token::Number(5),
            ]
        );
    }

    #[test]
    fn test_tokenize_invalid_char() {
        assert!(tokenize("3!2").is_err());
    }

    #[test]
    fn test_tokenize_single_dot_is_error() {
        assert!(tokenize("3.2").is_err());
    }

    #[test]
    fn test_tokenize_single_dash_is_error() {
        assert!(tokenize("3-2").is_err());
    }

    #[test]
    fn test_tokenize_single_lt_is_error() {
        assert!(tokenize("3<2").is_err());
    }

    // ── parse_move_cmd ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_slot_to_page() {
        let cmd = parse_move_cmd("3:2 -> 5").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Move {
                src: Src::Slots {
                    page: 3,
                    slots: SlotExpr::single(2),
                },
                dst: DstMove::Page(5),
            }
        );
    }

    #[test]
    fn test_parse_range_slots_to_page() {
        let cmd = parse_move_cmd("3:1..3,7 -> 5").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Move {
                src: Src::Slots {
                    page: 3,
                    slots: SlotExpr::from_list(vec![1, 2, 3, 7]),
                },
                dst: DstMove::Page(5),
            }
        );
    }

    #[test]
    fn test_parse_page_to_page() {
        let cmd = parse_move_cmd("3 -> 5").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Move {
                src: Src::Pages(PagesExpr::single(3)),
                dst: DstMove::Page(5),
            }
        );
    }

    #[test]
    fn test_parse_pages_to_page() {
        let cmd = parse_move_cmd("3,4 -> 5").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Move {
                src: Src::Pages(PagesExpr::from_list(vec![3, 4])),
                dst: DstMove::Page(5),
            }
        );
    }

    #[test]
    fn test_parse_page_range_to_page() {
        let cmd = parse_move_cmd("3..5 -> 2").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Move {
                src: Src::Pages(PagesExpr::from_range(3, 5)),
                dst: DstMove::Page(2),
            }
        );
    }

    #[test]
    fn test_parse_slot_to_new_page() {
        let cmd = parse_move_cmd("3:2 -> 4+").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Move {
                src: Src::Slots {
                    page: 3,
                    slots: SlotExpr::single(2),
                },
                dst: DstMove::NewPageAfter(4),
            }
        );
    }

    #[test]
    fn test_parse_swap_slots() {
        let cmd = parse_move_cmd("3:2 <> 5:6").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Swap {
                left: Src::Slots {
                    page: 3,
                    slots: SlotExpr::single(2),
                },
                right: DstSwap::Slots {
                    page: 5,
                    slots: SlotExpr::single(6),
                },
            }
        );
    }

    #[test]
    fn test_parse_swap_pages() {
        let cmd = parse_move_cmd("3 <> 5").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Swap {
                left: Src::Pages(PagesExpr::single(3)),
                right: DstSwap::Pages(PagesExpr::single(5)),
            }
        );
    }

    #[test]
    fn test_parse_swap_ranges() {
        let cmd = parse_move_cmd("3:1..3 <> 5:2..4").unwrap();
        assert_eq!(
            cmd,
            PageMoveCmd::Swap {
                left: Src::Slots {
                    page: 3,
                    slots: SlotExpr::from_range(1, 3),
                },
                right: DstSwap::Slots {
                    page: 5,
                    slots: SlotExpr::from_range(2, 4),
                },
            }
        );
    }

    #[test]
    fn test_parse_missing_operator() {
        let err = parse_move_cmd("3").unwrap_err();
        assert_eq!(err, ParseError::MissingOperator);
    }

    #[test]
    fn test_parse_missing_destination() {
        let err = parse_move_cmd("3 ->").unwrap_err();
        assert_eq!(err, ParseError::MissingDestination);
    }

    // ── parse_split_addr ──────────────────────────────────────────────────────

    #[test]
    fn test_parse_split_addr() {
        let (page, slot) = parse_split_addr("3:4").unwrap();
        assert_eq!(page, 3);
        assert_eq!(slot, 4);
    }

    // ── parse_unplace_addr ────────────────────────────────────────────────────

    #[test]
    fn test_parse_unplace_single_slot() {
        let (page, slots) = parse_unplace_addr("3:2").unwrap();
        assert_eq!(page, 3);
        assert_eq!(slots.slots, vec![2]);
    }

    #[test]
    fn test_parse_unplace_slot_range() {
        let (page, slots) = parse_unplace_addr("3:2..5").unwrap();
        assert_eq!(page, 3);
        assert_eq!(slots.slots, vec![2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_unplace_combined() {
        let (page, slots) = parse_unplace_addr("3:2..5,7").unwrap();
        assert_eq!(page, 3);
        assert_eq!(slots.slots, vec![2, 3, 4, 5, 7]);
    }

    // ── parse_pages_expr ──────────────────────────────────────────────────────

    #[test]
    fn test_parse_pages_single() {
        let pe = parse_pages_expr("5").unwrap();
        assert_eq!(pe.pages, vec![5]);
    }

    #[test]
    fn test_parse_pages_list() {
        let pe = parse_pages_expr("3,5").unwrap();
        assert_eq!(pe.pages, vec![3, 5]);
    }

    #[test]
    fn test_parse_pages_range() {
        let pe = parse_pages_expr("3..5").unwrap();
        assert_eq!(pe.pages, vec![3, 4, 5]);
    }
}
