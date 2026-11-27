use crate::{chess::prelude::*, engine::eval::nnue::NNUE};

pub mod psqt;
pub mod threats;

pub fn apply_updates(nnue: &mut NNUE, game: &Game) {
    psqt::updates::apply_updates(nnue, game);
    threats::updates::apply_updates(nnue, game);
}
