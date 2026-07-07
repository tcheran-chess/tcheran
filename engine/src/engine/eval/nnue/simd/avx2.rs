#![expect(unsafe_op_in_unsafe_fn, reason = "")]

use std::arch::x86_64::*;

pub type I16s = __m256i;
pub type I32s = __m256i;

pub const I16_LANES: usize = size_of::<I16s>() / size_of::<i16>();

#[inline(always)]
pub unsafe fn zeroed_i16() -> I16s {
    _mm256_setzero_si256()
}

#[inline(always)]
pub unsafe fn splat_i16(n: i16) -> I16s {
    _mm256_set1_epi16(n)
}

#[inline(always)]
pub unsafe fn load_i16(ptr: *const i16) -> I16s {
    _mm256_loadu_si256(ptr.cast())
}

#[inline(always)]
pub unsafe fn min_i16(n: I16s, min: I16s) -> I16s {
    _mm256_min_epi16(n, min)
}

#[inline(always)]
pub unsafe fn max_i16(n: I16s, max: I16s) -> I16s {
    _mm256_max_epi16(n, max)
}

#[inline(always)]
pub unsafe fn clamp_i16(n: I16s, min: I16s, max: I16s) -> I16s {
    min_i16(max_i16(n, min), max)
}

#[inline(always)]
pub unsafe fn add_i16_into_i32(a: I16s, b: I16s) -> I32s {
    _mm256_madd_epi16(a, b)
}

#[inline(always)]
pub unsafe fn mul_i16(a: I16s, b: I16s) -> I16s {
    _mm256_mullo_epi16(a, b)
}

#[inline(always)]
pub unsafe fn zeroed_i32() -> I32s {
    _mm256_setzero_si256()
}

#[inline(always)]
pub unsafe fn add_i32(a: I32s, b: I32s) -> I32s {
    _mm256_add_epi32(a, b)
}

#[inline(always)]
pub unsafe fn reduce_sum(n: I32s) -> i32 {
    let sums = _mm_add_epi32(_mm256_castsi256_si128(n), _mm256_extracti128_si256(n, 1));
    let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0xee));
    let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0x55));
    _mm_cvtsi128_si32(sums)
}
