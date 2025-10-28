use super::attacks;
use crate::chess::{bitboard::Bitboard, square::Square};

static ATTACKS_TABLE: [Bitboard; Square::N] = const {
    let mut arr = [Bitboard::EMPTY; Square::N];

    let mut bb = Bitboard::FULL;
    while let Some(s) = bb.pop_square_inplace() {
        arr[s] = attacks::generate_king_attacks(s);
    }

    arr
};

#[inline]
pub fn king_attacks(s: Square) -> Bitboard {
    unsafe { ATTACKS_TABLE[s] }
}
