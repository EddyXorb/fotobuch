//! `fotobuch page` and `fotobuch unplace` commands.
//!
//! See the design document at `docs/design/cli/page.md`.

mod combine;
mod helpers;
mod move_cmd;
mod split;
mod types;
mod unplace;

#[cfg(test)]
mod test_fixtures;

pub use combine::execute_combine;
pub use move_cmd::execute_move;
pub use split::execute_split;
pub use types::{
    DstMove, DstSwap, PageMoveCmd, PageMoveError, PageMoveResult, PagesExpr, SlotExpr, Src,
    ValidationError,
};
pub use unplace::execute_unplace;
