mod scalar;
mod vectorised;

use crate::engine::eval::nnue::{
    Accumulator,
    network::{NETWORK, QA, QB, SCALE},
};

pub fn forward(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    let mut output = sum_output_weights(us, them, output_bucket);

    // Reduce quantization from QA * QA * QB to QA * QB.
    output /= i32::from(QA);

    // Add bias.
    let output_bias = &NETWORK.output_bias[output_bucket];
    output += i32::from(*output_bias);

    // Apply eval scale.
    output *= SCALE;

    // Remove quantisation altogether.
    output /= i32::from(QA) * i32::from(QB);

    output
}

fn sum_output_weights(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    cfg_select! {
        any(target_feature = "avx512bw",
        target_feature = "avx2",
        target_feature = "neon") => {
            unsafe { vectorised::sum_output_weights(us, them, output_bucket) }
        }
        _ => {
            scalar::sum_output_weights(us, them, output_bucket)
        }
    }
}
