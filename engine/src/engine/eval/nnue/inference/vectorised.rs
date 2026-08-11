#![expect(unsafe_op_in_unsafe_fn, reason = "")]
#![expect(
    clippy::needless_range_loop,
    reason = "Looping over layers rather than outputs for readability"
)]
#![expect(clippy::cast_possible_truncation, reason = "")]

use crate::engine::eval::nnue::{
    Accumulator, network,
    network::{L1, L2, L3, Q, Q0},
    nnue::NETWORK,
    simd::*,
};

#[allow(clippy::cast_possible_wrap, reason = "Value confirmed not to wrap")]
const Q_BITS: ShiftType = network::Q_BITS as ShiftType;

#[allow(clippy::cast_possible_wrap, reason = "Value confirmed not to wrap")]
const L1_SHIFT: ShiftType = network::L1_SHIFT as ShiftType;

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

#[allow(clippy::cast_possible_wrap, reason = "Won't compile for targets with 16-bit pointers")]
#[allow(clippy::cast_sign_loss, reason = "Guaranteed that indices are >0")]
pub unsafe fn propagate_l1(input: &[u8; L1], output_bucket: usize) -> [i32; L2 * 2] {
    let zero = zeroed_i32();

    let weights = &NETWORK.l1_weights[output_bucket];
    let biases = NETWORK.l1_biases[output_bucket].as_ptr();

    let mut nnz_idxs = [0i16; L1 / 4];
    let mut nnz_count = 0;

    unsafe {
        let mut base = zeroed_i16();

        for i in (0..L1).step_by(I8_LANES) {
            let chunk = reinterpret_u8s_as_i32(load_u8(input.as_ptr().add(i)));
            let (idxs, count) = nnz_indices(chunk);

            store_i16(nnz_idxs.as_mut_ptr().add(nnz_count).cast(), add_i16(base, idxs));
            nnz_count += count as usize;

            base = add_i16(base, splat_i16(I32_LANES as i16));
        }
    }

    #[cfg(feature = "nnue-stats")]
    {
        use std::sync::atomic::Ordering;

        super::stats::NNZ_COUNT.fetch_add(nnz_count, Ordering::Relaxed);
        super::stats::NNZ_TOTAL.fetch_add(L1 / 4, Ordering::Relaxed);
    }

    let (pairs, remainder) = nnz_idxs[..nnz_count].as_chunks::<2>();
    let mut sums = [zero; L2 / I32_LANES];

    #[expect(
        clippy::cast_ptr_alignment,
        reason = "Intentionally loading i32s as blocks of u8s for dpbusd"
    )]
    let input_i32 = input.as_ptr().cast::<i32>();

    for &[idx1, idx2] in pairs {
        let ft1 = reinterpret_i32_as_u8s(input_i32.add(idx1 as usize));
        let ft2 = reinterpret_i32_as_u8s(input_i32.add(idx2 as usize));

        for j in (0..L2).step_by(I32_LANES) {
            let w1 = load_i8(weights[idx1 as usize].as_ptr().add(j * 4));
            let w2 = load_i8(weights[idx2 as usize].as_ptr().add(j * 4));

            let sum = &mut sums[j / I32_LANES];
            *sum = dpbusdx2(*sum, ft1, w1, ft2, w2);
        }
    }

    for &idx in remainder {
        let ft1 = reinterpret_i32_as_u8s(input_i32.add(idx as usize));

        for j in (0..L2).step_by(I32_LANES) {
            let w1 = load_i8(weights[idx as usize].as_ptr().add(j * 4));

            let sum = &mut sums[j / I32_LANES];
            *sum = dpbusd(*sum, ft1, w1);
        }
    }

    let mut output = [0; L2 * 2];
    let q = splat_i32(Q);
    let q2 = splat_i32(Q * Q);

    for i in (0..L2).step_by(I32_LANES) {
        let bias = load_i32(biases.add(i));
        let sum = add_i32(sums[i / I32_LANES], bias);
        let shifted = rshift_i32::<L1_SHIFT>(sum);

        let act1 = lshift_i32::<Q_BITS>(clamp_i32(shifted, zero, q));
        let act2 = clamp_i32(mul_i32(shifted, shifted), zero, q2);

        store_i32(output.as_mut_ptr().add(i), act1);
        store_i32(output.as_mut_ptr().add(i + L2), act2);
    }

    output
}

pub unsafe fn propagate_l2(input: &[i32; L2 * 2], output_bucket: usize) -> [i32; L3] {
    const L3_LANES: usize = L3 / I32_LANES;

    let biases = &NETWORK.l2_biases[output_bucket].as_ptr();

    let mut sums: [I32s; L3_LANES] = [zeroed_i32(); L3_LANES];
    for (lane, acc_lane) in sums.iter_mut().enumerate() {
        *acc_lane = load_i32(biases.add(lane * I32_LANES));
    }

    for l2 in 0..L2 * 2 {
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
