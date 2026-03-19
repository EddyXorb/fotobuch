//! Tests for the page CLI lexer and parser.

use super::lexer::tokenize;
use super::parse_api::{parse_move_cmd, parse_pages_expr, parse_split_addr, parse_unplace_addr};
use super::tokens::{ParseError, Token};
use fotobuch::commands::page::{DstMove, PageMoveCmd, PagesExpr, SlotExpr, SlotItem, Src};

// ── tokenize ──────────────────────────────────────────────────────────────────

#[test]
fn test_tokenize_basic() {
    let tokens = tokenize("3:2 to 5").unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Number(3),
            Token::Colon,
            Token::Number(2),
            Token::To,
            Token::Number(5),
        ]
    );
}

#[test]
fn test_tokenize_new_page() {
    let tokens = tokenize("3:2 to 4+").unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Number(3),
            Token::Colon,
            Token::Number(2),
            Token::To,
            Token::Number(4),
            Token::Plus,
        ]
    );
}

#[test]
fn test_tokenize_comma_list() {
    let tokens = tokenize("3,4 to 5").unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Number(3),
            Token::Comma,
            Token::Number(4),
            Token::To,
            Token::Number(5),
        ]
    );
}

#[test]
fn test_tokenize_out() {
    let tokens = tokenize("3 out").unwrap();
    assert_eq!(tokens, vec![Token::Number(3), Token::Out,]);
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

#[test]
fn test_tokenize_unknown_keyword_is_error() {
    assert!(tokenize("3 onto 5").is_err());
}

// ── parse_move_cmd ────────────────────────────────────────────────────────────

#[test]
fn test_parse_slot_to_page() {
    let cmd = parse_move_cmd("3:2 to 5").unwrap();
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
    let cmd = parse_move_cmd("3:1..3,7 to 5").unwrap();
    assert_eq!(
        cmd,
        PageMoveCmd::Move {
            src: Src::Slots {
                page: 3,
                slots: SlotExpr {
                    items: vec![
                        SlotItem::Range { from: Some(1), to: Some(3) },
                        SlotItem::Single(7),
                    ],
                },
            },
            dst: DstMove::Page(5),
        }
    );
}

#[test]
fn test_parse_page_to_page() {
    let cmd = parse_move_cmd("3 to 5").unwrap();
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
    let cmd = parse_move_cmd("3,4 to 5").unwrap();
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
    let cmd = parse_move_cmd("3..5 to 2").unwrap();
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
    let cmd = parse_move_cmd("3:2 to 4+").unwrap();
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
fn test_parse_missing_operator() {
    let err = parse_move_cmd("3").unwrap_err();
    assert_eq!(err, ParseError::MissingOperator);
}

#[test]
fn test_parse_page_unplace() {
    let cmd = parse_move_cmd("3 out").unwrap();
    assert_eq!(
        cmd,
        PageMoveCmd::Move {
            src: Src::Pages(PagesExpr::single(3)),
            dst: DstMove::Unplace,
        }
    );
}

#[test]
fn test_parse_slots_unplace() {
    let cmd = parse_move_cmd("3:4..6 out").unwrap();
    assert_eq!(
        cmd,
        PageMoveCmd::Move {
            src: Src::Slots {
                page: 3,
                slots: SlotExpr::from_range(4, 6),
            },
            dst: DstMove::Unplace,
        }
    );
}

// ── parse_split_addr ──────────────────────────────────────────────────────────

#[test]
fn test_parse_split_addr() {
    let (page, slot) = parse_split_addr("3:4").unwrap();
    assert_eq!(page, 3);
    assert_eq!(slot, 4);
}

// ── parse_unplace_addr ────────────────────────────────────────────────────────

#[test]
fn test_parse_unplace_single_slot() {
    let (page, slots) = parse_unplace_addr("3:2").unwrap();
    assert_eq!(page, 3);
    assert_eq!(slots.items, vec![SlotItem::Single(2)]);
}

#[test]
fn test_parse_unplace_slot_range() {
    let (page, slots) = parse_unplace_addr("3:2..5").unwrap();
    assert_eq!(page, 3);
    assert_eq!(slots.items, vec![SlotItem::Range { from: Some(2), to: Some(5) }]);
}

#[test]
fn test_parse_unplace_combined() {
    let (page, slots) = parse_unplace_addr("3:2..5,7").unwrap();
    assert_eq!(page, 3);
    assert_eq!(slots.items, vec![
        SlotItem::Range { from: Some(2), to: Some(5) },
        SlotItem::Single(7),
    ]);
}

#[test]
fn test_parse_open_end_slot_range() {
    let (page, slots) = parse_unplace_addr("1:2..").unwrap();
    assert_eq!(page, 1);
    assert_eq!(slots.items, vec![SlotItem::Range { from: Some(2), to: None }]);
}

#[test]
fn test_parse_open_start_slot_range() {
    let (page, slots) = parse_unplace_addr("1:..4").unwrap();
    assert_eq!(page, 1);
    assert_eq!(slots.items, vec![SlotItem::Range { from: None, to: Some(4) }]);
}

// ── parse_pages_expr ──────────────────────────────────────────────────────────

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

