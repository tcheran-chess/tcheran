use crate::engine::eval::nnue::{Accumulator, HIDDEN_SIZE, NETWORK, QA};

pub fn sum_output_weights(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    cfg_select! {
        target_feature = "avx512bw" => {
            sum_output_weights_simd(us, them, output_bucket)
        },
        target_feature = "avx2" => {
            sum_output_weights_simd(us, them, output_bucket)
        }
        _ => {
            sum_output_weights_scalar(us, them, output_bucket)
        }
    }
}

#[cfg(any(target_feature = "avx512bw", target_feature = "avx2"))]
#[allow(unused, reason = "May be unused when SIMD is unavailable")]
fn sum_output_weights_simd(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    use crate::engine::eval::simd::{
        i16::{self, *},
        i32,
    };

    let output_weights = &NETWORK.output_weights[output_bucket];

    unsafe {
        let zero = zeroed();
        let qa = splat(QA);

        let us = us.0.as_ptr();
        let them = them.0.as_ptr();
        let us_weights = output_weights.as_ptr();
        let them_weights = output_weights[HIDDEN_SIZE..].as_ptr();

        let mut sums0 = i32::zeroed();
        let mut sums1 = i32::zeroed();
        let mut sums2 = i32::zeroed();
        let mut sums3 = i32::zeroed();

        for i in (0..HIDDEN_SIZE).step_by(4 * i16::LANES) {
            let us0 = load(us.add(i));
            let us1 = load(us.add(i + i16::LANES));
            let us2 = load(us.add(i + 2 * i16::LANES));
            let us3 = load(us.add(i + 3 * i16::LANES));

            let us_clamped0 = min(max(us0, zero), qa);
            let us_clamped1 = min(max(us1, zero), qa);
            let us_clamped2 = min(max(us2, zero), qa);
            let us_clamped3 = min(max(us3, zero), qa);

            let us_weights0 = load(us_weights.add(i));
            let us_weights1 = load(us_weights.add(i + i16::LANES));
            let us_weights2 = load(us_weights.add(i + 2 * i16::LANES));
            let us_weights3 = load(us_weights.add(i + 3 * i16::LANES));

            let them0 = load(them.add(i));
            let them1 = load(them.add(i + i16::LANES));
            let them2 = load(them.add(i + 2 * i16::LANES));
            let them3 = load(them.add(i + 3 * i16::LANES));

            let them_clamped0 = min(max(them0, zero), qa);
            let them_clamped1 = min(max(them1, zero), qa);
            let them_clamped2 = min(max(them2, zero), qa);
            let them_clamped3 = min(max(them3, zero), qa);

            let them_weights0 = load(them_weights.add(i));
            let them_weights1 = load(them_weights.add(i + i16::LANES));
            let them_weights2 = load(them_weights.add(i + 2 * i16::LANES));
            let them_weights3 = load(them_weights.add(i + 3 * i16::LANES));

            sums0 = i32::add(
                sums0,
                i32::add(
                    add_i32(us_clamped0, mul(us_clamped0, us_weights0)),
                    add_i32(them_clamped0, mul(them_clamped0, them_weights0)),
                ),
            );

            sums1 = i32::add(
                sums1,
                i32::add(
                    add_i32(us_clamped1, mul(us_clamped1, us_weights1)),
                    add_i32(them_clamped1, mul(them_clamped1, them_weights1)),
                ),
            );

            sums2 = i32::add(
                sums2,
                i32::add(
                    add_i32(us_clamped2, mul(us_clamped2, us_weights2)),
                    add_i32(them_clamped2, mul(them_clamped2, them_weights2)),
                ),
            );

            sums3 = i32::add(
                sums3,
                i32::add(
                    add_i32(us_clamped3, mul(us_clamped3, us_weights3)),
                    add_i32(them_clamped3, mul(them_clamped3, them_weights3)),
                ),
            );
        }

        let sums = i32::add(i32::add(sums0, sums1), i32::add(sums2, sums3));
        i32::reduce_sum(sums)
    }
}

#[allow(unused, reason = "May be unused when SIMD is available")]
fn sum_output_weights_scalar(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
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
