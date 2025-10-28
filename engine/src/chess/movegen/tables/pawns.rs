use super::attacks;
use crate::chess::{bitboard::Bitboard, player::Player, square::Square};

static ATTACKS_TABLE: [[Bitboard; Square::N]; Player::N] = const {
    let mut arr = [[Bitboard::EMPTY; Square::N]; Player::N];

    let mut bb = Bitboard::FULL;
    while let Some(s) = bb.pop_square_inplace() {
        arr[Player::White][s] = attacks::generate_pawn_attacks(s, Player::White);
        arr[Player::Black][s] = attacks::generate_pawn_attacks(s, Player::Black);
    }

    arr
};

#[inline]
pub fn pawn_attacks(s: Square, player: Player) -> Bitboard {
    let a = unsafe { ATTACKS_TABLE[player][s] };
    a
}
