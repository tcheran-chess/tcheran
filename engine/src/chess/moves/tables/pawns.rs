use crate::chess::{moves::attacks, prelude::*};

const ATTACKS_TABLE: [[Bitboard; Square::N]; Player::N] = const {
    let mut attacks = [[Bitboard::EMPTY; Square::N]; Player::N];

    for s in Bitboard::FULL {
        attacks[White][s] = attacks::generate_pawn_attacks(s, White);
        attacks[Black][s] = attacks::generate_pawn_attacks(s, Black);
    }

    attacks
};

pub fn lookup_pawn_attacks(s: Square, player: Player) -> Bitboard {
    ATTACKS_TABLE[player][s]
}
