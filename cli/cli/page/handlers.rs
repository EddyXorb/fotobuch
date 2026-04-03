//! CLI entry-point handlers for page and unplace subcommands.
//!
//! Each subcommand lives in its own module under `handlers/`.

mod combine;
mod common;
mod info;
mod mode;
mod r#move;
mod pos;
mod split;
mod unplace;
mod weight;

pub use combine::handle_combine;
pub use info::handle_info;
pub use mode::handle_mode;
pub use r#move::{handle_move, handle_swap};
pub use pos::handle_pos;
pub use split::handle_split;
pub use unplace::handle_unplace;
pub use weight::handle_weight;
