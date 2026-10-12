use crate::chess::{moves::attacks, prelude::*};

static ATTACKS_TABLE: [[Bitboard; Square::N]; Player::N] = const {
    let mut attacks = [[Bitboard::EMPTY; Square::N]; Player::N];

    let mut s = 0;
    while s < Square::N {
        attacks[White.idx()][s] =
            attacks::generate_pawn_attacks(Square::from_index(s as u8), White);
        attacks[Black.idx()][s] =
            attacks::generate_pawn_attacks(Square::from_index(s as u8), Black);
        s += 1;
    }

    attacks
};

pub fn lookup_pawn_attacks(s: Square, player: Player) -> Bitboard {
    ATTACKS_TABLE[player][s]
}
