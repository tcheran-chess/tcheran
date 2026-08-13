use crate::{
    chess::{
        Board, File, Game, MoveObserver, Piece, PieceKind, Player, Square, moves::Move,
        squares::all::*,
    },
    engine::{
        eval::{
            Eval,
            nnue::{
                inference,
                network::{BUCKET_LAYOUT, INPUT_BUCKETS, L1, Network, OUTPUT_BUCKETS},
            },
        },
        search::MAX_PLIES_ARRAY_SIZE,
    },
    util::arrayvec::ArrayVec,
};

pub static NETWORK: Network = unsafe { std::mem::transmute(*include_bytes!(env!("NETWORK"))) };

fn input_bucket(king: Square, pov: Player) -> usize {
    let king_flip = 7 * u8::from(king.file().idx() >= 4);
    BUCKET_LAYOUT[(king.relative_for(pov).idx() ^ king_flip) as usize]
}

#[inline]
fn should_mirror(king: Square) -> bool {
    king.file() >= File::E
}

#[derive(Debug, Clone)]
pub struct NNUEChanges {
    pub adds: ArrayVec<NNUEChange, 2>,
    pub subs: ArrayVec<NNUEChange, 2>,
    pub mv: Move,
    pub moved_piece: Piece,
}

impl MoveObserver for NNUEChanges {
    fn init(&mut self, moved_piece: Piece, mv: Move) {
        self.moved_piece = moved_piece;
        self.mv = mv;
    }

    fn set(&mut self, sq: Square, piece: Piece) {
        self.adds.push(NNUEChange::new(sq, piece));
    }

