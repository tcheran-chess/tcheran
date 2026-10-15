#![allow(unused, reason = "Some functions may not be used on all arches")]
#![expect(
    unsafe_op_in_unsafe_fn,
    reason = "Architecture intrinsics require target-feature unsafe blocks"
)]

use std::{
    mem,
    simd::{Simd, SimdElement, cmp::SimdOrd, num::SimdInt},
};

use crate::engine::eval::nnue::network;

macro_rules! simd {
    (
        avx2 $avx2block:tt
        avx512 $avx512block:tt
        neon $neonblock:tt
    ) => {
        cfg_select! {
            target_feature = "avx512bw" => $avx512block,
            target_feature = "avx2" => $avx2block,
            target_feature = "neon" => $neonblock,
        }
    };
}

simd!(
    avx2 { use std::arch::x86_64::*; }
    avx512 { use std::arch::x86_64::*; }
    neon { use std::arch::aarch64::*; }
);

pub type ShiftType = simd!(
    avx2 { i32 }
    avx512 { u32 }
    neon { i32 }
);

pub type U8s = simd!(
    avx2 { Simd<u8, 32> }
    avx512 { Simd<u8, 64> }
    neon { Simd<u8, 16> }
);

pub type I8s = simd!(
    avx2 { Simd<i8, 32> }
    avx512 { Simd<i8, 64> }
    neon { Simd<i8, 16> }
);

pub type I16s = simd!(
    avx2 { Simd<i16, 16> }
    avx512 { Simd<i16, 32> }
    neon { Simd<i16, 8> }
);

pub type I32s = simd!(
    avx2 { Simd<i32, 8> }
    avx512 { Simd<i32, 16> }
    neon { Simd<i32, 4> }
);

pub type UI8s = I8s;

pub const I8_LANES: usize = I8s::LEN;
pub const I16_LANES: usize = I16s::LEN;
pub const I32_LANES: usize = I32s::LEN;

#[inline(always)]
pub unsafe fn load<T: SimdElement, const N: usize>(ptr: *const T) -> Simd<T, N> {
    let slice = unsafe { std::slice::from_raw_parts(ptr, N) };
    Simd::from_slice(slice)
}

#[inline(always)]
pub unsafe fn store<T: SimdElement, const N: usize>(ptr: *mut T, n: Simd<T, N>) {
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, N) };
    n.copy_to_slice(slice);
}

#[inline(always)]
pub unsafe fn shift_left_mul_high_i16s(a: I16s, b: I16s) -> I16s {
    const SHIFT: i32 = 16 - network::L0_SHIFT as i32;

    let a = Simd::<i32, I16_LANES>::from_array(a.to_array().map(i32::from));
    let b = Simd::<i32, I16_LANES>::from_array(b.to_array().map(i32::from));
    ((a << SHIFT) * b >> 16).cast::<i16>()
}

#[inline(always)]
pub unsafe fn packus(l: I16s, r: I16s) -> U8s {
    let l = l.simd_clamp(I16s::splat(0), I16s::splat(255)).cast::<u8>();
    let r = r.simd_clamp(I16s::splat(0), I16s::splat(255)).cast::<u8>();
    let l = l.to_array();
    let r = r.to_array();
    U8s::from_array(std::array::from_fn(|i| {
        if i < I16_LANES {
            l[i]
        } else {
            r[i - I16_LANES]
        }
    }))
}

#[inline(always)]
pub unsafe fn reinterpret_i32_as_u8s(n: *const i32) -> UI8s {
    let bytes = unsafe { (*n).to_ne_bytes() };
    I8s::from_array(std::array::from_fn(|i| bytes[i % bytes.len()] as i8))
}

#[inline(always)]
pub unsafe fn reinterpret_u8s_as_i32s(n: U8s) -> I32s {
    unsafe { mem::transmute(n) }
}

#[inline(always)]
#[allow(unused, reason = "Only used on some platforms")]
pub unsafe fn dpbusd(acc: I32s, u8s: UI8s, i8s: I8s) -> I32s {
    simd!(
        avx2 {{
            let acc: __m256i = unsafe { mem::transmute(acc) };
            let u8s: __m256i = unsafe { mem::transmute(u8s) };
            let i8s: __m256i = unsafe { mem::transmute(i8s) };
            let products = _mm256_maddubs_epi16(u8s, i8s);
            let ones = _mm256_set1_epi16(1);
            let summed = _mm256_madd_epi16(products, ones);
            mem::transmute(_mm256_add_epi32(acc, summed))
        }}
        avx512 {{
            let acc: __m512i = unsafe { mem::transmute(acc) };
            let u8s: __m512i = unsafe { mem::transmute(u8s) };
            let i8s: __m512i = unsafe { mem::transmute(i8s) };
            let result = cfg_select! {
                target_feature = "avx512vnni" => _mm512_dpbusd_epi32(acc, u8s, i8s),
                _ => {
                    let products = _mm512_maddubs_epi16(u8s, i8s);
                    let ones = _mm512_set1_epi16(1);
                    let summed = _mm512_madd_epi16(products, ones);
                    _mm512_add_epi32(acc, summed)
                }
            };
            mem::transmute(result)
        }}
        neon {{
            let acc: int32x4_t = unsafe { mem::transmute(acc) };
            let u8s: int8x16_t = unsafe { mem::transmute(u8s) };
            let i8s: int8x16_t = unsafe { mem::transmute(i8s) };
            let result = cfg_select! {
                target_feature = "dotprod" => vdotq_s32(acc, u8s, i8s),
                _ => {{
                    let lo = vmull_s8(vget_low_s8(u8s), vget_low_s8(i8s));
                    let hi = vmull_high_s8(u8s, i8s);
                    let pairwise = vpaddq_s16(lo, hi);
                    vpadalq_s16(acc, pairwise)
                }}
            };
            mem::transmute(result)
        }}
    )
}

