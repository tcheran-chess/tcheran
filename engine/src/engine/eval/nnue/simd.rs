#![expect(unsafe_op_in_unsafe_fn, reason = "")]
#![allow(unused, reason = "Some functions may not be used on all arches")]

use crate::engine::eval::nnue::network;

macro_rules! simd {
    (
        avx2 $avx2block:tt
        avx512 $avx512block:tt
        neon $neonblock:tt
    ) => {
        cfg_select! {
            target_feature = "avx512bw" => $avx512block,
            target_feature = "avx2" => $avx2block,
            target_feature = "neon" => $neonblock,
        }
    };
}

simd!(
    avx2 {
        use std::arch::x86_64::*;
    }
    avx512 {
        use std::arch::x86_64::*;
    }
    neon {
        use std::arch::aarch64::*;
    }
);

pub type ShiftType = simd!(
    avx2 { i32 }
    avx512 { u32 }
    neon { i32 }
);

pub type U8s = simd!(
    avx2 { __m256i }
    avx512 { __m512i }
    neon { uint8x16_t }
);

pub type I8s = simd!(
    avx2 { __m256i }
    avx512 { __m512i }
    neon { int8x16_t }
);

pub type I16s = simd!(
    avx2 { __m256i }
    avx512 { __m512i }
    neon { int16x8_t }
);

pub type I32s = simd!(
    avx2 { __m256i }
    avx512 { __m512i }
    neon { int32x4_t }
);

pub type UI8s = simd!(
    avx2 { __m256i }
    avx512 { __m512i }
    neon { int8x16_t }
);

pub const I8_LANES: usize = size_of::<I8s>() / size_of::<i8>();
pub const I16_LANES: usize = size_of::<I16s>() / size_of::<i16>();
pub const I32_LANES: usize = size_of::<I32s>() / size_of::<i32>();

#[inline(always)]
pub unsafe fn load_u8s(ptr: *const u8) -> U8s {
    simd!(
        avx2 { _mm256_loadu_si256(ptr.cast()) }
        avx512 { _mm512_loadu_si512(ptr.cast()) }
        neon { vld1q_u8(ptr) }
    )
}

#[inline(always)]
pub unsafe fn store_u8s(ptr: *mut u8, n: U8s) {
    simd!(
        avx2 { _mm256_storeu_si256(ptr.cast(), n) }
        avx512 { _mm512_storeu_si512(ptr.cast(), n) }
        neon { vst1q_u8(ptr, n) }
    );
}

#[inline(always)]
pub unsafe fn load_i8s(ptr: *const i8) -> I8s {
    simd!(
        avx2 { _mm256_loadu_si256(ptr.cast()) }
        avx512 { _mm512_loadu_si512(ptr.cast()) }
        neon { vld1q_s8(ptr) }
    )
}

#[inline(always)]
pub unsafe fn zero_i16s() -> I16s {
    simd!(
        avx2 { _mm256_setzero_si256() }
        avx512 { _mm512_setzero_si512() }
        neon { vdupq_n_s16(0) }
    )
}

#[inline(always)]
pub unsafe fn splat_i16(n: i16) -> I16s {
    simd!(
        avx2 { _mm256_set1_epi16(n) }
        avx512 { _mm512_set1_epi16(n) }
        neon { vdupq_n_s16(n) }
    )
}

#[inline(always)]
pub unsafe fn load_i16s(ptr: *const i16) -> I16s {
    simd!(
        avx2 { _mm256_loadu_si256(ptr.cast()) }
        avx512 { _mm512_loadu_si512(ptr.cast()) }
        neon { vld1q_s16(ptr) }
    )
}

#[inline(always)]
pub unsafe fn store_i16s(ptr: *mut i16, n: I16s) {
    simd!(
        avx2 { _mm256_storeu_si256(ptr.cast(), n) }
        avx512 { _mm512_storeu_si512(ptr.cast(), n) }
        neon { vst1q_s16(ptr, n) }
    );
}

