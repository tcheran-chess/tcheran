use std::arch::aarch64::*;

pub type I16Vec = int16x8_t;
pub type I32Vec = int32x4_t;

pub mod i16 {
    use super::*;

    pub const LANES: usize = size_of::<I16Vec>() / size_of::<i16>();

    #[target_feature(enable = "neon")]
    pub fn zeroed() -> I16Vec {
        vdupq_n_s16(0)
    }

    #[target_feature(enable = "neon")]
    pub fn splat(n: i16) -> I16Vec {
        vdupq_n_s16(n)
    }

    #[target_feature(enable = "neon")]
    pub unsafe fn load(ptr: *const i16) -> I16Vec {
        unsafe { vld1q_s16(ptr) }
    }

    #[target_feature(enable = "neon")]
    pub fn min(n: I16Vec, min: I16Vec) -> I16Vec {
        vminq_s16(n, min)
    }

    #[target_feature(enable = "neon")]
    pub fn max(n: I16Vec, max: I16Vec) -> I16Vec {
        vmaxq_s16(n, max)
    }

    #[target_feature(enable = "neon")]
    pub fn add_i32(a: I16Vec, b: I16Vec) -> I32Vec {
        let low = vmull_s16(vget_low_s16(a), vget_low_s16(b));
        let high = vmull_high_s16(a, b);
        vaddq_s32(low, high)
    }

    #[target_feature(enable = "neon")]
    pub fn mul(a: I16Vec, b: I16Vec) -> I16Vec {
        vmulq_s16(a, b)
    }
}

pub mod i32 {
    use super::*;

    #[target_feature(enable = "neon")]
    pub fn zeroed() -> I32Vec {
        vdupq_n_s32(0)
    }

    #[target_feature(enable = "neon")]
    pub fn add(a: I32Vec, b: I32Vec) -> I32Vec {
        vaddq_s32(a, b)
    }

    #[target_feature(enable = "neon")]
    pub fn reduce_sum(n: I32Vec) -> i32 {
        vaddvq_s32(n)
    }
}