#[inline(always)]
pub unsafe fn dpbusdx2(acc: I32s, u8s1: UI8s, i8s1: I8s, u8s2: UI8s, i8s2: I8s) -> I32s {
    simd!(
        avx2 {{
            let acc: __m256i = unsafe { mem::transmute(acc) };
            let u8s1: __m256i = unsafe { mem::transmute(u8s1) };
            let i8s1: __m256i = unsafe { mem::transmute(i8s1) };
            let u8s2: __m256i = unsafe { mem::transmute(u8s2) };
            let i8s2: __m256i = unsafe { mem::transmute(i8s2) };
            let p1 = _mm256_maddubs_epi16(u8s1, i8s1);
            let p2 = _mm256_maddubs_epi16(u8s2, i8s2);
            let combined = _mm256_adds_epi16(p1, p2);
            let ones = _mm256_set1_epi16(1);
            mem::transmute(_mm256_add_epi32(acc, _mm256_madd_epi16(combined, ones)))
        }}
        avx512 {{
            let acc: __m512i = unsafe { mem::transmute(acc) };
            let u8s1: __m512i = unsafe { mem::transmute(u8s1) };
            let i8s1: __m512i = unsafe { mem::transmute(i8s1) };
            let u8s2: __m512i = unsafe { mem::transmute(u8s2) };
            let i8s2: __m512i = unsafe { mem::transmute(i8s2) };
            let p1 = _mm512_maddubs_epi16(u8s1, i8s1);
            let p2 = _mm512_maddubs_epi16(u8s2, i8s2);
            let combined = _mm512_adds_epi16(p1, p2);
            let ones = _mm512_set1_epi16(1);
            mem::transmute(_mm512_add_epi32(acc, _mm512_madd_epi16(combined, ones)))
        }}
        neon {{
            let acc: int32x4_t = unsafe { mem::transmute(acc) };
            let u8s1: int8x16_t = unsafe { mem::transmute(u8s1) };
            let i8s1: int8x16_t = unsafe { mem::transmute(i8s1) };
            let u8s2: int8x16_t = unsafe { mem::transmute(u8s2) };
            let i8s2: int8x16_t = unsafe { mem::transmute(i8s2) };
            let result = cfg_select! {
                target_feature = "dotprod" => vdotq_s32(vdotq_s32(acc, u8s1, i8s1), u8s2, i8s2),
                _ => {{
                    let lo1 = vmull_s8(vget_low_s8(u8s1), i8s1);
                    let hi1 = vmull_high_s8(u8s1, i8s1);
                    let p1 = vpaddq_s16(lo1, hi1);
                    let lo2 = vmull_s8(vget_low_s8(u8s2), i8s2);
                    let hi2 = vmull_high_s8(u8s2, i8s2);
                    let p2 = vpaddq_s16(lo2, hi2);
                    vpadalq_s16(acc, vaddq_s16(p1, p2))
                }}
            };
            mem::transmute(result)
        }}
    )
}

macro_rules! nnz_table {
    () => {
        static NNZ_TABLE: [[i16; 8]; 256] = {
            let mut table = [[0i16; 8]; 256];

            let mut i = 0;
            while i < 256 {
                let mut j = i;
                let mut k = 0;
                while j != 0 {
                    table[i][k] = j.trailing_zeros() as i16;
                    j &= j - 1;
                    k += 1;
                }
                i += 1;
            }

            table
        };
    };
}

#[inline(always)]
pub unsafe fn nnz_indices(n: I32s) -> (I16s, u16) {
    simd!(
        avx2 {{
            nnz_table!();
            let n: __m256i = unsafe { mem::transmute(n) };
            let mask = _mm256_movemask_ps(_mm256_castsi256_ps(_mm256_cmpgt_epi32(n, _mm256_setzero_si256())));
            let idxs = unsafe { _mm_loadu_si128(NNZ_TABLE[mask as usize].as_ptr().cast()) };
            let idxs = _mm256_inserti128_si256::<0>(_mm256_setzero_si256(), idxs);
            (mem::transmute(idxs), mask.count_ones() as u16)
        }}
        avx512 {{
            let n: __m512i = unsafe { mem::transmute(n) };
            let mask = _mm512_test_epi32_mask(n, n);
            let idxs: [i16; 16] = std::array::from_fn(|i| i as i16);
            let idxs = unsafe { _mm256_loadu_si256(idxs.as_ptr().cast()) };
            let idxs = _mm512_castsi256_si512(_mm256_maskz_compress_epi16(mask, idxs));
            (mem::transmute(idxs), mask.count_ones() as u16)
        }}
        neon {{
            nnz_table!();
            let n: int32x4_t = unsafe { mem::transmute(n) };
            let mask = vtstq_s32(n, n);
            let bits = vaddvq_u32(vandq_u32(mask, unsafe { vld1q_u32([1, 2, 4, 8].as_ptr()) }));
            let idxs = unsafe { vld1q_s16(NNZ_TABLE[bits as usize].as_ptr()) };
            (mem::transmute(idxs), bits.count_ones() as u16)
        }}
    )
}
