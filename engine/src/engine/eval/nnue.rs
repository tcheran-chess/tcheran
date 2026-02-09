use crate::{
    chess::{
        board::Board,
        game::Game,
        moves::Move,
        piece::{Piece, PieceKind, PromotionPieceKind},
        player::Player,
        square::{File, Square, squares, squares::all::H8},
    },
    engine::{
        eval::{Eval, simd},
        search::MAX_SEARCH_DEPTH_SIZE,
    },
};

// Network parameters
const FEATURES: usize = 768;
pub const HIDDEN_SIZE: usize = 1024;
const OUTPUT_BUCKETS: usize = 8;

// Quantization factors
pub const QA: i16 = 255;
pub const QB: i16 = 64;

// Eval scaling factor
pub const SCALE: i32 = 289;

/// Container for all network parameters
#[repr(C, align(64))]
pub struct Network {
    pub feature_weights: [Accumulator; FEATURES],
    pub feature_bias: Accumulator,
    pub output_weights: [[i16; HIDDEN_SIZE * 2]; OUTPUT_BUCKETS],
    pub output_bias: [i16; OUTPUT_BUCKETS],
}

pub static NETWORK: Network = unsafe { std::mem::transmute(*include_bytes!(env!("NETWORK"))) };

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
            if self.current_entry().correct[pov] {
                continue;
            }

            // We can efficiently update if there hasn't been a move since our last 'correct' accumulator
            // that causes us to need to do a full refresh (e.g. a king crossing the horizontal mirroring
            // boundary).
            if let Some(i) = self.last_good_efficient_update_index(pov) {
                self.update(i, game, pov);
            } else {
                self.current_entry().network[pov].refresh(&game.board, pov);
                self.current_entry().correct[pov] = true;
            }
        }

        // We should now have a full stack of materialised accumulators, all the way up to our current one
        // so we can evaluate that.
        self.current_entry().network.evaluate(game.player, game)
    }

    fn last_good_efficient_update_index(&self, pov: Player) -> Option<usize> {
        // Starting at the current accumulator, look back until we find either a correct accumulator
        // we can work forward from, or a change in board state that means we can't do efficient updates.
        for i in (0..=self.current_idx).rev() {
            let entry = &self.stack[i];

            if entry.correct[pov] {
                return Some(i);
            }

            // If the king crossed over the middle of the board, the board either becomes, or stops being
            // mirrored and will require a full refresh.
            if entry.changes.moved_piece.kind == PieceKind::King {
                let from_file = entry.changes.mv.src().file().idx();
                let to_file = entry.changes.mv.dst().file().idx();

                let crossed_mirroring_boundary = (from_file <= File::D.idx()
                    && to_file >= File::E.idx())
                    || (from_file >= File::E.idx() && to_file <= File::D.idx());

                if crossed_mirroring_boundary {
                    return None;
                }
            }
        }

        None
    }

    fn update(&mut self, last_good_idx: usize, game: &Game, pov: Player) {
        // We know the king is on the same side of the board for all entries since last_good_idx by definition
        let king = game.board.king_square(pov);

        for i in last_good_idx..self.current_idx {
            if let (prev, [entry, ..]) = self.stack.split_at_mut(i + 1) {
                Self::update_accumulator(entry, &prev[i], king, pov);
            }

            self.stack[i].correct[pov] = true;
        }
    }

    fn update_accumulator(
        entry: &mut NetworkStackEntry,
        previous_entry: &NetworkStackEntry,
        king: Square,
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

        let add1 = nnue_index(moved_piece_at_dst, mv.dst(), king, pov);
        let sub1 = nnue_index(moved_piece, mv.src(), king, pov);

        if mv.is_castling() {
            let (rook_from, rook_to) = squares::castle_squares(player, mv.dst())
                .expect("Move should have castling squares");
            let rook = Piece::new(moved_piece.player, PieceKind::Rook);

            let add2 = nnue_index(rook, rook_to, king, pov);
            let sub2 = nnue_index(rook, rook_from, king, pov);

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
                nnue_index(taken_pawn, en_passant_capture_square, king, pov)
            } else {
                let taken_piece = captured_piece.expect("Move should have captured piece");
                nnue_index(taken_piece, mv.dst(), king, pov)
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
pub struct Accumulator(pub [i16; HIDDEN_SIZE]);

impl Accumulator {
    pub fn refresh(&mut self, board: &Board, pov: Player) {
        let king = board.king_square(pov);

        self.0 = NETWORK.feature_bias.0;

        for sq in board.occupancy() {
            let piece = board.piece_guaranteed_at(sq);
            NNUE::add1(self, nnue_index(piece, sq, king, pov));
        }
    }
}

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
            nnue[pov].refresh(board, pov);
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
        let (us, them) = (&self[player], &self[player.other()]);
        let output_bucket = Self::bucket(game);
        let mut output = simd::sum_output_weights(us, them, output_bucket);

        // Reduce quantization from QA * QA * QB to QA * QB.
        output /= i32::from(QA);

        // Add bias.
        let output_bias = &NETWORK.output_bias[output_bucket];
        output += i32::from(*output_bias);

        // Apply eval scale.
        output *= SCALE;

        // Remove quantisation altogether.
        output /= i32::from(QA) * i32::from(QB);

        Eval(output)
    }

    pub fn approx_contribution(&mut self, game: &Game, square: Square, player: Player) -> Eval {
        let eval = self.evaluate(player, game);

        let piece = game.board.piece_guaranteed_at(square);

        // Remove this feature from the accumulator to see what the eval looks like without it
        for pov in Player::ALL {
            let king = game.board.king_square(pov);
            Self::sub1(&mut self[pov], nnue_index(piece, square, king, pov));
        }

        let eval_without_feature = self.evaluate(player, game);

        // Add the feature back again
        for pov in Player::ALL {
            let king = game.board.king_square(pov);
            Self::add1(&mut self[pov], nnue_index(piece, square, king, pov));
        }

        eval - eval_without_feature
    }
}

fn nnue_index(piece: Piece, sq: Square, king: Square, pov: Player) -> usize {
    const COLOR_STRIDE: usize = Square::N * PieceKind::N;
    const PIECE_STRIDE: usize = Square::N;

    let p = piece.kind as usize;
    let c = piece.player as usize;

    let square_idx = sq.relative_for(pov).idx();
    let king_flip = 7 * u8::from(king.file().idx() >= 4);

    (c ^ pov as usize) * COLOR_STRIDE + p * PIECE_STRIDE + (square_idx ^ king_flip) as usize
}
