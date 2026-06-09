use std::arch::x86_64::*;

pub type I16Vec = __m256i;
pub type I32Vec = __m256i;

pub mod i16 {
    use super::*;

    pub const LANES: usize = size_of::<I16Vec>() / size_of::<i16>();

    #[target_feature(enable = "avx2")]
    pub fn zeroed() -> I16Vec {
        _mm256_setzero_si256()
    }

    #[target_feature(enable = "avx2")]
    pub fn splat(n: i16) -> I16Vec {
        _mm256_set1_epi16(n)
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn load(ptr: *const i16) -> I16Vec {
        unsafe { _mm256_loadu_si256(ptr.cast()) }
    }

    #[target_feature(enable = "avx2")]
    pub fn min(n: I16Vec, min: I16Vec) -> I16Vec {
        _mm256_min_epi16(n, min)
    }

    #[target_feature(enable = "avx2")]
    pub fn max(n: I16Vec, max: I16Vec) -> I16Vec {
        _mm256_max_epi16(n, max)
    }

    #[target_feature(enable = "avx2")]
    pub fn add_i32(a: I16Vec, b: I16Vec) -> I32Vec {
        _mm256_madd_epi16(a, b)
    }

    #[target_feature(enable = "avx2")]
    pub fn mul(a: I16Vec, b: I16Vec) -> I16Vec {
        _mm256_mullo_epi16(a, b)
    }
}

pub mod i32 {
    use super::*;

    #[target_feature(enable = "avx2")]
    pub fn zeroed() -> I32Vec {
        _mm256_setzero_si256()
    }

    #[target_feature(enable = "avx2")]
    pub fn add(a: I32Vec, b: I32Vec) -> I32Vec {
        _mm256_add_epi32(a, b)
    }

    #[target_feature(enable = "avx2")]
    pub fn reduce_sum(n: I32Vec) -> i32 {
        let sums = _mm_add_epi32(_mm256_castsi256_si128(n), _mm256_extracti128_si256(n, 1));
        let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0xee));
        let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0x55));
        _mm_cvtsi128_si32(sums)
    }
}
