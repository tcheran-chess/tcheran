#![allow(unused, reason = "Only here to debug vectorised inference or for unsupported platforms")]
#![expect(
    clippy::needless_range_loop,
    reason = "Looping over layers rather than outputs for readability"
)]
#![expect(clippy::cast_sign_loss, clippy::cast_possible_truncation, reason = "")]

use crate::engine::eval::nnue::{
    Accumulator,
    network::{L0_SHIFT, L1, L1_SHIFT, L2, L3, Q, Q0},
    nnue::NETWORK,
};

pub fn activate_ft(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> [u8; L1] {
    let mut output = [0u8; L1];

    for (side, acc) in [us, them].into_iter().enumerate() {
        let base = side * (L1 / 2);

        for i in 0..(L1 / 2) {
            let left: i16 = acc.0[i];
            let right: i16 = acc.0[i + L1 / 2];

            let left_clamped = left.clamp(0, Q0 as i16) as u16;
            let right_clamped = right.clamp(0, Q0 as i16) as u16;

            let pairwise = i32::from(left_clamped) * i32::from(right_clamped);

            let result: u8 = ((pairwise >> L0_SHIFT) as u8);

            output[base + i] = result;
        }
    }

    output
}

pub unsafe fn propagate_l1(input: &[u8; L1], output_bucket: usize) -> [i32; L2] {
    let mut intermediate = [0i32; L2];

    for l2 in 0..L2 {
        for l1 in 0..L1 {
            let l1_block = l1 / 4;
            let k = l1 % 4;

            intermediate[l2] += i32::from(input[l1])
                * i32::from(NETWORK.l1_weights[output_bucket][l1_block][l2 * 4 + k]);
        }
    }

    let mut output = [0i32; L2];

    for l2 in 0..L2 {
        let bias = NETWORK.l1_biases[output_bucket][l2];

        let out = (intermediate[l2] + bias) >> L1_SHIFT;

        let screlu = out.clamp(0, Q).pow(2);

        output[l2] = screlu;
    }

    output
}

pub unsafe fn propagate_l2(input: &[i32; L2], output_bucket: usize) -> [i32; L3] {
    let mut output = NETWORK.l2_biases[output_bucket];

    for l2 in 0..L2 {
        let i = input[l2];

        for l3 in 0..L3 {
            let weight = NETWORK.l2_weights[output_bucket][l2][l3];
            output[l3] += i * weight;
        }
    }

    for l3 in 0..L3 {
        output[l3] = output[l3].clamp(0, Q * Q * Q);
    }

    output
}

pub unsafe fn propagate_l3(input: &[i32; L3], output_bucket: usize) -> i32 {
    let mut output = NETWORK.l3_biases[output_bucket];

    for l3 in 0..L3 {
        output += input[l3] * NETWORK.l3_weights[output_bucket][l3];
    }

    output
}
