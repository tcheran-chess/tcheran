mod scalar;
#[cfg(any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon"))]
mod vectorised;

use crate::engine::eval::nnue::{
    Accumulator,
    network::{L1, L2, L3, Q, SCALE},
};

#[cfg(feature = "nnue-stats")]
pub mod stats {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    use crate::engine::eval::nnue::network::L1;

    pub static NNZ_COUNT: AtomicUsize = AtomicUsize::new(0);
    pub static NNZ_TOTAL: AtomicUsize = AtomicUsize::new(0);

    pub static L0_ACTIVATIONS: [AtomicU64; L1 / 2] = [const { AtomicU64::new(0) }; L1 / 2];

    pub fn log_l0_activations(l0_out: &[u8; L1]) {
        for (i, val) in l0_out.iter().enumerate() {
            if *val != 0 {
                L0_ACTIVATIONS[i % (L1 / 2)].fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn write_activation_counts() {
        use std::io::Write;
        let mut file = std::fs::File::create("activations.txt").unwrap();

        let counts: Vec<String> = L0_ACTIVATIONS
            .iter()
            .map(|c| c.load(Ordering::Relaxed).to_string())
            .collect();

        writeln!(file, "[{}]", counts.join(", ")).unwrap();
        println!("Wrote l0 activation counts to activations.txt");
    }
}

pub fn forward(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    let act_ft = activate_ft(us, them, output_bucket);
    #[cfg(feature = "nnue-stats")]
    stats::log_l0_activations(&act_ft);

    let act_l1 = propagate_l1(&act_ft, output_bucket);
    let act_l2 = propagate_l2(&act_l1, output_bucket);
    let act_l3 = propagate_l3(&act_l2, output_bucket);
    let scaled = i64::from(act_l3) * i64::from(SCALE);
    (scaled / i64::from(Q * Q * Q * Q)) as i32
}

fn activate_ft(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> [u8; L1] {
    cfg_select! {
        any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon") => unsafe {
            vectorised::activate_ft(us, them, output_bucket)
        },
        _ => scalar::activate_ft(us, them, output_bucket),
    }
}

fn propagate_l1(input: &[u8; L1], output_bucket: usize) -> [i32; L2 * 2] {
    cfg_select! {
        any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon") => unsafe {
            vectorised::propagate_l1(input, output_bucket)
        },
        _ => scalar::propagate_l1(input, output_bucket),
    }
}

fn propagate_l2(input: &[i32; L2 * 2], output_bucket: usize) -> [i32; L3] {
    cfg_select! {
        any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon") => unsafe {
            vectorised::propagate_l2(input, output_bucket)
        },
        _ => scalar::propagate_l2(input, output_bucket),
    }
}

fn propagate_l3(input: &[i32; L3], output_bucket: usize) -> i32 {
    cfg_select! {
        any(target_feature = "avx512bw", target_feature = "avx2", target_feature = "neon") => unsafe {
            vectorised::propagate_l3(input, output_bucket)
        },
        _ => scalar::propagate_l3(input, output_bucket),
    }
}
