use crate::chess::{moves::attacks, prelude::*};

static mut ATTACKS_TABLE: [[Bitboard; Square::N]; Player::N] =
    [[Bitboard::EMPTY; Square::N]; Player::N];

pub fn lookup_pawn_attacks(s: Square, player: Player) -> Bitboard {
    unsafe { ATTACKS_TABLE[player][s] }
}

pub fn init() {
    for s in Bitboard::FULL {
        let white_attacks = attacks::generate_pawn_attacks(s, White);
        let black_attacks = attacks::generate_pawn_attacks(s, Black);

        unsafe {
            ATTACKS_TABLE[White][s] = white_attacks;
            ATTACKS_TABLE[Black][s] = black_attacks;
        }
    }
}
