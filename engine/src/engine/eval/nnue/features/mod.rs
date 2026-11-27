use crate::{chess::prelude::*, engine::eval::nnue::NNUE};

pub mod psqt;

pub fn apply_updates(nnue: &mut NNUE, game: &Game) {
    psqt::updates::apply_updates(nnue, game);
}
