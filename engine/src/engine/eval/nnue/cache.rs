use crate::{
    chess::prelude::*,
    engine::eval::nnue::{
        Accumulator,
        network::{BUCKET_LAYOUT, INPUT_BUCKETS},
        nnue::{NETWORK, should_mirror},
    },
};

// [pov][mirrored][bucket]
pub(in crate::engine::eval::nnue) struct AccumulatorCache([[[CacheEntry; INPUT_BUCKETS]; 2]; Player::N]);

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

    pub fn get(&mut self, pov: Player, king: Square) -> &mut CacheEntry {
        let king_side = usize::from(should_mirror(king));
        let king_bucket = BUCKET_LAYOUT[king.relative_for(pov)];
        &mut self.0[pov][king_side][king_bucket]
    }
}

pub struct CacheEntry {
    pub value: Accumulator,
    pub board: Board,
}

impl CacheEntry {
    fn new() -> Self {
        Self {
            value: Accumulator(NETWORK.l0_biases),
            board: Board::EMPTY,
        }
    }
}
