#![expect(unsafe_op_in_unsafe_fn, reason = "")]

use std::arch::x86_64::*;

use crate::engine::eval::nnue::network::L0_SHIFT;

pub type U8s = __m256i;
pub type I8s = __m256i;
pub type I16s = __m256i;
pub type I32s = __m256i;

#[inline(always)]
pub unsafe fn store_u8(ptr: *mut u8, n: U8s) {
    _mm256_storeu_si256(ptr.cast(), n)
}

#[inline(always)]
pub unsafe fn load_i8(ptr: *const i8) -> I8s {
    _mm256_loadu_si256(ptr.cast())
}

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
pub unsafe fn shift_left_mul_high_i16(a: I16s, b: I16s) -> I16s {
    const SHIFT: i32 = 16 - L0_SHIFT as i32;
    _mm256_mulhi_epi16(_mm256_slli_epi16::<SHIFT>(a), b)
}

#[inline(always)]
pub unsafe fn packus(l: I16s, r: I16s) -> U8s {
    // From sp00ph - reverse the permutation applied by packus
    _mm256_permute4x64_epi64(_mm256_packus_epi16(l, r), 0xd8)
}

#[inline(always)]
pub unsafe fn zeroed_i32() -> I32s {
    _mm256_setzero_si256()
}

#[inline(always)]
pub unsafe fn splat_i32(n: i32) -> I32s {
    _mm256_set1_epi32(n)
}

#[inline(always)]
pub unsafe fn load_i32(ptr: *const i32) -> I32s {
    _mm256_loadu_si256(ptr.cast())
}

#[inline(always)]
pub unsafe fn store_i32(ptr: *mut i32, n: I32s) {
    _mm256_storeu_si256(ptr.cast(), n)
}

#[inline(always)]
pub unsafe fn add_i32(a: I32s, b: I32s) -> I32s {
    _mm256_add_epi32(a, b)
}

#[inline(always)]
pub unsafe fn mul_i32(a: I32s, b: I32s) -> I32s {
    _mm256_mullo_epi32(a, b)
}

#[inline(always)]
pub unsafe fn min_i32(n: I32s, min: I32s) -> I32s {
    _mm256_min_epi32(n, min)
}

#[inline(always)]
pub unsafe fn max_i32(n: I32s, max: I32s) -> I32s {
    _mm256_max_epi32(n, max)
}

#[inline(always)]
pub unsafe fn rshift_i32<const SHIFT: i32>(n: I32s) -> I32s {
    _mm256_srai_epi32::<SHIFT>(n)
}

#[inline(always)]
pub unsafe fn reinterpret_i32_as_u8s(n: *const i32) -> U8s {
    _mm256_set1_epi32(*n)
}

#[inline(always)]
#[expect(unused, reason = "Not yet used")]
pub unsafe fn dpbusd(acc: I32s, u8s: U8s, i8s: I8s) -> I32s {
    let products = _mm256_maddubs_epi16(u8s, i8s);
    let ones = _mm256_set1_epi16(1);
    let summed = _mm256_madd_epi16(products, ones);
    _mm256_add_epi32(acc, summed)
}

#[inline(always)]
pub unsafe fn dpbusdx2(acc: I32s, u8s1: U8s, i8s1: I8s, u8s2: U8s, i8s2: I8s) -> I32s {
    let p1 = _mm256_maddubs_epi16(u8s1, i8s1);
    let p2 = _mm256_maddubs_epi16(u8s2, i8s2);
    let combined = _mm256_adds_epi16(p1, p2);
    let ones = _mm256_set1_epi16(1);
    _mm256_add_epi32(acc, _mm256_madd_epi16(combined, ones))
}

#[inline(always)]
pub unsafe fn reduce_sum(n: I32s) -> i32 {
    let sums = _mm_add_epi32(_mm256_castsi256_si128(n), _mm256_extracti128_si256(n, 1));
    let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0xee));
    let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0x55));
    _mm_cvtsi128_si32(sums)
}
