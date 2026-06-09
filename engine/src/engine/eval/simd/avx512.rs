use std::arch::x86_64::*;

pub type I16Vec = __m512i;
pub type I32Vec = __m512i;

pub mod i16 {
    use super::*;

    pub const LANES: usize = size_of::<I16Vec>() / size_of::<i16>();

    #[target_feature(enable = "avx512f")]
    pub fn zeroed() -> I16Vec {
        _mm512_setzero_si512()
    }

    #[target_feature(enable = "avx512f")]
    pub fn splat(n: i16) -> I16Vec {
        _mm512_set1_epi16(n)
    }

    #[target_feature(enable = "avx512f")]
    pub fn load(ptr: *const i16) -> I16Vec {
        unsafe { _mm512_loadu_si512(ptr.cast()) }
    }

    #[target_feature(enable = "avx512bw")]
    pub fn min(n: I16Vec, min: I16Vec) -> I16Vec {
        _mm512_min_epi16(n, min)
    }

    #[target_feature(enable = "avx512bw")]
    pub fn max(n: I16Vec, max: I16Vec) -> I16Vec {
        _mm512_max_epi16(n, max)
    }

    #[target_feature(enable = "avx512bw")]
    pub fn add_i32(a: I16Vec, b: I16Vec) -> I32Vec {
        _mm512_madd_epi16(a, b)
    }

    #[target_feature(enable = "avx512bw")]
    pub fn mul(a: I16Vec, b: I16Vec) -> I16Vec {
        _mm512_mullo_epi16(a, b)
    }
}

pub mod i32 {
    use super::*;

    #[target_feature(enable = "avx512f")]
    pub fn zeroed() -> I16Vec {
        _mm512_setzero_si512()
    }

    #[target_feature(enable = "avx512f")]
    pub fn add(a: I32Vec, b: I32Vec) -> I32Vec {
        _mm512_add_epi32(a, b)
    }

    #[target_feature(enable = "avx512f")]
    pub fn reduce_sum(sums: I32Vec) -> i32 {
        _mm512_reduce_add_epi32(sums)
    }
}
