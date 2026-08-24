// Network parameters
pub const FEATURES: usize = 768;

pub const L1: usize = 1024;
pub const L2: usize = 16;
pub const L3: usize = 32;

pub const OUTPUT_BUCKETS: usize = 8;

// Quantisation factors
pub const Q0: i32 = 255;
pub const _Q1: i32 = 128;
pub const Q: i32 = 64;

pub const Q_BITS: u32 = 6;

pub const L0_SHIFT: u32 = 9;

pub const L1_SHIFT: u32 = 8;

// Eval scaling factor
pub const SCALE: i32 = 318;

// Redefinitions of Square::N / File::N / Rank::N so this file can be
// included directly in build.rs
const SQUARE_N: usize = 64;
const RANK_N: usize = 8;
const FILE_N: usize = 8;

/// Container for all network parameters
#[repr(C, align(64))]
pub struct Network {
    pub l0_weights: [[[i16; L1]; FEATURES]; INPUT_BUCKETS],
    pub l0_biases: [i16; L1],
    pub l1_weights: [[[i8; L2 * 4]; L1 / 4]; OUTPUT_BUCKETS],
    pub l1_biases: [[i32; L2]; OUTPUT_BUCKETS],
    pub l2_weights: [[[i32; L3]; L2 * 2]; OUTPUT_BUCKETS],
    pub l2_biases: [[i32; L3]; OUTPUT_BUCKETS],
    pub l3_weights: [[i32; L3]; OUTPUT_BUCKETS],
    pub l3_biases: [i32; OUTPUT_BUCKETS],
}

#[rustfmt::skip]
const BUCKET_SCHEME: [usize; SQUARE_N / 2] = [
    0, 1, 2, 3,
    4, 4, 5, 5,
    6, 6, 6, 6,
    6, 6, 6, 6,
    7, 7, 7, 7,
    7, 7, 7, 7,
    7, 7, 7, 7,
    7, 7, 7, 7,
];

pub const BUCKET_LAYOUT: [usize; SQUARE_N] = const {
    let mut layout = [0; SQUARE_N];
    let max_file_idx = FILE_N - 1;

    let mut rank = 0;
    while rank < RANK_N {
        let mut file = 0;
        while file < FILE_N / 2 {
            let bucket = BUCKET_SCHEME[rank * RANK_N / 2 + file];

            layout[rank * RANK_N + file] = bucket;
            layout[rank * RANK_N + (max_file_idx - file)] = bucket;

            file += 1;
        }

        rank += 1;
    }

    layout
};

pub const INPUT_BUCKETS: usize = const {
    let mut max = 0;
    let mut i = 0;

    while i < SQUARE_N {
        if BUCKET_LAYOUT[i] > max {
            max = BUCKET_LAYOUT[i];
        }

        i += 1;
    }

    max + 1
};
