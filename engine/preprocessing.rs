use std::{fs, path::PathBuf};

use crate::{
    ATTACKS_TABLE_SIZE, BISHOP_MAGICS, BISHOP_SHIFT, INPUT_BUCKETS, L1, L2, L3, Network, OUTPUT_BUCKETS, PSQT_FEATURES,
    ROOK_MAGICS, ROOK_SHIFT, THREAT_FEATURES,
};

#[repr(C, align(64))]
pub struct RawNetwork {
    pub l0_psqt_weights: [[[i16; L1]; PSQT_FEATURES]; INPUT_BUCKETS],
    pub l0_threat_weights: [[i8; L1]; THREAT_FEATURES],
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
    7518794, 7221533, 7002399, 6395892, 5966900, 5625083, 5121940, 5053419, 4650186, 4406726, 4135326, 3538722,
    3385471, 3305034, 3280784, 3254194, 3234797, 3200408, 3148206, 3080339, 3075412, 3033241, 2921611, 2901012,
    2860865, 2837318, 2810680, 2804960, 2792050, 2717181, 2691814, 2620868, 2462983, 2439302, 2420801, 2412456,
    2392178, 2362138, 2334933, 2321423, 2301493, 2256240, 2249402, 2203706, 2177417, 2170948, 2113415, 2097246,
    2084469, 2077138, 2040232, 2036549, 2030201, 2029242, 2016280, 2010991, 2007200, 1990866, 1950224, 1946432,
    1920056, 1912206, 1910093, 1905686, 1901259, 1899158, 1882099, 1876119, 1854561, 1837227, 1831121, 1824736,
    1812396, 1779587, 1768426, 1766012, 1763873, 1761171, 1748973, 1745413, 1744651, 1741096, 1683543, 1643418,
    1605069, 1562648, 1562291, 1557800, 1557300, 1538824, 1515028, 1495599, 1487131, 1473902, 1467270, 1463731,
    1461059, 1457201, 1455426, 1423799, 1420308, 1410158, 1410110, 1367594, 1367358, 1366788, 1365810, 1352460,
    1351530, 1332269, 1324500, 1323922, 1318000, 1315770, 1303803, 1294300, 1291380, 1274310, 1271435, 1270346,
    1264656, 1257518, 1252461, 1250482, 1248021, 1235630, 1227465, 1218125, 1212348, 1210775, 1208463, 1205066,
    1194150, 1179673, 1174167, 1173181, 1167021, 1164287, 1150918, 1150750, 1150713, 1108627, 1104305, 1102345,
    1090868, 1086499, 1081155, 1076222, 1071705, 1062926, 1061152, 1054069, 1049799, 1048501, 1047896, 1045245,
    1043800, 1041547, 1018129, 1015237, 1014695, 1010507, 1009556, 998573, 979935, 926411, 918167, 914030, 903198,
    902894, 895470, 883312, 865314, 865215, 861826, 861350, 859071, 858237, 855893, 842909, 825884, 819307, 811232,
    808032, 806476, 806005, 803540, 791612, 791510, 785874, 780581, 779654, 778536, 745299, 741916, 739307, 738997,
    719882, 704408, 704390, 701510, 679916, 643783, 630491, 623082, 617053, 612308, 606427, 605554, 597139, 590738,
    587099, 583580, 578990, 575163, 573419, 571638, 569123, 560553, 539709, 537626, 526516, 514275, 507572, 489710,
    478103, 468733, 461880, 460452, 455515, 442388, 429578, 425011, 420463, 394750, 379373, 374875, 367849, 359725,
    356411, 349353, 346122, 340244, 326959, 310967, 278935, 271992, 265756, 241952, 240772, 219368, 219266, 213532,
    204095, 199205, 190106,
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