#[inline(always)]
pub unsafe fn add_i16s(a: I16s, b: I16s) -> I16s {
    simd!(
        avx2 { _mm256_add_epi16(a, b) }
        avx512 { _mm512_add_epi16(a, b) }
        neon { vaddq_s16(a, b) }
    )
}

#[inline(always)]
pub unsafe fn min_i16s(n: I16s, min: I16s) -> I16s {
    simd!(
        avx2 { _mm256_min_epi16(n, min) }
        avx512 { _mm512_min_epi16(n, min) }
        neon { vminq_s16(n, min) }
    )
}

#[inline(always)]
pub unsafe fn max_i16s(n: I16s, max: I16s) -> I16s {
    simd!(
        avx2 { _mm256_max_epi16(n, max) }
        avx512 { _mm512_max_epi16(n, max) }
        neon { vmaxq_s16(n, max) }
    )
}

#[inline(always)]
pub unsafe fn clamp_i16s(n: I16s, min: I16s, max: I16s) -> I16s {
    min_i16s(max_i16s(n, min), max)
}

#[inline(always)]
pub unsafe fn shift_left_mul_high_i16s(a: I16s, b: I16s) -> I16s {
    const L0_SHIFT: ShiftType = network::L0_SHIFT as ShiftType;

    simd!(
        avx2 {{
            const SHIFT: i32 = 16 - L0_SHIFT;
            _mm256_mulhi_epi16(_mm256_slli_epi16::<SHIFT>(a), b)
        }}
        avx512 {{
            const SHIFT: u32 = 16 - L0_SHIFT;
            _mm512_mulhi_epi16(_mm512_slli_epi16::<SHIFT>(a), b)
        }}
        neon {{
            const SHIFT: i32 = 16 - L0_SHIFT - 1;
            vqdmulhq_s16(vshlq_n_s16::<SHIFT>(a), b)
        }}
    )
}

#[inline(always)]
pub unsafe fn packus(l: I16s, r: I16s) -> U8s {
    // From sp00ph - reverse the permutation applied by packus
    simd!(
        avx2 { _mm256_permute4x64_epi64(_mm256_packus_epi16(l, r), 0xd8) }
        avx512 {{
            let lo = _mm512_shuffle_i64x2(l, r, 136);
            let hi = _mm512_shuffle_i64x2(l, r, 221);
            _mm512_packus_epi16(lo, hi)
        }}
        neon { vqmovun_high_s16(vqmovun_s16(l), r) }
    )
}

#[inline(always)]
pub unsafe fn zeroed_i32s() -> I32s {
    simd!(
        avx2 { _mm256_setzero_si256() }
        avx512 { _mm512_setzero_si512() }
        neon { vdupq_n_s32(0) }
    )
}

#[inline(always)]
pub unsafe fn splat_i32(n: i32) -> I32s {
    simd!(
        avx2 { _mm256_set1_epi32(n) }
        avx512 { _mm512_set1_epi32(n) }
        neon { vdupq_n_s32(n) }
    )
}

#[inline(always)]
pub unsafe fn load_i32s(ptr: *const i32) -> I32s {
    simd!(
        avx2 { _mm256_loadu_si256(ptr.cast()) }
        avx512 { _mm512_loadu_si512(ptr.cast()) }
        neon { vld1q_s32(ptr) }
    )
}

#[inline(always)]
pub unsafe fn store_i32s(ptr: *mut i32, n: I32s) {
    simd!(
        avx2 { _mm256_storeu_si256(ptr.cast(), n) }
        avx512 { _mm512_storeu_si512(ptr.cast(), n) }
        neon { vst1q_s32(ptr, n) }
    );
}

#[inline(always)]
pub unsafe fn add_i32s(a: I32s, b: I32s) -> I32s {
    simd!(
        avx2 { _mm256_add_epi32(a, b) }
        avx512 { _mm512_add_epi32(a, b) }
        neon { vaddq_s32(a, b) }
    )
}

