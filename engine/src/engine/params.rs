use crate::engine::tuning::{non_tunable_parameters, parameters};

#[rustfmt::skip]
parameters!(
    (history_max_bonus, i32, 1600, 1, 4096, 256.0),
    (history_factor, i32, 350, 1, 2048, 32.0),
    (history_offset, i32, 350, 1, 2048, 32.0),

    (futility_prune_max_move_value, i32, 135, 8, 256, 10.0),

    (see_pawn_value, i32, 100, 4, 4096, 16.0),
    (see_knight_value, i32, 300, 4, 4096, 16.0),
    (see_bishop_value, i32, 300, 4, 4096, 16.0),
    (see_rook_value, i32, 500, 4, 4096, 16.0),
    (see_queen_value, i32, 900, 4, 4096, 16.0),

    (see_quiet_margin, i32, -30, -256, -8, 5.0),
    (see_capture_margin, i32, -100, -256, -8, 5.0),

    (see_prune_history_divisor, i32, 32, 8, 128, 6.0),

    (reverse_futility_prune_margin_per_ply, i32, 150, 16, 256, 10.0),

    (double_extension_margin, i32, 17, 1, 128, 1.0),

    (pawn_correction_history_weight, i32, 128, 48, 256, 8.0),
    (major_correction_history_weight, i32, 128, 48, 256, 8.0),
    (minor_correction_history_weight, i32, 128, 48, 256, 8.0),
    (non_pawn_correction_history_weight, i32, 128, 48, 256, 8.0),
    (threat_correction_history_weight, i32, 128, 48, 256, 8.0),

    (material_scale_base, i32, 950, 512, 1024, 20.0),
    (material_scale_divisor, i32, 32, 16, 128, 2.0),
);

#[rustfmt::skip]
non_tunable_parameters!(
    (aspiration_window_size, i32, 15),
    (aspiration_min_depth, u8, 5),

    (null_move_pruning_base_reduction, u8, 4),
    (null_move_pruning_reduction_factor, u8, 4),

    (futility_prune_depth, u8, 1),

    (see_prune_depth, u8, 10),

    (reverse_futility_prune_depth, u8, 4),

    (lmr_base, f32, 0.75),
    (lmr_factor, f32, 2.25),
    (lmr_depth, u8, 3),
    (lmr_move_threshold, usize, 3),

    (lmp_depth, u8, 2),
    (lmp_move_threshold, u8, 5),

    (iir_depth, u8, 4),

    (singular_extension_depth, u8, 5),
    (singular_extension_entry_depth_delta, u8, 3),
    (singular_extension_margin, i32, 2),

    (double_extension_max, u8, 4),

    (max_time_per_move, f32, 0.9),
    (default_moves_to_go, u32, 20),
    (increment_to_use, f32, 0.8),
    (soft_time_multiplier, f32, 0.70),
    (hard_time_multiplier, f32, 0.50),

    (best_move_stability_initial_depth, u8, 5),
    (node_tm_base, f32, 2.63),
    (node_tm_multiplier, f32, 1.7),
    (node_tm_min, f32, 0.9),
);
