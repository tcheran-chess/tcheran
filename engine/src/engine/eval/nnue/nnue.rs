use crate::{
    chess::{MoveObserver, prelude::*},
    engine::{
        eval::{
            Eval,
            nnue::{
                cache::AccumulatorCache,
                features,
                features::{psqt::accumulator::PsqtStackEntry, threats::accumulator::ThreatsStackEntry},
                inference,
                network::{BUCKET_LAYOUT, L1, Network, OUTPUT_BUCKETS},
            },
        },
        search::MAX_PLIES_ARRAY_SIZE,
    },
};

pub static NETWORK: Network = unsafe { std::mem::transmute(*include_bytes!(env!("NETWORK"))) };

/// A column of the feature-weights matrix.
#[derive(Clone)]
#[repr(C, align(64))]
pub struct Accumulator(pub [i16; L1]);

pub fn input_bucket(king: Square, pov: Player) -> usize {
    BUCKET_LAYOUT[king
        .relative_for(pov)
        .mirrored_horizontally_if(should_mirror(king))
        .idx()]
}

fn output_bucket(game: &Game) -> usize {
    let divisor = 32usize.div_ceil(OUTPUT_BUCKETS);
    (game.board.occupancy().count() as usize - 2) / divisor
}

#[inline]
pub fn should_mirror(king: Square) -> bool {
    king.file() >= File::E
}

pub struct NNUE {
    pub(super) stack: Vec<NetworkStackEntry>,
    pub(super) current_idx: usize,

    pub(super) cache: Box<AccumulatorCache>,
}

#[derive(Clone)]
pub struct NetworkStackEntry {
    pub psqt: PsqtStackEntry,
    pub threats: ThreatsStackEntry,
}

impl NNUE {
    pub fn new() -> Self {
        Self {
            stack: vec![
                NetworkStackEntry {
                    psqt: PsqtStackEntry::default(),
                    threats: ThreatsStackEntry::default(),
                };
                MAX_PLIES_ARRAY_SIZE
            ],
            current_idx: 0,

            cache: AccumulatorCache::new(),
        }
    }

    pub fn setup(&mut self, board: &Board) {
        for pov in Player::ALL {
            self.stack[0].refresh(board, pov, &mut self.cache);
        }

        self.current_idx = 0;
    }

    pub fn push(&mut self) {
        self.current_idx += 1;
        self.stack[self.current_idx].clear();
    }

    pub fn pop(&mut self) {
        self.current_idx -= 1;
    }

    pub fn current_entry(&mut self) -> &mut NetworkStackEntry {
        &mut self.stack[self.current_idx]
    }

    pub fn evaluate(&mut self, game: &Game) -> Eval {
        features::apply_updates(self, game);

        // We should now have a full stack of materialised accumulators, all the way up to our current one
        // so we can evaluate that.
        let current_entry = self.current_entry();
        let (us_psqt, them_psqt) =
            (&current_entry.psqt.accumulators[game.player], &current_entry.psqt.accumulators[!game.player]);
        let (us_threats, them_threats) =
            (&current_entry.threats.accumulators[game.player], &current_entry.threats.accumulators[!game.player]);
        let output = inference::forward(us_psqt, us_threats, them_psqt, them_threats, output_bucket(game));
        Eval(output)
    }
}

impl NetworkStackEntry {
    pub(self) fn clear(&mut self) {
        self.psqt.clear();
        self.threats.clear();
    }

    pub(self) fn refresh(&mut self, board: &Board, pov: Player, cache: &mut AccumulatorCache) {
        self.psqt.refresh(board, pov, cache);
        self.threats.refresh(board, pov);
    }
}

impl MoveObserver for NetworkStackEntry {
    fn init(&mut self, moved_piece: Piece, mv: Move) {
        self.psqt.init(moved_piece, mv);
        self.threats.init(moved_piece, mv);
    }

    fn set(&mut self, sq: Square, piece: Piece) {
        self.psqt.set(sq, piece);
        self.threats.set(sq, piece);
    }

    fn remove(&mut self, sq: Square, piece: Piece) {
        self.psqt.remove(sq, piece);
        self.threats.remove(sq, piece);
    }
}
