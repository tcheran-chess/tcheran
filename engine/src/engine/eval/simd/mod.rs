cfg_select! {
    target_feature = "avx512bw" => {
        mod avx512;
        pub use avx512::sum_output_weights_impl;
    },
    target_feature = "avx2" => {
        mod avx2;
        pub use avx2::sum_output_weights_impl;
    }
    _ => {
        mod generic;
        pub use generic::sum_output_weights_impl;
    }
}

use crate::engine::eval::nnue::Accumulator;

pub fn sum_output_weights(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    sum_output_weights_impl(us, them, output_bucket)
}
