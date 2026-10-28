use crate::chess::{moves::attacks, prelude::*};

static ATTACKS_TABLE: [Bitboard; Square::N] = const {
    let mut attacks = [Bitboard::EMPTY; Square::N];

    let mut s = 0;
    while s < Square::N {
        attacks[s] = attacks::generate_king_attacks(Square::from_index(s as u8));
        s += 1;
    }

    attacks
};

pub const fn lookup_king_attacks(s: Square) -> Bitboard {
    ATTACKS_TABLE[s.array_idx()]
}
