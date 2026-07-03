use crate::{FEATURES, INPUT_BUCKETS, L1, L2, L3, Network, OUTPUT_BUCKETS};

#[repr(C, align(64))]
pub struct RawNetwork {
    pub l0_weights: [[i16; L1]; INPUT_BUCKETS * FEATURES],
    pub l0_biases: [i16; L1],
    pub l1_weights: [[[i8; L2]; OUTPUT_BUCKETS]; L1],
    pub l1_biases: [[i32; L2]; OUTPUT_BUCKETS],
    pub l2_weights: [[[i32; L3]; OUTPUT_BUCKETS]; L2],
    pub l2_biases: [[i32; L3]; OUTPUT_BUCKETS],
    pub l3_weights: [[i32; OUTPUT_BUCKETS]; L3],
    pub l3_biases: [i32; OUTPUT_BUCKETS],
}

pub fn preprocess(src: &RawNetwork, dst: &mut Network) {
    unsafe {
        std::ptr::copy_nonoverlapping(&raw const src.l0_weights, &raw mut dst.l0_weights, 1);
        std::ptr::copy_nonoverlapping(&raw const src.l0_biases, &raw mut dst.l0_biases, 1);

        for bucket in 0..OUTPUT_BUCKETS {
            for l1 in 0..L1 {
                let l1_block = l1 / 4;
                let k = l1 % 4;

                for l2 in 0..L2 {
                    dst.l1_weights[bucket][l1_block][l2 * 4 + k] = src.l1_weights[l1][bucket][l2];
                }
            }
        }

        std::ptr::copy_nonoverlapping(&raw const src.l1_biases, &raw mut dst.l1_biases, 1);

        for bucket in 0..OUTPUT_BUCKETS {
            for l2 in 0..L2 {
                for l3 in 0..L3 {
                    dst.l2_weights[bucket][l2][l3] = src.l2_weights[l2][bucket][l3];
                }
            }
        }

        std::ptr::copy_nonoverlapping(&raw const src.l2_biases, &raw mut dst.l2_biases, 1);

        for bucket in 0..OUTPUT_BUCKETS {
            for l3 in 0..L3 {
                dst.l3_weights[bucket][l3] = src.l3_weights[l3][bucket];
            }
        }

        std::ptr::copy_nonoverlapping(&raw const src.l3_biases, &raw mut dst.l3_biases, 1);
    }
}
