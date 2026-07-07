#![expect(unsafe_op_in_unsafe_fn, reason = "")]

use crate::engine::eval::nnue::{
    Accumulator,
    network::{HIDDEN_SIZE, NETWORK, QA},
};

#[cfg(any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon"))]
#[allow(unused, reason = "May be unused when SIMD is unavailable")]
pub unsafe fn sum_output_weights(
    us: &Accumulator,
    them: &Accumulator,
    output_bucket: usize,
) -> i32 {
    use crate::engine::eval::nnue::simd::*;

    let output_weights = &NETWORK.output_weights[output_bucket];

    let zero = zeroed_i16();
    let qa = splat_i16(QA);

    let us = us.0.as_ptr();
    let them = them.0.as_ptr();
    let us_weights = output_weights.as_ptr();
    let them_weights = output_weights[HIDDEN_SIZE..].as_ptr();

    let mut sums0 = zeroed_i32();
    let mut sums1 = zeroed_i32();
    let mut sums2 = zeroed_i32();
    let mut sums3 = zeroed_i32();

    for i in (0..HIDDEN_SIZE).step_by(4 * I16_LANES) {
        let us0 = load_i16(us.add(i));
        let us1 = load_i16(us.add(i + I16_LANES));
        let us2 = load_i16(us.add(i + 2 * I16_LANES));
        let us3 = load_i16(us.add(i + 3 * I16_LANES));

        let us_clamped0 = clamp_i16(us0, zero, qa);
        let us_clamped1 = clamp_i16(us1, zero, qa);
        let us_clamped2 = clamp_i16(us2, zero, qa);
        let us_clamped3 = clamp_i16(us3, zero, qa);

        let us_weights0 = load_i16(us_weights.add(i));
        let us_weights1 = load_i16(us_weights.add(i + I16_LANES));
        let us_weights2 = load_i16(us_weights.add(i + 2 * I16_LANES));
        let us_weights3 = load_i16(us_weights.add(i + 3 * I16_LANES));

        let them0 = load_i16(them.add(i));
        let them1 = load_i16(them.add(i + I16_LANES));
        let them2 = load_i16(them.add(i + 2 * I16_LANES));
        let them3 = load_i16(them.add(i + 3 * I16_LANES));

        let them_clamped0 = clamp_i16(them0, zero, qa);
        let them_clamped1 = clamp_i16(them1, zero, qa);
        let them_clamped2 = clamp_i16(them2, zero, qa);
        let them_clamped3 = clamp_i16(them3, zero, qa);

        let them_weights0 = load_i16(them_weights.add(i));
        let them_weights1 = load_i16(them_weights.add(i + I16_LANES));
        let them_weights2 = load_i16(them_weights.add(i + 2 * I16_LANES));
        let them_weights3 = load_i16(them_weights.add(i + 3 * I16_LANES));

        sums0 = add_i32(
            sums0,
            add_i32(
                add_i16_into_i32(us_clamped0, mul_i16(us_clamped0, us_weights0)),
                add_i16_into_i32(them_clamped0, mul_i16(them_clamped0, them_weights0)),
            ),
        );

        sums1 = add_i32(
            sums1,
            add_i32(
                add_i16_into_i32(us_clamped1, mul_i16(us_clamped1, us_weights1)),
                add_i16_into_i32(them_clamped1, mul_i16(them_clamped1, them_weights1)),
            ),
        );

        sums2 = add_i32(
            sums2,
            add_i32(
                add_i16_into_i32(us_clamped2, mul_i16(us_clamped2, us_weights2)),
                add_i16_into_i32(them_clamped2, mul_i16(them_clamped2, them_weights2)),
            ),
        );

        sums3 = add_i32(
            sums3,
            add_i32(
                add_i16_into_i32(us_clamped3, mul_i16(us_clamped3, us_weights3)),
                add_i16_into_i32(them_clamped3, mul_i16(them_clamped3, them_weights3)),
            ),
        );
    }

    let sums = add_i32(add_i32(sums0, sums1), add_i32(sums2, sums3));
    reduce_sum(sums)
}
