use crate::{
    chess::{
        board::Board,
        game::Game,
        moves::Move,
        piece::{Piece, PieceKind, PromotionPieceKind},
        player::Player,
        square::{Square, squares, squares::all::H8},
    },
    engine::{eval::Eval, search::MAX_SEARCH_DEPTH_SIZE},
};

// Network parameters
const FEATURES: usize = 768;
const HIDDEN_SIZE: usize = 1024;
const OUTPUT_BUCKETS: usize = 8;

// Quantization factors
const QA: i32 = 255;
const QB: i32 = 64;

// Eval scaling factor
pub const SCALE: i32 = 313;

#[derive(Clone)]
struct Changes {
    pub mv: Move,
    pub moved_piece: Piece,
    pub captured_piece: Option<Piece>,
}

impl Changes {
    pub const fn uninit() -> Self {
        Self {
            // Arbitrarily chosen - these values will never be read
            mv: Move::quiet(H8, H8),
            moved_piece: Piece::WHITE_KING,
            captured_piece: None,
        }
    }
}

pub struct NetworkStack {
    stack: Vec<NetworkStackEntry>,
    current_idx: usize,
}

#[derive(Clone)]
pub struct NetworkStackEntry {
    pub network: NNUE,
    changes: Changes,
    correct: [bool; Player::N],
}

impl NetworkStack {
    pub fn new() -> Self {
        Self {
            stack: vec![
                NetworkStackEntry {
                    network: NNUE::default(),
                    changes: Changes::uninit(),
                    correct: [false; Player::N],
                };
                MAX_SEARCH_DEPTH_SIZE
            ],
            current_idx: 0,
        }
    }

    pub fn setup(&mut self, board: &Board) {
        self.stack[0].network = NNUE::from_board(board);
        self.stack[0].correct = [true; Player::N];
        self.current_idx = 0;
    }

    pub fn push(&mut self, board: &Board, mv: Move) {
        self.current_idx += 1;

        let current_entry = self.current_entry();
        current_entry.changes.mv = mv;
        current_entry.changes.moved_piece = board.piece_guaranteed_at(mv.src());
        current_entry.changes.captured_piece = board.piece_at(mv.dst());
        current_entry.correct = [false; Player::N];
    }

    pub fn pop(&mut self) {
        self.current_idx -= 1;
    }

    fn current_entry(&mut self) -> &mut NetworkStackEntry {
        &mut self.stack[self.current_idx]
    }

    pub fn evaluate(&mut self, game: &Game) -> Eval {
        // First, we need to cycle through our stack and update any network accumulators that we
        // deferred updates for.

        for pov in Player::ALL {
            // We don't need to do this if we've only got one entry in the stack.
            for i in 1..=self.current_idx {
                if self.stack[i].correct[pov] {
                    continue;
                }

                if let (prev, [entry, ..]) = self.stack.split_at_mut(i) {
                    let previous_entry = prev.last().unwrap();

                    Self::update_accumulator(entry, previous_entry, pov);
                }

                self.stack[i].correct[pov] = true;
            }
        }

        // We should now have a full stack of materialised accumulators, all the way up to our current one
        // so we can evaluate that.
        self.current_entry().network.evaluate(game.player, game)
    }