#[inline(always)]
pub unsafe fn mul_i32s(a: I32s, b: I32s) -> I32s {
    simd!(
        avx2 { _mm256_mullo_epi32(a, b) }
        avx512 { _mm512_mullo_epi32(a, b) }
        neon { vmulq_s32(a, b) }
    )
}

#[inline(always)]
pub unsafe fn min_i32s(n: I32s, min: I32s) -> I32s {
    simd!(
        avx2 { _mm256_min_epi32(n, min) }
        avx512 { _mm512_min_epi32(n, min) }
        neon { vminq_s32(n, min) }
    )
}

#[inline(always)]
pub unsafe fn max_i32s(n: I32s, max: I32s) -> I32s {
    simd!(
        avx2 { _mm256_max_epi32(n, max) }
        avx512 { _mm512_max_epi32(n, max) }
        neon { vmaxq_s32(n, max) }
    )
}

#[inline(always)]
pub unsafe fn clamp_i32s(x: I32s, min: I32s, max: I32s) -> I32s {
    min_i32s(max_i32s(x, min), max)
}

#[inline(always)]
pub unsafe fn lshift_i32s<const SHIFT: ShiftType>(n: I32s) -> I32s {
    simd!(
        avx2 { _mm256_slli_epi32::<SHIFT>(n) }
        avx512 { _mm512_slli_epi32::<SHIFT>(n) }
        neon { vshlq_n_s32::<SHIFT>(n) }
    )
}

#[inline(always)]
pub unsafe fn rshift_i32s<const SHIFT: ShiftType>(n: I32s) -> I32s {
    simd!(
        avx2 { _mm256_srai_epi32::<SHIFT>(n) }
        avx512 { _mm512_srai_epi32::<SHIFT>(n) }
        neon { vshrq_n_s32::<SHIFT>(n) }
    )
}

#[inline(always)]
pub unsafe fn reinterpret_i32_as_u8s(n: *const i32) -> UI8s {
    simd!(
        avx2 { _mm256_set1_epi32(*n) }
        avx512 { _mm512_set1_epi32(*n) }
        neon { vreinterpretq_s8_s32(vdupq_n_s32(*n)) }
    )
}

#[inline(always)]
pub unsafe fn reinterpret_u8s_as_i32s(n: U8s) -> I32s {
    simd!(
        avx2 { n }
        avx512 { n }
        neon { vreinterpretq_s32_u8(n) }
    )
}

#[inline(always)]
#[allow(unused, reason = "Only used on some platforms")]
pub unsafe fn dpbusd(acc: I32s, u8s: UI8s, i8s: UI8s) -> I32s {
    simd!(
        avx2 {{
            let products = _mm256_maddubs_epi16(u8s, i8s);
            let ones = _mm256_set1_epi16(1);
            let summed = _mm256_madd_epi16(products, ones);
            _mm256_add_epi32(acc, summed)
        }}
        avx512 {
            cfg_select! {
                target_feature = "avx512vnni" => unsafe { _mm512_dpbusd_epi32(acc, u8s, i8s) },
                _ => {{
                    let products = _mm512_maddubs_epi16(u8s, i8s);
                    let ones = _mm512_set1_epi16(1);
                    let summed = _mm512_madd_epi16(products, ones);
                    _mm512_add_epi32(acc, summed)
                }}
            }
        }
        neon {
            cfg_select! {
                target_feature = "dotprod" => vdotq_s32(acc, u8s, i8s),
                _ => {{
                    let lo = vmull_s8(vget_low_s8(u8s), vget_low_s8(i8s));
                    let hi = vmull_high_s8(u8s, i8s);
                    let pairwise = vpaddq_s16(lo, hi);
                    vpadalq_s16(acc, pairwise)
                }}
            }
        }
    )
}

