use crate::chess::{
    bitboard::Bitboard, board::Board, movegen::tables, player::Player, square::Square,
};

pub fn all_attackers_of(board: &Board, square: Square, occupied: Bitboard) -> Bitboard {
    use Player::*;

    let mut attackers = Bitboard::EMPTY;

    attackers |= tables::pawn_attacks(square, White) & board.pawns(Black);
    attackers |= tables::pawn_attacks(square, Black) & board.pawns(White);

    attackers |= tables::knight_attacks(square) & board.all_knights();
    attackers |= tables::bishop_attacks(square, occupied) & board.all_diagonal_sliders();
    attackers |= tables::rook_attacks(square, occupied) & board.all_orthogonal_sliders();
    attackers |= tables::king_attacks(square) & board.all_kings();

    attackers
}
