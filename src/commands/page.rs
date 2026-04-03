//! `fotobuch page` subcommands (move, split, combine, swap, info, weight, mode).
//!
//! See the design document at `docs/design/cli/page.md`.

mod combine;
mod helpers;
mod info;
mod mode;
mod move_cmd;
mod pos;
mod split;
mod types;
mod weight;

#[cfg(test)]
pub(crate) mod test_fixtures;

pub use combine::execute_combine;
pub(crate) use helpers::{delete_empty_pages, page_idx, remove_slots, resolve_slots};
pub use info::execute_info;
pub use mode::{PageModeResult, execute_mode};
pub use pos::{PosConfig, PosMode, PosResult, SlotChange, execute_pos};
pub use move_cmd::execute_move;
pub use split::execute_split;
pub use types::{
    DstMove, DstSwap, InfoFilter, PageInfoResult, PageMoveCmd, PageMoveError, PageMoveResult,
    PagesExpr, SlotExpr, SlotInfo, SlotItem, Src, ValidationError, WeightAddress,
};
pub use weight::execute_weight;
