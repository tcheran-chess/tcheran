use crate::{
    chess::{
        MoveObserver,
        moves::Move,
        prelude::{Board, H8, Piece, PieceKind, Player, Square},
    },
    engine::eval::nnue::{
        Accumulator,
        cache::AccumulatorCache,
        features::psqt::index::psqt_feature_index,
        network::L1,
        nnue::{NETWORK, input_bucket},
    },
    util::arrayvec::ArrayVec,
};

#[derive(Debug, Clone)]
pub struct PsqtChanges {
    pub adds: ArrayVec<(Piece, Square), 2>,
    pub subs: ArrayVec<(Piece, Square), 2>,
    pub mv: Move,
    pub moved_piece: Piece,
}

impl PsqtChanges {
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

#[derive(Clone)]
pub struct PsqtStackEntry {
    pub accumulators: [Accumulator; Player::N],
    pub changes: PsqtChanges,
    pub correct: [bool; Player::N],
}

impl Default for PsqtStackEntry {
    fn default() -> Self {
        Self {
            accumulators: [Accumulator(NETWORK.l0_biases), Accumulator(NETWORK.l0_biases)],
            changes: PsqtChanges::uninit(),
            correct: [false; Player::N],
        }
    }
}

impl MoveObserver for PsqtStackEntry {
    fn init(&mut self, moved_piece: Piece, mv: Move) {
        self.changes.moved_piece = moved_piece;
        self.changes.mv = mv;
    }

    fn set(&mut self, sq: Square, piece: Piece) {
        self.changes.adds.push((piece, sq));
    }

    fn remove(&mut self, sq: Square, piece: Piece) {
        self.changes.subs.push((piece, sq));
    }
}

impl PsqtStackEntry {
    pub fn clear(&mut self) {
        self.changes.clear();
        self.correct = [false; Player::N];
    }

    pub(in crate::engine::eval::nnue) fn refresh(&mut self, board: &Board, pov: Player, cache: &mut AccumulatorCache) {
        let king = board.king_square(pov);
        let cache_entry = cache.get(pov, king);

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
                    adds.push(psqt_feature_index((piece, square), king, pov));
                }

                for square in to_sub {
                    subs.push(psqt_feature_index((piece, square), king, pov));
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
        self.correct[pov] = true;
    }

    pub fn update(&mut self, prev: &Self, king: Square, pov: Player) {
        match (self.changes.adds.as_slice(), self.changes.subs.as_slice()) {
            (&[add1], &[sub1]) => {
                let bucket = input_bucket(king, pov);
                let add1 = psqt_feature_index(add1, king, pov);
                let sub1 = psqt_feature_index(sub1, king, pov);
                Self::add1_sub1(&prev.accumulators[pov], &mut self.accumulators[pov], bucket, add1, sub1);
            }
            (&[add1], &[sub1, sub2]) => {
                let bucket = input_bucket(king, pov);
                let add1 = psqt_feature_index(add1, king, pov);
                let sub1 = psqt_feature_index(sub1, king, pov);
                let sub2 = psqt_feature_index(sub2, king, pov);
                Self::add1_sub2(&prev.accumulators[pov], &mut self.accumulators[pov], bucket, add1, sub1, sub2);
            }
            (&[add1, add2], &[sub1, sub2]) => {
                let bucket = input_bucket(king, pov);
                let add1 = psqt_feature_index(add1, king, pov);
                let add2 = psqt_feature_index(add2, king, pov);
                let sub1 = psqt_feature_index(sub1, king, pov);
                let sub2 = psqt_feature_index(sub2, king, pov);
                Self::add2_sub2(&prev.accumulators[pov], &mut self.accumulators[pov], bucket, add1, add2, sub1, sub2);
            }
            (_, _) => unreachable!(),
        }

        self.correct[pov] = true;
    }

    fn add1(acc: &mut Accumulator, bucket: usize, add1: usize) {
        let add1_features = &NETWORK.l0_psqt_weights[bucket][add1];

        for i in 0..L1 {
            acc.0[i] += add1_features[i];
        }
    }

    fn sub1(acc: &mut Accumulator, bucket: usize, sub1: usize) {
        let sub1_features = &NETWORK.l0_psqt_weights[bucket][sub1];

        for i in 0..L1 {
            acc.0[i] -= sub1_features[i];
        }
    }

    fn add1_sub1(previous_acc: &Accumulator, acc: &mut Accumulator, bucket: usize, add1: usize, sub1: usize) {
        let add1_features = &NETWORK.l0_psqt_weights[bucket][add1];
        let sub1_features = &NETWORK.l0_psqt_weights[bucket][sub1];

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
        let add1_features = &NETWORK.l0_psqt_weights[bucket][add1];
        let sub1_features = &NETWORK.l0_psqt_weights[bucket][sub1];
        let sub2_features = &NETWORK.l0_psqt_weights[bucket][sub2];

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
        let add1_features = &NETWORK.l0_psqt_weights[bucket][add1];
        let add2_features = &NETWORK.l0_psqt_weights[bucket][add2];
        let sub1_features = &NETWORK.l0_psqt_weights[bucket][sub1];
        let sub2_features = &NETWORK.l0_psqt_weights[bucket][sub2];

        for i in 0..L1 {
            acc.0[i] = previous_acc.0[i] + add1_features[i] + add2_features[i] - sub1_features[i] - sub2_features[i];
        }
    }
}
