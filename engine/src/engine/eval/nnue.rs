use crate::{
    chess::{
        board::Board,
        game::Game,
        moves::Move,
        piece::{Piece, PieceKind},
        player::Player,
        square::{Square, squares},
    },
    engine::{eval::Eval, search::MAX_SEARCH_DEPTH_SIZE},
};

// Network parameters
const FEATURES: usize = 768;
const HIDDEN_SIZE: usize = 768;

// Quantization factors
const QA: i32 = 255;
const QB: i32 = 64;

// Eval scaling factor
const SCALE: i32 = 400;

#[derive(Clone)]
enum FeatureChanges {
    None,
    Add1Sub1 {
        white_add1: usize,
        white_sub1: usize,
        black_add1: usize,
        black_sub1: usize,
    },
    Add1Sub2 {
        white_add1: usize,
        white_sub1: usize,
        white_sub2: usize,
        black_add1: usize,
        black_sub1: usize,
        black_sub2: usize,
    },
    Add2Sub2 {
        white_add1: usize,
        white_add2: usize,
        white_sub1: usize,
        white_sub2: usize,
        black_add1: usize,
        black_add2: usize,
        black_sub1: usize,
        black_sub2: usize,
    },
}

pub struct NetworkStack {
    stack: Vec<NetworkStackEntry>,
    current_idx: usize,
}

#[derive(Clone)]
pub struct NetworkStackEntry {
    pub network: NNUE,
    feature_changes: FeatureChanges,
    correct: bool,
}

impl NetworkStack {
    pub fn from_board(board: &Board) -> Self {
        let mut t = Self {
            stack: vec![
                NetworkStackEntry {
                    network: NNUE::default(),
                    feature_changes: FeatureChanges::None,
                    correct: false,
                };
                MAX_SEARCH_DEPTH_SIZE
            ],
            current_idx: 0,
        };

        t.stack[0].network = NNUE::from_board(board);
        t.stack[0].correct = true;
        t
    }

    pub fn push(&mut self, board: &Board, mv: Move) {
        self.current_idx += 1;
        self.current_entry().correct = false;

        let piece_at_src = board.piece_guaranteed_at(mv.src());
        let piece_at_dst = mv
            .promotion()
            .map_or(piece_at_src, |p| Piece::new(piece_at_src.player, p.piece()));

        let player = piece_at_src.player;

        let (white_add1, black_add1) = nnue_index(piece_at_dst, mv.dst());
        let (white_sub1, black_sub1) = nnue_index(piece_at_src, mv.src());

        if mv.is_castling() {
            let (rook_from, rook_to) = squares::castle_squares(player, mv.dst())
                .expect("Move should have castling squares");
            let rook = board.piece_guaranteed_at(rook_from);

            let (white_add2, black_add2) = nnue_index(rook, rook_to);
            let (white_sub2, black_sub2) = nnue_index(rook, rook_from);

            self.current_entry().feature_changes = FeatureChanges::Add2Sub2 {
                white_add1,
                white_add2,
                white_sub1,
                white_sub2,
                black_add1,
                black_add2,
                black_sub1,
                black_sub2,
            };
        } else if mv.is_capture() {
            let (white_sub2, black_sub2) = if mv.is_en_passant() {
                let en_passant_capture_square = mv.dst().backward(player);
                let taken_pawn = board.piece_guaranteed_at(en_passant_capture_square);
                nnue_index(taken_pawn, en_passant_capture_square)
            } else {
                let taken_piece = board.piece_guaranteed_at(mv.dst());
                nnue_index(taken_piece, mv.dst())
            };

            self.current_entry().feature_changes = FeatureChanges::Add1Sub2 {
                white_add1,
                white_sub1,
                white_sub2,
                black_add1,
                black_sub1,
                black_sub2,
            };
        } else {
            self.current_entry().feature_changes = FeatureChanges::Add1Sub1 {
                white_add1,
                white_sub1,
                black_add1,
                black_sub1,
            };
        }
    }

    pub fn pop(&mut self) {
        self.current_idx -= 1;
    }

    fn current_entry(&mut self) -> &mut NetworkStackEntry {
        &mut self.stack[self.current_idx]
    }

