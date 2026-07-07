#![expect(unsafe_op_in_unsafe_fn, reason = "")]

use super::{I16s, max_i16, min_i16};

pub const I16_LANES: usize = size_of::<I16s>() / size_of::<i16>();

#[inline(always)]
pub unsafe fn clamp_i16(n: I16s, min: I16s, max: I16s) -> I16s {
    min_i16(max_i16(n, min), max)
}
