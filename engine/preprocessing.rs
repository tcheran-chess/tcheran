use std::{fs, path::PathBuf};

use crate::{Network, ATTACKS_TABLE_SIZE, BISHOP_MAGICS, BISHOP_SHIFT, INPUT_BUCKETS, L1, L2, L3, OUTPUT_BUCKETS, PSQT_FEATURES, ROOK_MAGICS, ROOK_SHIFT, THREAT_INPUTS};

#[repr(C, align(64))]
pub struct RawNetwork {
    pub l0_psqt_weights: [[[i16; L1]; PSQT_FEATURES]; INPUT_BUCKETS],
    pub l0_threat_weights: [[i8; L1]; THREAT_INPUTS],
    pub l0_biases: [i16; L1],
    pub l1_weights: [[[i8; L2]; OUTPUT_BUCKETS]; L1],
    pub l1_biases: [[i32; L2]; OUTPUT_BUCKETS],
    pub l2_weights: [[[i32; L3]; OUTPUT_BUCKETS]; L2 * 2],
    pub l2_biases: [[i32; L3]; OUTPUT_BUCKETS],
    pub l3_weights: [[i32; OUTPUT_BUCKETS]; L3],
    pub l3_biases: [i32; OUTPUT_BUCKETS],
}

// Set this to true for generating permutations for new nets
const DISABLE_SPARSITY_PERMUTATION: bool = false;

const L0_ACTIVATIONS: [usize; L1 / 2] = [
    2077138, 679916, 1831121, 1108627, 806005, 2301493, 4406726, 2901012, 590738, 1270346, 1761171, 1420308, 569123,
    1041547, 791510, 2420801, 780581, 199205, 571638, 1768426, 1779587, 605554, 1174167, 1015237, 778536, 1766012,
    1150713, 1323922, 1061152, 2921611, 3234797, 741916, 825884, 791612, 1748973, 1557300, 219266, 1562291, 2203706,
    1047896, 1837227, 367849, 1102345, 507572, 5053419, 3075412, 1274310, 861350, 865314, 1741096, 265756, 3305034,
    1351530, 2804960, 4650186, 1179673, 374875, 1318000, 1557800, 1227465, 1538824, 1854561, 1257518, 883312, 2249402,
    455515, 2010991, 356411, 3080339, 514275, 2016280, 2097246, 1367594, 803540, 271992, 1164287, 1457201, 1366788,
    979935, 539709, 1167021, 1303803, 7518794, 914030, 855893, 2412456, 460452, 1264656, 1515028, 1352460, 2084469,
    1912206, 745299, 1455426, 1744651, 858237, 1643418, 2321423, 1461059, 2113415, 2170948, 1562648, 1901259, 349353,
    1173181, 2620868, 7002399, 1410158, 1899158, 1009556, 1294300, 526516, 2362138, 2462983, 1332269, 1487131, 204095,
    3385471, 2256240, 278935, 1315770, 617053, 1876119, 630491, 1062926, 560553, 1495599, 241952, 5121940, 1018129,
    1365810, 2040232, 537626, 190106, 704408, 1882099, 2392178, 3254194, 2007200, 2439302, 811232, 1812396, 340244,
    606427, 1763873, 575163, 1086499, 3280784, 578990, 902894, 918167, 240772, 1045245, 779654, 1950224, 1683543,
    842909, 998573, 719882, 903198, 926411, 1218125, 1210775, 1250482, 3148206, 5625083, 738997, 1291380, 1081155,
    1150750, 2177417, 1043800, 1104305, 1990866, 461880, 6395892, 1071705, 1473902, 861826, 1208463, 3033241, 1605069,
    478103, 379373, 597139, 587099, 442388, 1367358, 429578, 2029242, 2036549, 4135326, 2860865, 612308, 3200408,
    7221533, 2334933, 573419, 623082, 859071, 489710, 1824736, 643783, 2030201, 2691814, 2792050, 2837318, 2810680,
    2717181, 1324500, 1150918, 806476, 1048501, 1076222, 1049799, 1463731, 1271435, 1252461, 468733, 1235630, 808032,
    704390, 1910093, 1467270, 1205066, 420463, 1014695, 1054069, 219368, 394750, 865215, 5966900, 213532, 895470,
    1010507, 1090868, 1410110, 1194150, 310967, 1905686, 346122, 3538722, 583580, 1920056, 326959, 425011, 1946432,
    1423799, 819307, 739307, 359725, 1745413, 1212348, 701510, 1248021, 785874,
];

