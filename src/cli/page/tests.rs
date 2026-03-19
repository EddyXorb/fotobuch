//! Tests for the page CLI lexer and parser.

use super::lexer::tokenize;
use super::parse_api::{parse_move_cmd, parse_pages_expr, parse_split_addr, parse_unplace_addr};
use super::tokens::{ParseError, Token};
use fotobuch::commands::page::*;

// ── tokenize ──────────────────────────────────────────────────────────────────

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

// ── parse_move_cmd ────────────────────────────────────────────────────────────

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
