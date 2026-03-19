//! CLI parser for `fotobuch page` and `fotobuch unplace` commands.
//!
//! # Architecture
//!
//! Lexer → Parser → (lib) Validator + Executor
//!
//! This module handles **syntax only** — it never touches the project state.
//! Semantic validation (pages exist, slots exist, …) is done in `commands::page`.

mod handlers;
mod lexer;
mod parse_api;
mod parser;
mod tokens;

#[cfg(test)]
mod tests;

pub use handlers::{handle_combine, handle_move, handle_split, handle_swap, handle_unplace};
pub use parse_api::{
    parse_move_cmd, parse_pages_expr, parse_split_addr, parse_swap_addrs, parse_unplace_addr,
};
pub use tokens::{ParseError, Token};
