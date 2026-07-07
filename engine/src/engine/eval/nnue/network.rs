use crate::chess::{File, Rank, Square};

// Network parameters
pub const FEATURES: usize = 768;
pub const HIDDEN_SIZE: usize = 1024;
pub const OUTPUT_BUCKETS: usize = 8;

// Quantization factors
pub const QA: i16 = 255;
pub const QB: i16 = 64;

// Eval scaling factor
pub const SCALE: i32 = 267;

/// Container for all network parameters
#[repr(C, align(64))]
pub struct Network {
    pub feature_weights: [[i16; HIDDEN_SIZE]; INPUT_BUCKETS * FEATURES],
    pub feature_bias: [i16; HIDDEN_SIZE],
    pub output_weights: [[i16; HIDDEN_SIZE * 2]; OUTPUT_BUCKETS],
    pub output_bias: [i16; OUTPUT_BUCKETS],
}

pub static NETWORK: Network = unsafe { std::mem::transmute(*include_bytes!(env!("NETWORK"))) };

#[rustfmt::skip]
const BUCKET_SCHEME: [usize; Square::N / 2] = [
    0, 1, 2, 3,
    4, 4, 5, 5,
    6, 6, 6, 6,
    6, 6, 6, 6,
    7, 7, 7, 7,
    7, 7, 7, 7,
    7, 7, 7, 7,
    7, 7, 7, 7,
];

pub const BUCKET_LAYOUT: [usize; Square::N] = const {
    let mut layout = [0; Square::N];
    let max_file_idx = File::N - 1;

    let mut rank = 0;
    while rank < Rank::N {
        let mut file = 0;
        while file < File::N / 2 {
            let bucket = BUCKET_SCHEME[rank * Rank::N / 2 + file];

            layout[rank * Rank::N + file] = bucket;
            layout[rank * Rank::N + (max_file_idx - file)] = bucket;

            file += 1;
        }

        rank += 1;
    }

    layout
};

pub const INPUT_BUCKETS: usize = const {
    let mut max = 0;
    let mut i = 0;

    while i < Square::N {
        if BUCKET_LAYOUT[i] > max {
            max = BUCKET_LAYOUT[i];
        }

        i += 1;
    }

    max + 1
};
