use crate::chess::{moves::attacks, prelude::*};

const ATTACKS_TABLE: [Bitboard; Square::N] = const {
    let mut attacks = [Bitboard::EMPTY; Square::N];

    for s in Bitboard::FULL {
        attacks[s] = attacks::generate_king_attacks(s);
    }

    attacks
};

pub fn lookup_king_attacks(s: Square) -> Bitboard {
    ATTACKS_TABLE[s]
}
