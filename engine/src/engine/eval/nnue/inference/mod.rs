mod scalar;
#[cfg(any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon"))]
mod vectorised;

use crate::engine::eval::nnue::{
    Accumulator,
    network::{L1, L2, L3, Q, SCALE},
};

#[expect(
    clippy::cast_possible_truncation,
    reason = "Truncating down to i32 is safe due to quantisation"
)]
pub fn forward(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    let act_ft = activate_ft(us, them, output_bucket);
    let act_l1 = propagate_l1(&act_ft, output_bucket);
    let act_l2 = propagate_l2(&act_l1, output_bucket);
    let act_l3 = propagate_l3(&act_l2, output_bucket);
    let scaled = i64::from(act_l3) * i64::from(SCALE);
    (scaled / i64::from(Q * Q * Q * Q)) as i32
}

fn activate_ft(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> [u8; L1] {
    cfg_select! {
        any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon") => {
            unsafe { vectorised::activate_ft(us, them, output_bucket) }
        }
        _ => {
            scalar::activate_ft(us, them, output_bucket)
        }
    }
}

fn propagate_l1(input: &[u8; L1], output_bucket: usize) -> [i32; L2] {
    cfg_select! {
        any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon") => {
            unsafe { vectorised::propagate_l1(input, output_bucket) }
        }
        _ => {
            scalar::propagate_l1(input, output_bucket)
        }
    }
}

fn propagate_l2(input: &[i32; L2], output_bucket: usize) -> [i32; L3] {
    cfg_select! {
        any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon") => {
            unsafe { vectorised::propagate_l2(input, output_bucket) }
        }
        _ => {
            scalar::propagate_l2(input, output_bucket)
        }
    }
}

fn propagate_l3(input: &[i32; L3], output_bucket: usize) -> i32 {
    cfg_select! {
        any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon") => {
            unsafe { vectorised::propagate_l3(input, output_bucket) }
        }
        _ => {
            scalar::propagate_l3(input, output_bucket)
        }
    }
}
