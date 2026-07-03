#![expect(unsafe_op_in_unsafe_fn, reason = "")]
#![expect(
    clippy::needless_range_loop,
    reason = "Looping over layers rather than outputs for readability"
)]
#![expect(clippy::cast_possible_truncation, reason = "")]

use crate::engine::eval::nnue::{
    Accumulator,
    network::{L1, L1_SHIFT, L2, L3, Q, Q0},
    nnue::NETWORK,
    simd::*,
};

#[cfg(any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon"))]
#[allow(unused, reason = "May be unused when SIMD is unavailable")]
pub unsafe fn activate_ft(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> [u8; L1] {
    let mut output: [u8; L1] = [0; L1];

    let zero = zeroed_i16();
    let q0 = splat_i16(Q0 as i16);

    let us = us.0.as_ptr();
    let them = them.0.as_ptr();

    for i in (0..L1 / 2).step_by(2 * I16_LANES) {
        let us0 = load_i16(us.add(i));
        let us1 = load_i16(us.add(i + L1 / 2));
        let us2 = load_i16(us.add(i + I16_LANES));
        let us3 = load_i16(us.add(i + L1 / 2 + I16_LANES));

        let us_clamped0 = clamp_i16(us0, zero, q0);
        let us_clamped1 = clamp_i16(us1, zero, q0);
        let us_clamped2 = clamp_i16(us2, zero, q0);
        let us_clamped3 = clamp_i16(us3, zero, q0);

        let them0 = load_i16(them.add(i));
        let them1 = load_i16(them.add(i + L1 / 2));
        let them2 = load_i16(them.add(i + I16_LANES));
        let them3 = load_i16(them.add(i + L1 / 2 + I16_LANES));

        let them_clamped0 = clamp_i16(them0, zero, q0);
        let them_clamped1 = clamp_i16(them1, zero, q0);
        let them_clamped2 = clamp_i16(them2, zero, q0);
        let them_clamped3 = clamp_i16(them3, zero, q0);

        let us_pair1 = shift_left_mul_high_i16(us_clamped0, us_clamped1);
        let us_pair2 = shift_left_mul_high_i16(us_clamped2, us_clamped3);

        let them_pair1 = shift_left_mul_high_i16(them_clamped0, them_clamped1);
        let them_pair2 = shift_left_mul_high_i16(them_clamped2, them_clamped3);

        let packed1 = packus(us_pair1, us_pair2);
        let packed2 = packus(them_pair1, them_pair2);

        store_u8(output.as_mut_ptr().add(i).cast(), packed1);
        store_u8(output.as_mut_ptr().add(i + L1 / 2).cast(), packed2);
    }

    output
}

pub unsafe fn propagate_l1(input: &[u8; L1], output_bucket: usize) -> [i32; L2] {
    const N_TILES: usize = L1 / 4;
    const N_CHUNKS: usize = L2 / I32_LANES;
    const WEIGHT_STRIDE: usize = I32_LANES * 4;

    let zero = zeroed_i32();

    let weights = &NETWORK.l1_weights[output_bucket];
    let biases = NETWORK.l1_biases[output_bucket].as_ptr();

    let mut sums = [zero; N_CHUNKS];
    let mut output = [0; L2];

    #[expect(
        clippy::cast_ptr_alignment,
        reason = "Intentionally loading i32s as blocks of u8s for dpbusd"
    )]
    let input_i32 = input.as_ptr().cast::<i32>();

    for i in (0..N_TILES).step_by(2) {
        let ft1 = reinterpret_i32_as_u8s(input_i32.add(i));
        let ft2 = reinterpret_i32_as_u8s(input_i32.add(i + 1));

        for r in 0..N_CHUNKS {
            let w1 = load_i8(weights[i].as_ptr().add(r * WEIGHT_STRIDE));
            let w2 = load_i8(weights[i + 1].as_ptr().add(r * WEIGHT_STRIDE));

            sums[r] = dpbusdx2(sums[r], ft1, w1, ft2, w2);
        }
    }

    let q = splat_i32(Q);

    for (lane, acc_lane) in sums.iter().enumerate() {
        let bias = load_i32(biases.add(lane * I32_LANES));
        let sum = add_i32(*acc_lane, bias);
        let shifted = rshift_i32::<L1_SHIFT>(sum);

        let clamped = clamp_i32(shifted, zero, q);
        let screlu = mul_i32(clamped, clamped);

        store_i32(output.as_mut_ptr().add(lane * I32_LANES), screlu);
    }

    output
}

pub unsafe fn propagate_l2(input: &[i32; L2], output_bucket: usize) -> [i32; L3] {
    const L3_LANES: usize = L3 / I32_LANES;

    let biases = &NETWORK.l2_biases[output_bucket].as_ptr();

    let mut sums: [I32s; L3_LANES] = [zeroed_i32(); L3_LANES];
    for (lane, acc_lane) in sums.iter_mut().enumerate() {
        *acc_lane = load_i32(biases.add(lane * I32_LANES));
    }

    for l2 in 0..L2 {
        let i = splat_i32(input[l2]);

        for l3 in 0..L3_LANES {
            let weights = load_i32(
                NETWORK.l2_weights[output_bucket][l2]
                    .as_ptr()
                    .add(l3 * I32_LANES),
            );
            sums[l3] = add_i32(sums[l3], mul_i32(i, weights));
        }
    }

    let mut output = [0; L3];

    let zero = zeroed_i32();
    let q3 = splat_i32(Q * Q * Q);

    for l3 in 0..L3_LANES {
        let clamped = clamp_i32(*sums.as_ptr().add(l3), zero, q3);
        store_i32(output.as_mut_ptr().add(l3 * I32_LANES), clamped);
    }

    output
}

pub unsafe fn propagate_l3(input: &[i32; L3], output_bucket: usize) -> i32 {
    let mut sum = splat_i32(0);

    let input = input.as_ptr();
    let weights = NETWORK.l3_weights[output_bucket].as_ptr();
    let bias = NETWORK.l3_biases[output_bucket];

    for l3 in (0..L3).step_by(I32_LANES) {
        let inp = load_i32(input.add(l3));
        let weight = load_i32(weights.add(l3));
        sum = add_i32(sum, mul_i32(inp, weight));
    }

    reduce_sum(sum) + bias
}
