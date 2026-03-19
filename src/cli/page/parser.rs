//! Recursive-descent parser: tokens → AST (PageMoveCmd / addresses).

use fotobuch::commands::page::{DstMove, DstSwap, PageMoveCmd, PagesExpr, SlotExpr, Src};

use super::tokens::{ParseError, Token};

pub struct Parser {
    pub tokens: Vec<Token>,
    pub pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    pub fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    pub fn expect_number(&mut self, ctx: &'static str) -> Result<u32, ParseError> {
        match self.advance() {
            Some(Token::Number(n)) => Ok(n),
            Some(t) => Err(ParseError::UnexpectedToken {
                got: format!("{t:?}"),
                expected: ctx,
            }),
            None => Err(ParseError::UnexpectedEnd { expected: ctx }),
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Parse `pages_expr`: `NUMBER | NUMBER ".." NUMBER | NUMBER ("," NUMBER)*`
    pub fn parse_pages_expr(&mut self) -> Result<PagesExpr, ParseError> {
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
    /// where `slot_item` is `NUMBER` or `NUMBER ".." NUMBER`.
    pub fn parse_slot_expr(&mut self) -> Result<SlotExpr, ParseError> {
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
    pub fn parse_src(&mut self) -> Result<Src, ParseError> {
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
    pub fn parse_dst_move(&mut self) -> Result<DstMove, ParseError> {
        let page = self.expect_number("destination page")?;
        if let Some(Token::Plus) = self.peek() {
            self.advance(); // consume "+"
            Ok(DstMove::NewPageAfter(page))
        } else {
            Ok(DstMove::Page(page))
        }
    }

    /// Parse `dst_swap`: `pages_expr | page ":" slot_expr`
    pub fn parse_dst_swap(&mut self) -> Result<DstSwap, ParseError> {
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
    pub fn parse_move_cmd(&mut self) -> Result<PageMoveCmd, ParseError> {
        let src = self.parse_src()?;

        match self.advance() {
            Some(Token::Arrow) => {
                if self.is_at_end() {
                    return Ok(PageMoveCmd::Move { src, dst: DstMove::Unplace });
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
