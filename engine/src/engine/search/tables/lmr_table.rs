use crate::engine::{params::*, search::types::Depth};

static mut LMR_TABLE: [[u8; 64]; 64] = [[0; 64]; 64];

pub fn lmr_reduction(depth: Depth, move_count: usize) -> u8 {
    unsafe { LMR_TABLE[depth.idx().min(63)][move_count.min(63)] }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "Calculation is intentionally approximate"
)]
pub fn init() {
    unsafe {
        let base = lmr_base() as f32 / 100.0;
        let factor = lmr_factor() as f32 / 100.0;

        for (depth, table) in LMR_TABLE.iter_mut().enumerate().skip(1) {
            for (move_count, reduction) in table.iter_mut().enumerate().skip(1) {
                *reduction =
                    (base + f32::ln(depth as f32) * f32::ln(move_count as f32) / factor) as u8;
            }
        }
    }
}