#[inline(always)]
pub unsafe fn dpbusdx2(acc: I32s, u8s1: UI8s, i8s1: UI8s, u8s2: UI8s, i8s2: UI8s) -> I32s {
    simd!(
        avx2 {{
            let p1 = _mm256_maddubs_epi16(u8s1, i8s1);
            let p2 = _mm256_maddubs_epi16(u8s2, i8s2);
            let combined = _mm256_adds_epi16(p1, p2);
            let ones = _mm256_set1_epi16(1);
            _mm256_add_epi32(acc, _mm256_madd_epi16(combined, ones))
        }}
        avx512 {{
            let p1 = _mm512_maddubs_epi16(u8s1, i8s1);
            let p2 = _mm512_maddubs_epi16(u8s2, i8s2);
            let combined = _mm512_adds_epi16(p1, p2);
            let ones = _mm512_set1_epi16(1);
            _mm512_add_epi32(acc, _mm512_madd_epi16(combined, ones))
        }}
        neon {{
            cfg_select! {
                target_feature = "dotprod" => {
                    dpbusd(dpbusd(acc, u8s1, i8s1), u8s2, i8s2)
                }
                _ => {
                    let lo1 = vmull_s8(vget_low_s8(u8s1), vget_low_s8(i8s1));
                    let hi1 = vmull_high_s8(u8s1, i8s1);
                    let p1 = vpaddq_s16(lo1, hi1);

                    let lo2 = vmull_s8(vget_low_s8(u8s2), vget_low_s8(i8s2));
                    let hi2 = vmull_high_s8(u8s2, i8s2);
                    let p2 = vpaddq_s16(lo2, hi2);

                    vpadalq_s16(acc, vaddq_s16(p1, p2))
                }
            }
        }}
    )
}

#[inline(always)]
pub unsafe fn reduce_sum(n: I32s) -> i32 {
    simd!(
        avx2 {{
            let sums = _mm_add_epi32(_mm256_castsi256_si128(n), _mm256_extracti128_si256(n, 1));
            let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0xee));
            let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0x55));
            _mm_cvtsi128_si32(sums)
        }}
        avx512 { _mm512_reduce_add_epi32(n) }
        neon { vaddvq_s32(n) }
    )
}

macro_rules! nnz_table {
    () => {
        static NNZ_TABLE: [[i16; 8]; 256] = {
            let mut table = [[0i16; 8]; 256];

            let mut i = 0;
            while i < 256 {
                let mut j = i;
                let mut k = 0;
                while j != 0 {
                    table[i][k] = j.trailing_zeros() as i16;
                    j &= j - 1;
                    k += 1;
                }
                i += 1;
            }

            table
        };
    };
}

#[inline(always)]
pub unsafe fn nnz_indices(n: I32s) -> (I16s, u16) {
    simd!(
        avx2 {{
            nnz_table!();

            let nnz_mask = _mm256_movemask_ps(_mm256_castsi256_ps(_mm256_cmpgt_epi32(n, zeroed_i32s())));
            let idxs = unsafe { _mm_loadu_si128(NNZ_TABLE[nnz_mask as usize].as_ptr().cast()) };

            (
                _mm256_castsi128_si256(idxs),
                nnz_mask.count_ones() as u16,
            )
        }}
        avx512 {{
            let nnz_mask = _mm512_test_epi32_mask(n, n);
            let idxs: [i16; 16] = std::array::from_fn(|i| i as i16);
            let idxs = unsafe { _mm256_loadu_si256(idxs.as_ptr().cast()) };

            (
                _mm512_castsi256_si512(_mm256_maskz_compress_epi16(nnz_mask, idxs)),
                nnz_mask.count_ones() as u16,
            )
        }}
        neon {{
            nnz_table!();

            let mask = vtstq_s32(n, n);
            let bitmask = vaddvq_u32(vandq_u32(mask, unsafe { vld1q_u32([1, 2, 4, 8].as_ptr()) }));
            let idxs = unsafe { vld1q_s16(NNZ_TABLE[bitmask as usize].as_ptr()) };
            (idxs, bitmask.count_ones() as u16)
        }}
    )
}
