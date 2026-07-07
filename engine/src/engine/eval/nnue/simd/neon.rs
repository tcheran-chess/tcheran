use std::arch::aarch64::*;

pub type I16s = int16x8_t;
pub type I32s = int32x4_t;

pub const I16_LANES: usize = size_of::<I16s>() / size_of::<i16>();

#[target_feature(enable = "neon")]
pub fn zeroed_i16() -> I16s {
    vdupq_n_s16(0)
}

#[target_feature(enable = "neon")]
pub fn splat_i16(n: i16) -> I16s {
    vdupq_n_s16(n)
}

#[target_feature(enable = "neon")]
pub fn load_i16(ptr: *const i16) -> I16s {
    unsafe { vld1q_s16(ptr) }
}

#[target_feature(enable = "neon")]
pub fn min_i16(n: I16s, min: I16s) -> I16s {
    vminq_s16(n, min)
}

#[target_feature(enable = "neon")]
pub fn max_i16(n: I16s, max: I16s) -> I16s {
    vmaxq_s16(n, max)
}

#[target_feature(enable = "neon")]
pub fn add_i16_into_i32(a: I16s, b: I16s) -> I32s {
    let low = vmull_s16(vget_low_s16(a), vget_low_s16(b));
    let high = vmull_high_s16(a, b);
    vaddq_s32(low, high)
}

#[target_feature(enable = "neon")]
pub fn mul_i16(a: I16s, b: I16s) -> I16s {
    vmulq_s16(a, b)
}

#[target_feature(enable = "neon")]
pub fn zeroed_i32() -> I32s {
    vdupq_n_s32(0)
}

#[target_feature(enable = "neon")]
pub fn add_i32(a: I32s, b: I32s) -> I32s {
    vaddq_s32(a, b)
}

#[target_feature(enable = "neon")]
pub fn reduce_sum(n: I32s) -> i32 {
    vaddvq_s32(n)
}