    fn remove(&mut self, sq: Square, piece: Piece) {
        self.subs.push(NNUEChange::new(sq, piece));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NNUEChange {
    pub piece: Piece,
    pub square: Square,
}

impl NNUEChanges {
    pub fn uninit() -> Self {
        Self {
            adds: ArrayVec::new(),
            subs: ArrayVec::new(),

            // Values chosen arbitrarily since they will always be overwritten
            mv: Move::quiet(H8, H8),
            moved_piece: Piece::WHITE_PAWN,
        }
    }

    pub fn clear(&mut self) {
        self.adds.clear();
        self.subs.clear();

        self.mv = Move::quiet(H8, H8);
        self.moved_piece = Piece::WHITE_PAWN;
    }
}

impl NNUEChange {
    pub fn new(square: Square, piece: Piece) -> Self {
        Self { piece, square }
    }
}

// [pov][mirrored][bucket]
pub struct AccumulatorCache([[[CacheEntry; INPUT_BUCKETS]; 2]; Player::N]);

impl AccumulatorCache {
    pub fn new() -> Box<Self> {
        let mut cache = unsafe { Box::<Self>::new_zeroed().assume_init() };

        for pov in 0..Player::N {
            for side in 0..2 {
                for bucket in 0..INPUT_BUCKETS {
                    cache.0[pov][side][bucket] = CacheEntry::new();
                }
            }
        }
        cache
    }
}

pub struct NetworkStack {
    stack: Vec<NetworkStackEntry>,
    current_idx: usize,

    cache: Box<AccumulatorCache>,
}

#[derive(Clone)]
pub struct NetworkStackEntry {
    pub network: NNUE,
    changes: NNUEChanges,
    correct: [bool; Player::N],
}

pub struct CacheEntry {
    value: Accumulator,
    board: Board,
}

impl CacheEntry {
    fn new() -> Self {
        Self {
            value: Accumulator(NETWORK.l0_biases),
            board: Board::EMPTY,
        }
    }
}

impl NetworkStack {
    pub fn new() -> Self {
        Self {
            stack: vec![
                NetworkStackEntry {
                    network: NNUE::default(),
                    changes: NNUEChanges::uninit(),
                    correct: [false; Player::N],
                };
                MAX_PLIES_ARRAY_SIZE
            ],
            current_idx: 0,

            cache: AccumulatorCache::new(),
        }
    }

    pub fn setup(&mut self, board: &Board) {
        for pov in Player::ALL {
            self.stack[0].network.refresh(board, pov, &mut self.cache);
            self.stack[0].correct[pov] = true;
        }

        self.current_idx = 0;
    }

    pub fn next_changes(&mut self) -> &mut NNUEChanges {
        self.current_idx += 1;

        self.stack[self.current_idx].changes.clear();
        self.stack[self.current_idx].correct = [false; Player::N];

        &mut self.stack[self.current_idx].changes
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
                let current_entry = &mut self.stack[self.current_idx];
                current_entry
                    .network
                    .refresh(&game.board, pov, &mut self.cache);
                current_entry.correct[pov] = true;
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
                let from = entry.changes.mv.from();
                let to = entry.changes.mv.to();

                let crossed_mirroring_boundary = should_mirror(from) != should_mirror(to);
                if crossed_mirroring_boundary {
                    return None;
                }

                let changed_input_bucket = input_bucket(entry.changes.mv.from(), pov)
                    != input_bucket(entry.changes.mv.to(), pov);

                if changed_input_bucket {
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
        match (entry.changes.adds.as_slice(), entry.changes.subs.as_slice()) {
            ([add1], [sub1]) => {
                let bucket = input_bucket(king, pov);
                let add1 = nnue_index(add1.piece, add1.square, king, pov);
                let sub1 = nnue_index(sub1.piece, sub1.square, king, pov);
                NNUE::add1_sub1(
                    &previous_entry.network[pov],
                    &mut entry.network[pov],
                    bucket,
                    add1,
                    sub1,
                );
            }
            ([add1], [sub1, sub2]) => {
                let bucket = input_bucket(king, pov);
                let add1 = nnue_index(add1.piece, add1.square, king, pov);
                let sub1 = nnue_index(sub1.piece, sub1.square, king, pov);
                let sub2 = nnue_index(sub2.piece, sub2.square, king, pov);

                NNUE::add1_sub2(
                    &previous_entry.network[pov],
                    &mut entry.network[pov],
                    bucket,
                    add1,
                    sub1,
                    sub2,
                );
            }
            ([add1, add2], [sub1, sub2]) => {
                let bucket = input_bucket(king, pov);
                let add1 = nnue_index(add1.piece, add1.square, king, pov);
                let add2 = nnue_index(add2.piece, add2.square, king, pov);
                let sub1 = nnue_index(sub1.piece, sub1.square, king, pov);
                let sub2 = nnue_index(sub2.piece, sub2.square, king, pov);

                NNUE::add2_sub2(
                    &previous_entry.network[pov],
                    &mut entry.network[pov],
                    bucket,
                    add1,
                    add2,
                    sub1,
                    sub2,
                );
            }
            (_, _) => unreachable!(),
        }
    }
}

/// A column of the feature-weights matrix.
#[derive(Clone)]
#[repr(C, align(64))]
pub struct Accumulator(pub [i16; L1]);

#[derive(Clone)]
pub struct NNUE {
    accumulators: [Accumulator; Player::N],
}

impl Default for NNUE {
    fn default() -> Self {
        Self {
            accumulators: [
                Accumulator(NETWORK.l0_biases),
                Accumulator(NETWORK.l0_biases),
            ],
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
    pub fn refresh(&mut self, board: &Board, pov: Player, cache: &mut AccumulatorCache) {
        let king = board.king_square(pov);
        let king_side = usize::from(should_mirror(king));
        let king_bucket = BUCKET_LAYOUT[king.relative_for(pov)];

        let cache_entry = &mut cache.0[pov][king_side][king_bucket];

        let mut adds = ArrayVec::<_, 32>::new();
        let mut subs = ArrayVec::<_, 32>::new();

        for player in Player::ALL {
            for piece in PieceKind::ALL {
                let pieces = board.pieces_of_kind(piece, player);
                let old_pieces = cache_entry.board.pieces_of_kind(piece, player);

                let to_add = pieces & !old_pieces;
                let to_sub = !pieces & old_pieces;

                let piece = Piece::new(player, piece);

                for square in to_add {
                    adds.push(nnue_index(piece, square, king, pov));
                }

                for square in to_sub {
                    subs.push(nnue_index(piece, square, king, pov));
                }
            }
        }

        let bucket = input_bucket(king, pov);

        for add in &adds {
            Self::add1(&mut cache_entry.value, bucket, *add);
        }

        for sub in &subs {
            Self::sub1(&mut cache_entry.value, bucket, *sub);
        }

        cache_entry.board = board.clone();

        self.accumulators[pov] = cache_entry.value.clone();
    }

    #[expect(clippy::needless_range_loop, reason = "Readability")]
    fn add1(acc: &mut Accumulator, bucket: usize, add1: usize) {
        let add1_features = &NETWORK.l0_weights[bucket][add1];

        for i in 0..L1 {
            acc.0[i] += add1_features[i];
        }
    }

    #[expect(clippy::needless_range_loop, reason = "Readability")]
    fn sub1(acc: &mut Accumulator, bucket: usize, sub1: usize) {
        let sub1_features = &NETWORK.l0_weights[bucket][sub1];

        for i in 0..L1 {
            acc.0[i] -= sub1_features[i];
        }
    }

    fn add1_sub1(
        previous_acc: &Accumulator,
        acc: &mut Accumulator,
        bucket: usize,
        add1: usize,
        sub1: usize,
    ) {
        let add1_features = &NETWORK.l0_weights[bucket][add1];
        let sub1_features = &NETWORK.l0_weights[bucket][sub1];

        for i in 0..L1 {
            acc.0[i] = previous_acc.0[i] + add1_features[i] - sub1_features[i];
        }
    }

    pub fn add1_sub2(
        previous_acc: &Accumulator,
        acc: &mut Accumulator,
        bucket: usize,
        add1: usize,
        sub1: usize,
        sub2: usize,
    ) {
        let add1_features = &NETWORK.l0_weights[bucket][add1];
        let sub1_features = &NETWORK.l0_weights[bucket][sub1];
        let sub2_features = &NETWORK.l0_weights[bucket][sub2];

        for i in 0..L1 {
            acc.0[i] = previous_acc.0[i] + add1_features[i] - sub1_features[i] - sub2_features[i];
        }
    }

    pub fn add2_sub2(
        previous_acc: &Accumulator,
        acc: &mut Accumulator,
        bucket: usize,
        add1: usize,
        add2: usize,
        sub1: usize,
        sub2: usize,
    ) {
        let add1_features = &NETWORK.l0_weights[bucket][add1];
        let add2_features = &NETWORK.l0_weights[bucket][add2];
        let sub1_features = &NETWORK.l0_weights[bucket][sub1];
        let sub2_features = &NETWORK.l0_weights[bucket][sub2];

        for i in 0..L1 {
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
        let output = inference::forward(us, them, output_bucket);
        Eval(output)
    }

    pub fn approx_contribution(&mut self, game: &Game, square: Square, player: Player) -> Eval {
        let eval = self.evaluate(player, game);

        let piece = game.board.piece_guaranteed_at(square);

        // Remove this feature from the accumulator to see what the eval looks like without it
        for pov in Player::ALL {
            let king = game.board.king_square(pov);
            let bucket = input_bucket(king, pov);
            Self::sub1(&mut self[pov], bucket, nnue_index(piece, square, king, pov));
        }

        let eval_without_feature = self.evaluate(player, game);

        // Add the feature back again
        for pov in Player::ALL {
            let king = game.board.king_square(pov);
            let bucket = input_bucket(king, pov);
            Self::add1(&mut self[pov], bucket, nnue_index(piece, square, king, pov));
        }

        eval - eval_without_feature
    }
}

fn nnue_index(piece: Piece, sq: Square, king: Square, pov: Player) -> usize {
    const COLOR_STRIDE: usize = Square::N * PieceKind::N;
    const PIECE_STRIDE: usize = Square::N;

    let p = piece.kind as usize;

    let c = piece.player as usize ^ pov as usize;

    let square_idx = sq
        .relative_for(pov)
        .mirrored_horizontally_if(should_mirror(king))
        .idx();

    c * COLOR_STRIDE + p * PIECE_STRIDE + square_idx as usize
}
