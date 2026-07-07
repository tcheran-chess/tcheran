#![expect(unsafe_op_in_unsafe_fn, reason = "")]

use std::arch::x86_64::*;

pub type I16s = __m512i;
pub type I32s = __m512i;

pub const I16_LANES: usize = size_of::<I16s>() / size_of::<i16>();

#[inline(always)]
pub unsafe fn zeroed_i16() -> I16s {
    _mm512_setzero_si512()
}

#[inline(always)]
pub unsafe fn splat_i16(n: i16) -> I16s {
    _mm512_set1_epi16(n)
}

#[inline(always)]
pub unsafe fn load_i16(ptr: *const i16) -> I16s {
    _mm512_loadu_si512(ptr.cast())
}

#[inline(always)]
pub unsafe fn min_i16(n: I16s, min: I16s) -> I16s {
    _mm512_min_epi16(n, min)
}

#[inline(always)]
pub unsafe fn max_i16(n: I16s, max: I16s) -> I16s {
    _mm512_max_epi16(n, max)
}

#[inline(always)]
pub unsafe fn add_i16_into_i32(a: I16s, b: I16s) -> I32s {
    _mm512_madd_epi16(a, b)
}

#[inline(always)]
pub unsafe fn mul_i16(a: I16s, b: I16s) -> I16s {
    _mm512_mullo_epi16(a, b)
}

#[inline(always)]
pub unsafe fn zeroed_i32() -> I32s {
    _mm512_setzero_si512()
}

#[inline(always)]
pub unsafe fn add_i32(a: I32s, b: I32s) -> I32s {
    _mm512_add_epi32(a, b)
}

#[inline(always)]
pub unsafe fn reduce_sum(sums: I32s) -> i32 {
    _mm512_reduce_add_epi32(sums)
}
