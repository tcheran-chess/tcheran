#![expect(unsafe_op_in_unsafe_fn, reason = "")]

use std::arch::aarch64::*;

use crate::engine::eval::nnue::network::L0_SHIFT;

pub type U8s = uint8x16_t;
pub type I8s = int8x16_t;
pub type I16s = int16x8_t;
pub type I32s = int32x4_t;

#[inline(always)]
pub unsafe fn store_u8(ptr: *mut u8, n: U8s) {
    vst1q_u8(ptr, n);
}

#[inline(always)]
pub unsafe fn load_i8(ptr: *const i8) -> I8s {
    vld1q_s8(ptr)
}

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
pub unsafe fn shift_left_mul_high_i16(a: I16s, b: I16s) -> I16s {
    const SHIFT: i32 = 16 - L0_SHIFT.cast_signed() - 1;
    vqdmulhq_s16(vshlq_n_s16::<SHIFT>(a), b)
}

#[inline(always)]
pub unsafe fn packus(l: I16s, r: I16s) -> U8s {
    vqmovun_high_s16(vqmovun_s16(l), r)
}

#[inline(always)]
pub unsafe fn zeroed_i32() -> I32s {
    vdupq_n_s32(0)
}

#[inline(always)]
pub unsafe fn splat_i32(n: i32) -> I32s {
    vdupq_n_s32(n)
}

#[inline(always)]
pub unsafe fn load_i32(ptr: *const i32) -> I32s {
    vld1q_s32(ptr)
}

#[inline(always)]
pub unsafe fn store_i32(ptr: *mut i32, n: I32s) {
    vst1q_s32(ptr, n);
}

#[inline(always)]
pub unsafe fn add_i32(a: I32s, b: I32s) -> I32s {
    vaddq_s32(a, b)
}

#[inline(always)]
pub unsafe fn mul_i32(a: I32s, b: I32s) -> I32s {
    vmulq_s32(a, b)
}

#[inline(always)]
pub unsafe fn min_i32(n: I32s, min: I32s) -> I32s {
    vminq_s32(n, min)
}

#[inline(always)]
pub unsafe fn max_i32(n: I32s, max: I32s) -> I32s {
    vmaxq_s32(n, max)
}

#[inline(always)]
pub unsafe fn rshift_i32<const SHIFT: i32>(n: I32s) -> I32s {
    vshrq_n_s32::<SHIFT>(n)
}

#[inline(always)]
pub unsafe fn reinterpret_i32_as_u8s(n: *const i32) -> U8s {
    vreinterpretq_u8_s32(vdupq_n_s32(*n))
}

#[inline(always)]
pub unsafe fn dpbusd(acc: I32s, u8s: U8s, i8s: I8s) -> I32s {
    cfg_select! {
        // NEON dotprod is unstable, so for now we use inline ASM.
        target_feature = "dotprod" => {
            {
                let mut result = acc;
                std::arch::asm!(
                    "sdot {0:v}.4s, {1:v}.16b, {2:v}.16b",
                    inlateout(vreg) result,
                    in(vreg) u8s,
                    in(vreg) i8s,
                    options(pure, nomem, nostack),
                );
                result
            }
        }
        _ => {
            let lo = vmull_s8(vget_low_s8(u8s), vget_low_s8(i8s));
            let hi = vmull_high_s8(u8s, i8s);
            let pairwise = vpaddq_s16(lo, hi);
            vpadalq_s16(acc, pairwise)
        }
    }
}

#[inline(always)]
pub unsafe fn dpbusdx2(acc: I32s, u8s1: U8s, i8s1: I8s, u8s2: U8s, i8s2: I8s) -> I32s {
    cfg_select! {
        target_feature = "dotprod" => {
            dpbusd(dpbusd(acc, u8s1, i8s1), u8s2, i8s2)
        }
        _ => {
            let lo1 = vmull_s8(vget_low_s8(u1), vget_low_s8(w1));
            let hi1 = vmull_high_s8(u1, w1);
            let p1 = vpaddq_s16(lo1, hi1);

            let lo2 = vmull_s8(vget_low_s8(u2), vget_low_s8(w2));
            let hi2 = vmull_high_s8(u2, w2);
            let p2 = vpaddq_s16(lo2, hi2);

            vpadalq_s16(acc, vaddq_s16(p1, p2))
        }
    }
}

#[inline(always)]
pub unsafe fn reduce_sum(n: I32s) -> i32 {
    vaddvq_s32(n)
}