    pub fn evaluate(&mut self, player: Player) -> Eval {
        // First, we need to cycle through our stack and update any network accumulators that we
        // deferred updates for.

        // We don't need to do this if we've only got one entry in the stack.
        for i in 1..=self.current_idx {
            if self.stack[i].correct {
                continue;
            }

            if let (prev, [entry, ..]) = self.stack.split_at_mut(i) {
                let previous_entry = prev.last().unwrap();

                // For each accumulator, copy over the values from the previous (now-materialised) accumulator
                // while also applying feature changes.
                match entry.feature_changes {
                    FeatureChanges::Add1Sub1 {
                        white_add1,
                        white_sub1,
                        black_add1,
                        black_sub1,
                    } => {
                        NNUE::add1_sub1(
                            &previous_entry.network.white,
                            &mut entry.network.white,
                            white_add1,
                            white_sub1,
                        );
                        NNUE::add1_sub1(
                            &previous_entry.network.black,
                            &mut entry.network.black,
                            black_add1,
                            black_sub1,
                        );
                    }
                    FeatureChanges::Add1Sub2 {
                        white_add1,
                        white_sub1,
                        white_sub2,
                        black_add1,
                        black_sub1,
                        black_sub2,
                    } => {
                        NNUE::add1_sub2(
                            &previous_entry.network.white,
                            &mut entry.network.white,
                            white_add1,
                            white_sub1,
                            white_sub2,
                        );
                        NNUE::add1_sub2(
                            &previous_entry.network.black,
                            &mut entry.network.black,
                            black_add1,
                            black_sub1,
                            black_sub2,
                        );
                    }
                    FeatureChanges::Add2Sub2 {
                        white_add1,
                        white_add2,
                        white_sub1,
                        white_sub2,
                        black_add1,
                        black_add2,
                        black_sub1,
                        black_sub2,
                    } => {
                        NNUE::add2_sub2(
                            &previous_entry.network.white,
                            &mut entry.network.white,
                            white_add1,
                            white_add2,
                            white_sub1,
                            white_sub2,
                        );
                        NNUE::add2_sub2(
                            &previous_entry.network.black,
                            &mut entry.network.black,
                            black_add1,
                            black_add2,
                            black_sub1,
                            black_sub2,
                        );
                    }
                    FeatureChanges::None => {
                        unreachable!();
                    }
                }
            }

            self.stack[i].correct = true;
        }

        // We should now have a full stack of materialised accumulators, all the way up to our current one
        // so we can evaluate that.
        self.current_entry().network.evaluate(player)
    }
}

/// A column of the feature-weights matrix.
#[derive(Clone)]
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

static NETWORK: Network = unsafe { std::mem::transmute(*include_bytes!(env!("NETWORK"))) };

#[derive(Clone)]
pub struct NNUE {
    white: Accumulator,
    black: Accumulator,
}

impl Default for NNUE {
    fn default() -> Self {
        Self {
            white: NETWORK.feature_bias.clone(),
            black: NETWORK.feature_bias.clone(),
        }
    }
}

impl NNUE {
    pub fn from_board(board: &Board) -> Self {
        let mut nnue = Self::default();

        for sq in board.occupancy() {
            let piece = board.piece_guaranteed_at(sq);

            let (white_idx, black_idx) = nnue_index(piece, sq);

            Self::add1(&mut nnue.white, white_idx);
            Self::add1(&mut nnue.black, black_idx);
        }

        nnue
    }

    #[expect(clippy::needless_range_loop, reason = "Readability")]
    fn add1(acc: &mut Accumulator, add1: usize) {
        let add1_features = &NETWORK.feature_weights[add1].0;

        for i in 0..HIDDEN_SIZE {
            acc.0[i] += add1_features[i];
        }
    }

    #[expect(clippy::needless_range_loop, reason = "Readability")]
    fn sub1(acc: &mut Accumulator, sub1: usize) {
        let sub1_features = &NETWORK.feature_weights[sub1].0;

        for i in 0..HIDDEN_SIZE {
            acc.0[i] -= sub1_features[i];
        }
    }

    fn add1_sub1(previous_acc: &Accumulator, acc: &mut Accumulator, add1: usize, sub1: usize) {
        let add1_features = &NETWORK.feature_weights[add1].0;
        let sub1_features = &NETWORK.feature_weights[sub1].0;

        for i in 0..HIDDEN_SIZE {
            acc.0[i] = previous_acc.0[i] + add1_features[i] - sub1_features[i];
        }
    }

    pub fn add1_sub2(
        previous_acc: &Accumulator,
        acc: &mut Accumulator,
        add1: usize,
        sub1: usize,
        sub2: usize,
    ) {
        let add1_features = &NETWORK.feature_weights[add1].0;
        let sub1_features = &NETWORK.feature_weights[sub1].0;
        let sub2_features = &NETWORK.feature_weights[sub2].0;

        for i in 0..HIDDEN_SIZE {
            acc.0[i] = previous_acc.0[i] + add1_features[i] - sub1_features[i] - sub2_features[i];
        }
    }

    pub fn add2_sub2(
        previous_acc: &Accumulator,
        acc: &mut Accumulator,
        add1: usize,
        add2: usize,
        sub1: usize,
        sub2: usize,
    ) {
        let add1_features = &NETWORK.feature_weights[add1].0;
        let add2_features = &NETWORK.feature_weights[add2].0;
        let sub1_features = &NETWORK.feature_weights[sub1].0;
        let sub2_features = &NETWORK.feature_weights[sub2].0;

        for i in 0..HIDDEN_SIZE {
            acc.0[i] = previous_acc.0[i] + add1_features[i] + add2_features[i]
                - sub1_features[i]
                - sub2_features[i];
        }
    }

    pub fn evaluate(&self, side: Player) -> Eval {
        let (us, them) = match side {
            Player::White => (&self.white, &self.black),
            Player::Black => (&self.black, &self.white),
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

    pub fn approx_contribution(&mut self, game: &Game, square: Square, player: Player) -> Eval {
        let eval = self.evaluate(player);

        let piece = game.board.piece_guaranteed_at(square);

        let (white_idx, black_idx) = nnue_index(piece, square);

        // Remove this feature from the accumulator to see what the eval looks like without it
        Self::sub1(&mut self.white, white_idx);
        Self::sub1(&mut self.black, black_idx);

        let eval_without_feature = self.evaluate(player);

        // Add the feature back again
        Self::add1(&mut self.white, white_idx);
        Self::add1(&mut self.black, black_idx);

        eval - eval_without_feature
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
