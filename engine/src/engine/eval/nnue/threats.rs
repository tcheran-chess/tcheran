use std::sync::OnceLock;

use crate::{
    chess::{
        moves::{bishop_attacks, knight_attacks, pawn_attacks, rook_attacks},
        prelude::*,
    },
    engine::eval::nnue::network::THREAT_INPUTS,
};

const PAWN_FEATURES: usize = 6 * 84;

struct Threats {
    pawn_map: [usize; 12],
    non_pawn_data: [PieceThreatsData; 4],
    offsets: [usize; 5],
}

struct PieceThreatsData {
    piece: PieceKind,
    attacks: [u64; Square::N],
    indices: [usize; Square::N],
    targets: [usize; 12],
    count: usize,
}

impl Threats {
    fn new() -> Self {
        let knight = PieceThreatsData::new(Knight, [Pawn, Knight, Bishop, Rook, Queen], |sq| knight_attacks(sq).raw());
        let bishop =
            PieceThreatsData::new(Bishop, [Pawn, Knight, Bishop, Rook], |sq| bishop_attacks(sq, Bitboard::EMPTY).raw());
        let rook =
            PieceThreatsData::new(Rook, [Pawn, Knight, Bishop, Rook], |sq| rook_attacks(sq, Bitboard::EMPTY).raw());
        let queen = PieceThreatsData::new(Queen, [Pawn, Knight, Bishop, Rook, Queen], |sq| {
            (bishop_attacks(sq, Bitboard::EMPTY) | rook_attacks(sq, Bitboard::EMPTY)).raw()
        });

        let mut offsets = [PAWN_FEATURES; 5];
        for (i, count) in [10 * knight.count, 8 * bishop.count, 8 * rook.count, 10 * queen.count]
            .into_iter()
            .enumerate()
        {
            offsets[i + 1] = offsets[i] + count;
        }

        debug_assert_eq!(offsets[4] * 2, THREAT_INPUTS);

        Self {
            pawn_map: make_targets([Pawn, Knight, Rook]),
            non_pawn_data: [knight, bishop, rook, queen],
            offsets,
        }
    }

    fn map(&self, board: &Board, pov: Player, mut on_feature: impl FnMut(usize)) {
        let king = board.king_square(pov);
        let mask = (if pov == Black { 56 } else { 0 }) ^ (if king.file() >= File::E { 7 } else { 0 });
        let occupied = board.occupancy();

        for side in Player::ALL {
            let side_offset = self.offsets[4] * (side as usize ^ pov as usize);
            for piece in [Pawn, Knight, Bishop, Rook, Queen] {
                for source in board.pieces_of_kind(piece, side) {
                    let attack_map = match piece {
                        Pawn => pawn_attacks(source, side),
                        Knight => knight_attacks(source),
                        Bishop => bishop_attacks(source, occupied),
                        Rook => rook_attacks(source, occupied),
                        Queen => bishop_attacks(source, occupied) | rook_attacks(source, occupied),
                        King => unreachable!(),
                    }
                    .and(occupied);

                    for destination in attack_map {
                        let target = board.piece_guaranteed_at(destination);
                        let target_idx = (target.player as usize ^ pov as usize) * PieceKind::N + target.kind as usize;
                        let src = source.idx() ^ mask;
                        let dest = destination.idx() ^ mask;

                        if let Some(feature) = self.map_single(piece, src, dest, target_idx) {
                            on_feature(side_offset + feature);
                        }
                    }
                }
            }
        }
    }

    fn map_single(&self, piece: PieceKind, src: usize, dest: usize, target: usize) -> Option<usize> {
        if piece == Pawn {
            let target_class = self.pawn_map[target];
            if target_class == usize::MAX {
                return None;
            }

            let expected_delta = if dest > src { 7 } else { 9 };
            let id = if dest.abs_diff(src) == expected_delta { 0 } else { 1 };
            let attack = 2 * (src % 8) + id - 1;
            return Some(target_class * 84 + (src / 8 - 1) * 14 + attack);
        }

        let data_idx = piece as usize - 1;
        self.non_pawn_data[data_idx].map(src, dest, target, self.offsets[data_idx])
    }
}

impl PieceThreatsData {
    fn new<const N: usize>(piece: PieceKind, valid: [PieceKind; N], attacks_for: impl Fn(Square) -> u64) -> Self {
        let mut attacks = [0; Square::N];
        let mut indices = [0; Square::N];
        let mut count = 0;

        for idx in 0..Square::N {
            let square = Square::from_index(idx);
            attacks[idx] = attacks_for(square);
            indices[idx] = count;
            count += attacks[idx].count_ones() as usize;
        }

        Self {
            piece,
            attacks,
            indices,
            targets: make_targets(valid),
            count,
        }
    }

    fn map(&self, src: usize, dest: usize, target: usize, offset: usize) -> Option<usize> {
        let target_kind = target % PieceKind::N;
        if self.targets[target] == usize::MAX || (dest > src && target_kind == self.piece as usize) {
            return None;
        }

        let idx = self.indices[src] + (self.attacks[src] & ((1_u64 << dest) - 1)).count_ones() as usize;
        Some(offset + self.targets[target] * self.count + idx)
    }
}

fn make_targets<const N: usize>(valid: [PieceKind; N]) -> [usize; 12] {
    let mut targets = [usize::MAX; 12];
    for (index, piece) in valid.into_iter().enumerate() {
        targets[piece as usize] = index;
        targets[PieceKind::N + piece as usize] = index + N;
    }
    targets
}

pub fn map_features(board: &Board, pov: Player, on_feature: impl FnMut(usize)) {
    static THREATS: OnceLock<Threats> = OnceLock::new();
    THREATS.get_or_init(Threats::new).map(board, pov, on_feature);
}
