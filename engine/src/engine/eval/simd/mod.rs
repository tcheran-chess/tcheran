#[cfg(target_feature = "avx512bw")]
mod avx512;
#[cfg(target_feature = "avx512bw")]
pub use avx512::sum_output_weights_impl;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512bw")))]
mod avx2;
#[cfg(all(target_feature = "avx2", not(target_feature = "avx512bw")))]
pub use avx2::sum_output_weights_impl;

#[cfg(all(not(target_feature = "avx2"), not(target_feature = "avx512bw")))]
mod generic;
#[cfg(all(not(target_feature = "avx2"), not(target_feature = "avx512bw")))]
pub use generic::sum_output_weights_impl;

use crate::engine::eval::nnue::Accumulator;

pub fn sum_output_weights(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    sum_output_weights_impl(us, them, output_bucket)
}