    fn update_accumulator(
        entry: &mut NetworkStackEntry,
        previous_entry: &NetworkStackEntry,
        pov: Player,
    ) {
        let mv = entry.changes.mv;
        let moved_piece = entry.changes.moved_piece;
        let captured_piece = entry.changes.captured_piece;
        let player = moved_piece.player;

        let moved_piece_at_dst = Piece::new(
            player,
            mv.promotion()
                .map_or(moved_piece.kind, PromotionPieceKind::piece),
        );

        let add1 = nnue_index(moved_piece_at_dst, mv.dst(), pov);
        let sub1 = nnue_index(moved_piece, mv.src(), pov);

        if mv.is_castling() {
            let (rook_from, rook_to) = squares::castle_squares(player, mv.dst())
                .expect("Move should have castling squares");
            let rook = Piece::new(moved_piece.player, PieceKind::Rook);

            let add2 = nnue_index(rook, rook_to, pov);
            let sub2 = nnue_index(rook, rook_from, pov);

            NNUE::add2_sub2(
                &previous_entry.network[pov],
                &mut entry.network[pov],
                add1,
                add2,
                sub1,
                sub2,
            );
        } else if mv.is_capture() {
            let sub2 = if mv.is_en_passant() {
                let en_passant_capture_square = mv.dst().backward(player);
                let taken_pawn = Piece::new(player.other(), PieceKind::Pawn);
                nnue_index(taken_pawn, en_passant_capture_square, pov)
            } else {
                let taken_piece = captured_piece.expect("Move should have captured piece");
                nnue_index(taken_piece, mv.dst(), pov)
            };

            NNUE::add1_sub2(
                &previous_entry.network[pov],
                &mut entry.network[pov],
                add1,
                sub1,
                sub2,
            );
        } else {
            NNUE::add1_sub1(&previous_entry.network[pov], &mut entry.network[pov], add1, sub1);
        }
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
    output_weights: [[i16; HIDDEN_SIZE * 2]; OUTPUT_BUCKETS],
    output_bias: [i16; OUTPUT_BUCKETS],
}

static NETWORK: Network = unsafe { std::mem::transmute(*include_bytes!(env!("NETWORK"))) };

#[derive(Clone)]
pub struct NNUE {
    accumulators: [Accumulator; Player::N],
}

impl Default for NNUE {
    fn default() -> Self {
        Self {
            accumulators: [NETWORK.feature_bias.clone(), NETWORK.feature_bias.clone()],
        }
    }
}

impl std::ops::Index<Player> for NNUE {
    type Output = Accumulator;

    fn index(&self, index: Player) -> &Self::Output {
        &self.accumulators[index as usize]
    }
}

impl std::ops::IndexMut<Player> for NNUE {
    fn index_mut(&mut self, index: Player) -> &mut Self::Output {
        &mut self.accumulators[index as usize]
    }
}

impl NNUE {
    pub fn from_board(board: &Board) -> Self {
        let mut nnue = Self::default();

        for pov in Player::ALL {
            for sq in board.occupancy() {
                let piece = board.piece_guaranteed_at(sq);
                Self::add1(&mut nnue[pov], nnue_index(piece, sq, pov));
            }
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

    fn bucket(game: &Game) -> usize {
        let divisor = 32usize.div_ceil(OUTPUT_BUCKETS);
        (game.board.occupancy().count() as usize - 2) / divisor
    }

    pub fn evaluate(&self, player: Player, game: &Game) -> Eval {
        let (us, them) = match player {
            Player::White => (&self[Player::White], &self[Player::Black]),
            Player::Black => (&self[Player::Black], &self[Player::White]),
        };

        let mut output = 0;

        let output_bucket = Self::bucket(game);
        let output_weights = &NETWORK.output_weights[output_bucket];
        let output_bias = &NETWORK.output_bias[output_bucket];

        for (&value, &weight) in us.0.iter().zip(&output_weights[..HIDDEN_SIZE]) {
            output += screlu(value) * i32::from(weight);
        }

        for (&value, &weight) in them.0.iter().zip(&output_weights[HIDDEN_SIZE..]) {
            output += screlu(value) * i32::from(weight);
        }

        // Reduce quantization from QA * QA * QB to QA * QB.
        output /= QA;

        // Add bias.
        output += i32::from(*output_bias);

        // Apply eval scale.
        output *= SCALE;

        // Remove quantisation altogether.
        output /= QA * QB;

        Eval(output)
    }

    pub fn approx_contribution(&mut self, game: &Game, square: Square, player: Player) -> Eval {
        let eval = self.evaluate(player, game);

        let piece = game.board.piece_guaranteed_at(square);

        // Remove this feature from the accumulator to see what the eval looks like without it
        for pov in Player::ALL {
            Self::sub1(&mut self[pov], nnue_index(piece, square, pov));
        }

        let eval_without_feature = self.evaluate(player, game);

        // Add the feature back again
        for pov in Player::ALL {
            Self::add1(&mut self[pov], nnue_index(piece, square, pov));
        }

        eval - eval_without_feature
    }
}

fn nnue_index(piece: Piece, sq: Square, pov: Player) -> usize {
    const COLOR_STRIDE: usize = Square::N * PieceKind::N;
    const PIECE_STRIDE: usize = Square::N;

    let p = piece.kind as usize;
    let c = piece.player as usize;

    let square_idx = sq.relative_for(pov).idx();

    (c ^ pov as usize) * COLOR_STRIDE + p * PIECE_STRIDE + square_idx as usize
}

fn screlu(value: i16) -> i32 {
    let v = i32::from(value).clamp(0, QA);

    v * v
}
