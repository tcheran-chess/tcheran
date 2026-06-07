use crate::chess::{Bitboard, Player, Square, moves::attacks};

static mut ATTACKS_TABLE: [[Bitboard; Square::N]; Player::N] =
    [[Bitboard::EMPTY; Square::N]; Player::N];

pub fn lookup_pawn_attacks(s: Square, player: Player) -> Bitboard {
    unsafe { ATTACKS_TABLE[player][s] }
}

pub fn init() {
    for s in Bitboard::FULL {
        let white_attacks = attacks::generate_pawn_attacks(s, Player::White);
        let black_attacks = attacks::generate_pawn_attacks(s, Player::Black);

        unsafe {
            ATTACKS_TABLE[Player::White][s] = white_attacks;
            ATTACKS_TABLE[Player::Black][s] = black_attacks;
        }
    }
}
