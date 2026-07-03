#![expect(unsafe_op_in_unsafe_fn, reason = "")]

use super::{I16s, I32s, max_i16, max_i32, min_i16, min_i32};

pub const I16_LANES: usize = size_of::<I16s>() / size_of::<i16>();
pub const I32_LANES: usize = size_of::<I32s>() / size_of::<i32>();

#[inline(always)]
pub unsafe fn clamp_i16(n: I16s, min: I16s, max: I16s) -> I16s {
    min_i16(max_i16(n, min), max)
}

#[inline(always)]
pub unsafe fn clamp_i32(x: I32s, min: I32s, max: I32s) -> I32s {
    min_i32(max_i32(x, min), max)
}
