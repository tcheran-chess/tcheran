use crate::{FEATURES, INPUT_BUCKETS, L1, L2, L3, Network, OUTPUT_BUCKETS};

#[repr(C, align(64))]
pub struct RawNetwork {
    pub l0_weights: [[i16; L1]; INPUT_BUCKETS * FEATURES],
    pub l0_biases: [i16; L1],
    pub l1_weights: [[[i8; L2]; OUTPUT_BUCKETS]; L1],
    pub l1_biases: [[i32; L2]; OUTPUT_BUCKETS],
    pub l2_weights: [[[i32; L3]; OUTPUT_BUCKETS]; L2 * 2],
    pub l2_biases: [[i32; L3]; OUTPUT_BUCKETS],
    pub l3_weights: [[i32; OUTPUT_BUCKETS]; L3],
    pub l3_biases: [i32; OUTPUT_BUCKETS],
}

pub fn preprocess(src: &RawNetwork, dst: &mut Network) {
    unsafe {
        std::ptr::copy_nonoverlapping(&raw const src.l0_weights, &raw mut dst.l0_weights, 1);
        std::ptr::copy_nonoverlapping(&raw const src.l0_biases, &raw mut dst.l0_biases, 1);

        // L0 inference uses packus which permutes groups of values. We can cancel this out by applying
        // a reverse permutation to the values being used.
        permute_for_packus(dst);

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
            for l2 in 0..L2 * 2 {
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

fn permute_for_packus(network: &mut Network) {
    let packus_permute_order: &[u8] = cfg_select! {
        target_feature = "avx512bw" => &[0, 2, 4, 6, 1, 3, 5, 7],
        target_feature = "avx2" => &[0, 2, 1, 3],
        _ => &[]
    };

    if packus_permute_order.is_empty() {
        return;
    }

    let num_chunks = packus_permute_order.len();
    let chunk_size = 8;
    let block_size = num_chunks * chunk_size;

    for row in &mut network.l0_weights {
        permute_i16s(row, packus_permute_order, chunk_size, block_size);
    }

    // Permute L0 biases.
    permute_i16s(&mut network.l0_biases, packus_permute_order, chunk_size, block_size);
}

// Permute a flat slice of i16 values in-place
fn permute_i16s(data: &mut [i16], order: &[u8], chunk_size: usize, block_size: usize) {
    let num_chunks = order.len();
    let mut temp = vec![0i16; block_size];
    for block_start in (0..data.len()).step_by(block_size) {
        temp.copy_from_slice(&data[block_start..block_start + block_size]);
        for dst_chunk in 0..num_chunks {
            let src_chunk = order[dst_chunk] as usize;
            let dst_offset = block_start + dst_chunk * chunk_size;
            let src_offset = src_chunk * chunk_size;
            data[dst_offset..dst_offset + chunk_size]
                .copy_from_slice(&temp[src_offset..src_offset + chunk_size]);
        }
    }
}
