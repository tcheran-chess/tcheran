pub mod inference;
pub mod network;
mod nnue;
#[cfg(any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon"))]
mod simd;

pub use nnue::{Accumulator, AccumulatorCache, NNUE, NetworkStack};
