use crate::{
    chess::{
        board::Board,
        piece::{Piece, PieceKind},
        player::Player,
        square::Square,
    },
    engine::eval::Eval,
};

// Network parameters
const FEATURES: usize = 768;
const HIDDEN_SIZE: usize = 512;

// Quantization factors
const QA: i32 = 255;
const QB: i32 = 64;

// Eval scaling factor
const SCALE: i32 = 400;

/// A column of the feature-weights matrix.
#[derive(Clone, Copy, Debug)]
#[repr(C, align(64))]
pub struct Accumulator([i16; HIDDEN_SIZE]);

/// Container for all network parameters
#[repr(C, align(64))]
struct Network {
    feature_weights: [Accumulator; FEATURES],
    feature_bias: Accumulator,
    output_weights: [i16; HIDDEN_SIZE * 2],
    output_bias: i16,
}

static NETWORK: Network =
    unsafe { std::mem::transmute(*include_bytes!("../../../../data/network-9cfba.bin")) };

#[derive(Clone, Debug)]
pub struct NNUE {
    white: Accumulator,
    black: Accumulator,
}

impl Default for NNUE {
    fn default() -> Self {
        Self {
            white: NETWORK.feature_bias,
            black: NETWORK.feature_bias,
        }
    }
}

impl NNUE {
    pub fn from_board(board: &Board) -> Self {
        let mut acc = Self::default();

        for sq in board.occupancy() {
            acc.add_feature(board.piece_at(sq).unwrap(), sq);
        }

        acc
    }

    fn update_feature<const ON: bool>(acc: &mut Accumulator, feature_idx: usize) {
        let zip = acc
            .0
            .iter_mut()
            .zip(&NETWORK.feature_weights[feature_idx].0);

        for (acc_val, &weight) in zip {
            if ON {
                *acc_val += weight;
            } else {
                *acc_val -= weight;
            }
        }
    }

    fn move_feature(acc: &mut Accumulator, from_feature_idx: usize, to_feature_idx: usize) {
        let zip = acc.0.iter_mut().zip(
            NETWORK.feature_weights[from_feature_idx]
                .0
                .iter()
                .zip(NETWORK.feature_weights[to_feature_idx].0.iter()),
        );

        for (acc_val, (&from, &to)) in zip {
            *acc_val += to - from;
        }
    }

    pub fn add_feature(&mut self, piece: Piece, sq: Square) {
        let (white_idx, black_idx) = nnue_index(piece, sq);

        Self::update_feature::<true>(&mut self.white, white_idx);
        Self::update_feature::<true>(&mut self.black, black_idx);
    }

    pub fn remove_feature(&mut self, piece: Piece, sq: Square) {
        let (white_idx, black_idx) = nnue_index(piece, sq);

        Self::update_feature::<false>(&mut self.white, white_idx);
        Self::update_feature::<false>(&mut self.black, black_idx);
    }

    pub fn move_piece_feature(&mut self, piece: Piece, from_sq: Square, to_sq: Square) {
        let (from_white_idx, from_black_idx) = nnue_index(piece, from_sq);
        let (to_white_idx, to_black_idx) = nnue_index(piece, to_sq);

        Self::move_feature(&mut self.white, from_white_idx, to_white_idx);
        Self::move_feature(&mut self.black, from_black_idx, to_black_idx);
    }

    pub fn evaluate(&self, side: Player) -> Eval {
        let (us, them) = match side {
            Player::White => (self.white, self.black),
            Player::Black => (self.black, self.white),
        };

        let mut output = 0;

        for (&value, &weight) in us.0.iter().zip(&NETWORK.output_weights[..HIDDEN_SIZE]) {
            output += screlu(value) * i32::from(weight);
        }

        for (&value, &weight) in them.0.iter().zip(&NETWORK.output_weights[HIDDEN_SIZE..]) {
            output += screlu(value) * i32::from(weight);
        }

        // Reduce quantization from QA * QA * QB to QA * QB.
        output /= QA;

        // Add bias.
        output += i32::from(NETWORK.output_bias);

        // Apply eval scale.
        output *= SCALE;

        // Remove quantisation altogether.
        output /= QA * QB;

        Eval(output)
    }
}

const fn nnue_index(piece: Piece, sq: Square) -> (usize, usize) {
    const COLOR_STRIDE: usize = Square::N * PieceKind::N;
    const PIECE_STRIDE: usize = Square::N;

    let p = piece.kind as usize;
    let c = piece.player as usize;

    let white_idx =
        c * COLOR_STRIDE + p * PIECE_STRIDE + sq.relative_for(Player::White).idx() as usize;

    let black_idx =
        (1 ^ c) * COLOR_STRIDE + p * PIECE_STRIDE + sq.relative_for(Player::Black).idx() as usize;

    (white_idx, black_idx)
}

fn screlu(value: i16) -> i32 {
    let v = i32::from(value).clamp(0, QA);

    v * v
}
