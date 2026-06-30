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
    let end = (game.halfmove_clock as usize).min(game.history.len().saturating_sub(1));

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

        if ply as usize > i {
            return true;
        }

        let from = mv.from();
        let to = mv.to();

        let mut target_sq = from;
        if game.board.piece_at(from).is_none() {
            target_sq = to;
        }

        return game.board.occupancy_for(game.player).contains(target_sq);
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
