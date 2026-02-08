use std::arch::x86_64::*;

use crate::engine::eval::nnue::{Accumulator, HIDDEN_SIZE, NETWORK, QA};

const STEP: usize = 64;
const _: () = assert!(HIDDEN_SIZE.is_multiple_of(STEP));

#[inline]
pub fn sum_output_weights_impl(us: &Accumulator, them: &Accumulator, output_bucket: usize) -> i32 {
    unsafe {
        let zero = _mm256_setzero_si256();
        let qa = _mm256_set1_epi16(QA);

        let output_weights = &NETWORK.output_weights[output_bucket];

        let us = us.0.as_ptr().cast::<__m256i>();
        let them = them.0.as_ptr().cast::<__m256i>();
        let us_weights = output_weights.as_ptr().cast::<__m256i>();
        let them_weights = output_weights[HIDDEN_SIZE..].as_ptr().cast::<__m256i>();

        let mut sums0 = _mm256_setzero_si256();
        let mut sums1 = _mm256_setzero_si256();
        let mut sums2 = _mm256_setzero_si256();
        let mut sums3 = _mm256_setzero_si256();

        for i in 0..HIDDEN_SIZE / STEP {
            let us0 = _mm256_loadu_si256(us.add(4 * i + 0));
            let us1 = _mm256_loadu_si256(us.add(4 * i + 1));
            let us2 = _mm256_loadu_si256(us.add(4 * i + 2));
            let us3 = _mm256_loadu_si256(us.add(4 * i + 3));

            let us_clamped0 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us0));
            let us_clamped1 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us1));
            let us_clamped2 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us2));
            let us_clamped3 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us3));

            let us_weights0 = _mm256_loadu_si256(us_weights.add(4 * i + 0));
            let us_weights1 = _mm256_loadu_si256(us_weights.add(4 * i + 1));
            let us_weights2 = _mm256_loadu_si256(us_weights.add(4 * i + 2));
            let us_weights3 = _mm256_loadu_si256(us_weights.add(4 * i + 3));

            let them0 = _mm256_loadu_si256(them.add(4 * i + 0));
            let them1 = _mm256_loadu_si256(them.add(4 * i + 1));
            let them2 = _mm256_loadu_si256(them.add(4 * i + 2));
            let them3 = _mm256_loadu_si256(them.add(4 * i + 3));

            let them_clamped0 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them0));
            let them_clamped1 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them1));
            let them_clamped2 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them2));
            let them_clamped3 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them3));

            let them_weights0 = _mm256_loadu_si256(them_weights.add(4 * i + 0));
            let them_weights1 = _mm256_loadu_si256(them_weights.add(4 * i + 1));
            let them_weights2 = _mm256_loadu_si256(them_weights.add(4 * i + 2));
            let them_weights3 = _mm256_loadu_si256(them_weights.add(4 * i + 3));

            sums0 = _mm256_add_epi32(
                sums0,
                _mm256_add_epi32(
                    _mm256_madd_epi16(us_clamped0, _mm256_mullo_epi16(us_clamped0, us_weights0)),
                    _mm256_madd_epi16(
                        them_clamped0,
                        _mm256_mullo_epi16(them_clamped0, them_weights0),
                    ),
                ),
            );
            sums1 = _mm256_add_epi32(
                sums1,
                _mm256_add_epi32(
                    _mm256_madd_epi16(us_clamped1, _mm256_mullo_epi16(us_clamped1, us_weights1)),
                    _mm256_madd_epi16(
                        them_clamped1,
                        _mm256_mullo_epi16(them_clamped1, them_weights1),
                    ),
                ),
            );
            sums2 = _mm256_add_epi32(
                sums2,
                _mm256_add_epi32(
                    _mm256_madd_epi16(us_clamped2, _mm256_mullo_epi16(us_clamped2, us_weights2)),
                    _mm256_madd_epi16(
                        them_clamped2,
                        _mm256_mullo_epi16(them_clamped2, them_weights2),
                    ),
                ),
            );
            sums3 = _mm256_add_epi32(
                sums3,
                _mm256_add_epi32(
                    _mm256_madd_epi16(us_clamped3, _mm256_mullo_epi16(us_clamped3, us_weights3)),
                    _mm256_madd_epi16(
                        them_clamped3,
                        _mm256_mullo_epi16(them_clamped3, them_weights3),
                    ),
                ),
            );
        }

        let sums = _mm256_add_epi32(_mm256_add_epi32(sums0, sums1), _mm256_add_epi32(sums2, sums3));

        let sums = _mm_add_epi32(_mm256_castsi256_si128(sums), _mm256_extracti128_si256(sums, 1));
        let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0xee));
        let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0x55));
        _mm_cvtsi128_si32(sums)
    }
}