pub fn preprocess(src: &RawNetwork, dst: &mut Network) {
    let sparsity_permutation_indices = {
        let mut indices: [usize; L1 / 2] = std::array::from_fn(|i| i);

        if !DISABLE_SPARSITY_PERMUTATION {
            indices.sort_by(|&a, &b| L0_ACTIVATIONS[b].cmp(&L0_ACTIVATIONS[a]));
        }

        indices
    };

    unsafe {
        for (dst_bucket, src_bucket) in dst.l0_psqt_weights.iter_mut().zip(src.l0_psqt_weights.iter()) {
            for (src_row, dst_row) in src_bucket.iter().zip(dst_bucket.iter_mut()) {
                for (dst_idx, &src_idx) in sparsity_permutation_indices.iter().enumerate() {
                    dst_row[dst_idx] = src_row[src_idx];
                    dst_row[dst_idx + L1 / 2] = src_row[src_idx + L1 / 2];
                }
            }
        }

        for (dst_row, src_row) in dst.l0_threat_weights.iter_mut().zip(src.l0_threat_weights.iter()) {
            for (dst_idx, &src_idx) in sparsity_permutation_indices.iter().enumerate() {
                dst_row[dst_idx] = src_row[src_idx];
                dst_row[dst_idx + L1 / 2] = src_row[src_idx + L1 / 2];
            }
        }

        for (dst_idx, &src_idx) in sparsity_permutation_indices.iter().enumerate() {
            dst.l0_biases[dst_idx] = src.l0_biases[src_idx];
            dst.l0_biases[dst_idx + L1 / 2] = src.l0_biases[src_idx + L1 / 2];
        }

        for bucket in 0..OUTPUT_BUCKETS {
            for (dst_input_idx, &src_input_idx) in sparsity_permutation_indices.iter().enumerate() {
                for half in 0..2 {
                    let dst_idx = dst_input_idx + half * (L1 / 2);
                    let src_idx = src_input_idx + half * (L1 / 2);

                    let in_block = dst_idx / 4;
                    let k = dst_idx % 4;

                    for output_idx in 0..L2 {
                        dst.l1_weights[bucket][in_block][output_idx * 4 + k] =
                            src.l1_weights[src_idx][bucket][output_idx];
                    }
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

pub fn init_magics() {
    let rook_offsets = &[(1, 0), (0, 1), (-1, 0), (0, -1)];
    let bishop_offsets = &[(1, 1), (-1, 1), (-1, -1), (1, -1)];

    let rook_not_masks = generate_not_masks(rook_offsets);
    let bishop_not_masks = generate_not_masks(bishop_offsets);

    let mut table = vec![0_u64; ATTACKS_TABLE_SIZE];
    fill_attacks(&mut table, &ROOK_MAGICS, &rook_not_masks, rook_offsets, ROOK_SHIFT);
    fill_attacks(&mut table, &BISHOP_MAGICS, &bishop_not_masks, bishop_offsets, BISHOP_SHIFT);

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    fs::write(out_dir.join("magics.bin"), to_raw_bytes(&table)).unwrap();
    fs::write(out_dir.join("rook_not_masks.bin"), to_raw_bytes(&rook_not_masks)).unwrap();
    fs::write(out_dir.join("bishop_not_masks.bin"), to_raw_bytes(&bishop_not_masks)).unwrap();
    println!("cargo:rerun-if-changed=src/chess/moves/tables/magic_consts.rs");
}

fn generate_not_masks(directions: &[(i8, i8)]) -> [u64; 64] {
    std::array::from_fn(|square| {
        let rank = (square / 8) as i8;
        let file = (square % 8) as i8;

        let mut end_mask = 0_u64;

        if file != 0 {
            end_mask |= file_mask(0);
        }
        if file != 7 {
            end_mask |= file_mask(7);
        }
        if rank != 0 {
            end_mask |= rank_mask(0);
        }
        if rank != 7 {
            end_mask |= rank_mask(7);
        }

        !(sliding_attacks(square, 0, directions) & !end_mask)
    })
}

fn fill_attacks(
    table: &mut [u64],
    magics: &[(u64, usize)],
    not_masks: &[u64; 64],
    directions: &[(i8, i8)],
    shift: usize,
) {
    for (square, &(magic, base_index)) in magics.iter().enumerate() {
        let not_mask = not_masks[square];
        let occupancy_mask = !not_mask;
        let mut blockers = 0_u64;

        loop {
            blockers = blockers.wrapping_sub(occupancy_mask) & occupancy_mask;
            let subset_index = (blockers | not_mask).wrapping_mul(magic) >> (64 - shift);
            let table_index = base_index + subset_index as usize;
            table[table_index] = sliding_attacks(square, blockers, directions);

            if blockers == 0 {
                break;
            }
        }
    }
}

fn sliding_attacks(square: usize, blockers: u64, directions: &[(i8, i8)]) -> u64 {
    let rank = (square / 8) as i8;
    let file = (square % 8) as i8;
    let mut attacks = 0_u64;

    for &(rank_step, file_step) in directions {
        let (mut next_rank, mut next_file) = (rank, file);
        loop {
            next_rank += rank_step;
            next_file += file_step;
            if !inside_board(next_rank, next_file) {
                break;
            }

            let next_square = square_bb(next_rank, next_file);
            attacks |= next_square;
            if blockers & next_square != 0 {
                break;
            }
        }
    }

    attacks
}

const fn inside_board(rank: i8, file: i8) -> bool {
    rank >= 0 && rank < 8 && file >= 0 && file < 8
}

const fn square_bb(rank: i8, file: i8) -> u64 {
    1_u64 << (rank as usize * 8 + file as usize)
}

const fn file_mask(file: usize) -> u64 {
    0x0101_0101_0101_0101_u64 << file
}

const fn rank_mask(rank: usize) -> u64 {
    0xFF_u64 << (rank * 8)
}

fn to_raw_bytes(values: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(size_of_val(values));
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}
