use crate::engine::eval::nnue::{
    Accumulator,
    network::{HIDDEN_SIZE, NETWORK, QA},
};

#[allow(unused, reason = "May be unused when SIMD is available")]
pub fn sum_output_weights(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    let output_weights = &NETWORK.output_weights[output_bucket];
    let us_output_weights = &output_weights[..HIDDEN_SIZE];
    let them_output_weights = &output_weights[HIDDEN_SIZE..];

    let mut output = 0;

    unsafe {
        for i in 0..HIDDEN_SIZE {
            let us: *const i16 = us.0.as_ptr().add(i).cast();
            let us_clamped = (*us).clamp(0, QA);
            let us_weight: *const i16 = us_output_weights.as_ptr().add(i).cast();

            let them: *const i16 = them.0.as_ptr().add(i).cast();
            let them_clamped = (*them).clamp(0, QA);
            let them_weight: *const i16 = them_output_weights.as_ptr().add(i).cast();

            output += i32::from(us_clamped * *us_weight) * i32::from(us_clamped);
            output += i32::from(them_clamped * *them_weight) * i32::from(them_clamped);
        }
    }

    output
}
