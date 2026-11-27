use crate::{chess::prelude::*, engine::eval::nnue::nnue::should_mirror};

const PLAYER_STRIDE: usize = Square::N * PieceKind::N;
const PIECE_STRIDE: usize = Square::N;

pub fn psqt_feature_index((piece, sq): (Piece, Square), king: Square, pov: Player) -> usize {
    let piece_index = piece.kind as usize;
    let player_index = piece.player as usize ^ pov as usize;
    let square_idx = sq.relative_for(pov).mirrored_horizontally_if(should_mirror(king)).idx();

    player_index * PLAYER_STRIDE + piece_index * PIECE_STRIDE + square_idx
}
