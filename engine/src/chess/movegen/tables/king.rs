use super::attacks;
use crate::chess::{bitboard::Bitboard, square::Square};

static mut ATTACKS_TABLE: [Bitboard; Square::N] = [Bitboard::EMPTY; Square::N];

pub fn king_attacks(s: Square) -> Bitboard {
    unsafe { ATTACKS_TABLE[s] }
}

pub fn init() {
    for s in Bitboard::FULL {
        let attacks = attacks::generate_king_attacks(s);

        unsafe {
            ATTACKS_TABLE[s] = attacks;
        }
    }
}
