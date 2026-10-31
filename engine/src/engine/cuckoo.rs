use std::sync::OnceLock;

use crate::chess::{
    moves::{bishop_attacks, king_attacks, knight_attacks, rook_attacks},
    prelude::*,
    rays::ray_between,
    zobrist,
    zobrist::ZobristHash,
};

const SIZE: usize = 8192;

static mut INIT: OnceLock<()> = OnceLock::new();

// Safety: These tables are written exactly once during startup, and only read thereafter.
static mut CUCKOO_KEYS: [ZobristHash; SIZE] = [ZobristHash(0u64); SIZE];
static mut CUCKOO_MOVES: [Option<Move>; SIZE] = [None; SIZE];

pub fn has_upcoming_repetition(game: &Game, ply: u8) -> bool {
    let end = (game.halfmove_clock as usize).min(game.history.len());

    if end < 3 {
        return false;
    }

    let prev_key = |p: usize| game.history[game.history.len() - p].hash;

    let occupancy = game.board.occupancy();
    let current_key = game.hash;

    let mut other = current_key ^ prev_key(1) ^ zobrist::side_to_play();

    for i in (3..=end).step_by(2) {
        other ^= prev_key(i - 1) ^ prev_key(i) ^ zobrist::side_to_play();

        if other.0 != 0 {
            continue;
        }

        let diff = current_key ^ prev_key(i);

        let mut slot = h1(diff);

        if keys(slot) != diff {
            slot = h2(diff);

            if keys(slot) != diff {
                continue;
            }
        }

        let mv = moves(slot);
        let ray = ray_between(mv.from(), mv.to());

        if !(ray & occupancy).is_empty() {
            continue;
        }

        // In-tree twofold
        if ply as usize >= i {
            return true;
        }

        // Threefold
        if (i + 2..=end).step_by(2).any(|j| prev_key(j) == prev_key(i)) {
            return true;
        }
    }

    false
}

pub const fn h1(key: ZobristHash) -> usize {
    ((key.0 >> 32) & 0x1FFF) as usize
}

pub const fn h2(key: ZobristHash) -> usize {
    ((key.0 >> 48) & 0x1FFF) as usize
}

fn keys(idx: usize) -> ZobristHash {
    unsafe { CUCKOO_KEYS[idx] }
}

fn moves(idx: usize) -> Move {
    unsafe { CUCKOO_MOVES[idx].unwrap() }
}

fn init_table() {
    let mut count = 0;

    for &piece in PieceKind::ALL.iter().filter(|&&piece| piece != Pawn) {
        for player in Player::ALL {
            for from in Bitboard::FULL {
                let attacks = match piece {
                    Knight => knight_attacks(from),
                    Bishop => bishop_attacks(from, Bitboard::EMPTY),
                    Rook => rook_attacks(from, Bitboard::EMPTY),
                    Queen => {
                        bishop_attacks(from, Bitboard::EMPTY) | rook_attacks(from, Bitboard::EMPTY)
                    }
                    King => king_attacks(from),
                    Pawn => unreachable!(),
                };

                for to_idx in (from.idx() + 1)..64 {
                    let to = Square::from_index(to_idx);

                    if !attacks.contains(to) {
                        continue;
                    }

                    let mut key = ZobristHash(
                        zobrist::piece_on_square(player, piece, from)
                            ^ zobrist::piece_on_square(player, piece, to)
                            ^ zobrist::side_to_play(),
                    );

                    let mut mv = Some(Move::quiet(from, to));
                    let mut slot = h1(key);

                    loop {
                        unsafe {
                            std::mem::swap(&mut CUCKOO_KEYS[slot], &mut key);
                            std::mem::swap(&mut CUCKOO_MOVES[slot], &mut mv);
                        }

                        if mv.is_none() {
                            break;
                        }

                        slot = if slot == h1(key) { h2(key) } else { h1(key) };
                    }

                    count += 1;
                }
            }
        }
    }

    assert_eq!(count, 3668);
}

pub fn init() {
    unsafe {
        INIT.get_or_init(init_table);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::moves::MoveListExt;

    #[test]
    // Ported verbatim from Viridithas
    fn upcoming_repetition_simple() {
        init();

        let mut game = Game::from_valid_fen("4k3/8/8/8/8/8/8/R3K3 w - - 0 1");

        for (from, to) in [(A1, A3), (E8, D8), (A3, A2), (D8, E7), (A2, A1)] {
            game.make_move(game.moves().expect_matching(from, to));
        }

        // rook is back on a1 and we could repeat with e7e8
        assert!(has_upcoming_repetition(&game, 5));
        assert!(!has_upcoming_repetition(&game, 4));
    }

    #[test]
    // Ported verbatim from Viridithas
    fn upcoming_repetition_threefold() {
        init();

        // 1. Nf3 Nc6 2. Ng1: black can play Nb8.
        let mut game = Game::new();
        for (from, to) in [(G1, F3), (B8, C6), (F3, G1)] {
            game.make_move(game.moves().expect_matching(from, to));
        }

        // If we include the root in the tree window,
        // Nb8 gets us a valid twofold.
        assert!(has_upcoming_repetition(&game, 3));

        // If we don’t, then it’s not valid, as out-of-tree
        // repetitions must be threefolds.
        assert!(!has_upcoming_repetition(&game, 2));

        // Shuffle some more, s.t. the move *would* be
        // a threefold:
        for (from, to) in [(C6, B8), (G1, F3), (B8, C6), (F3, G1)] {
            game.make_move(game.moves().expect_matching(from, to));
        }

        // yay
        assert!(has_upcoming_repetition(&game, 2));
    }
}
