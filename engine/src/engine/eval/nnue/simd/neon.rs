#![expect(unsafe_op_in_unsafe_fn, reason = "")]

use std::arch::aarch64::*;

pub type I16s = int16x8_t;
pub type I32s = int32x4_t;

#[inline(always)]
pub unsafe fn zeroed_i16() -> I16s {
    vdupq_n_s16(0)
}

#[inline(always)]
pub unsafe fn splat_i16(n: i16) -> I16s {
    vdupq_n_s16(n)
}

#[inline(always)]
pub unsafe fn load_i16(ptr: *const i16) -> I16s {
    vld1q_s16(ptr)
}

#[inline(always)]
pub unsafe fn min_i16(n: I16s, min: I16s) -> I16s {
    vminq_s16(n, min)
}

#[inline(always)]
pub unsafe fn max_i16(n: I16s, max: I16s) -> I16s {
    vmaxq_s16(n, max)
}

#[inline(always)]
pub unsafe fn add_i16_into_i32(a: I16s, b: I16s) -> I32s {
    let low = vmull_s16(vget_low_s16(a), vget_low_s16(b));
    let high = vmull_high_s16(a, b);
    vaddq_s32(low, high)
}

#[inline(always)]
pub unsafe fn mul_i16(a: I16s, b: I16s) -> I16s {
    vmulq_s16(a, b)
}

#[inline(always)]
pub unsafe fn zeroed_i32() -> I32s {
    vdupq_n_s32(0)
}

#[inline(always)]
pub unsafe fn add_i32(a: I32s, b: I32s) -> I32s {
    vaddq_s32(a, b)
}

#[inline(always)]
pub unsafe fn reduce_sum(n: I32s) -> i32 {
    vaddvq_s32(n)
}
