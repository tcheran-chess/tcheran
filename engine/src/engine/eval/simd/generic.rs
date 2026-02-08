use crate::engine::eval::nnue::{Accumulator, HIDDEN_SIZE, NETWORK, QA};

#[inline]
pub fn sum_output_weights_impl(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    let output_weights = &NETWORK.output_weights[output_bucket];

    let mut output = 0;

    for (&us, &weight) in us.0.iter().zip(&output_weights[..HIDDEN_SIZE]) {
        let us_clamped = us.clamp(0, QA);
        output += i32::from(us_clamped * weight) * i32::from(us_clamped);
    }

    for (&them, &weight) in them.0.iter().zip(&output_weights[HIDDEN_SIZE..]) {
        let them_clamped = them.clamp(0, QA);
        output += i32::from(them_clamped * weight) * i32::from(them_clamped);
    }

    output
}
