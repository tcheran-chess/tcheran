use crate::chess::{Bitboard, Square, moves::attacks};

static mut ATTACKS_TABLE: [Bitboard; Square::N] = [Bitboard::EMPTY; Square::N];

pub fn lookup_king_attacks(s: Square) -> Bitboard {
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
