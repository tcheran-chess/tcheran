use crate::engine::tuning::{non_tunable_parameters, parameters};

#[rustfmt::skip]
parameters!(
    // Search
    aspiration_window_size: i32 = 16 [10..30];
    aspiration_min_depth: u8 = 5 [3..10];

    // Whole node techniques
    reverse_futility_prune_margin_per_ply: i32 = 106 [16..256];
    reverse_futility_prune_depth: u8 = 5 [2..10];

    razoring_margin: i32 = 429 [8..512];
    razoring_depth: u8 = 5 [2..10];

    null_move_pruning_base_reduction: u8 = 5 [2..10];
    null_move_pruning_reduction_factor: u8 = 4 [2..10];

    iir_depth: u8 = 3 [2..10];

    singular_extension_depth: u8 = 5 [2..10];
    singular_extension_entry_depth_delta: u8 = 3 [2..10];
    singular_extension_margin: i32 = 2 [2..10];
    double_extension_margin: i32 = 15 [2..32];
    double_extension_max: u8 = 5 [2..10];

    // Move loop techniques
    futility_prune_base_value: i32 = 500 [128..1024];
    futility_prune_depth_multiplier: i32 = 140 [8..256];
    futility_prune_depth: u8 = 8 [4..12];

    history_prune_depth: u8 = 4 [2..16];
    history_prune_margin: i32 = -2951 [-8192..-1024];

    see_quiet_margin: i32 = -58 [-256..-8];
    see_capture_margin: i32 = -229 [-256..-8];
    see_prune_history_divisor: i32 = 8 [8..128];
    see_prune_depth: u8 = 10 [2..16];

    lmp_depth: u8 = 6 [1..10];
    lmp_move_threshold: u8 = 6 [2..16];

    lmr_base: i32 = 20 [16..256];
    lmr_factor: i32 = 268 [64..1028];
    lmr_start_depth: u8 = 2 [1..8];
    lmr_move_threshold: u8 = 3 [2..16];

    lmr_cut_node_factor: u32 = 422 [128..2048];
    lmr_is_not_pv_factor: u32 = 1043 [128..2048];
    lmr_many_fail_highs_factor: u32 = 866 [128..2048];
    lmr_in_check_factor: u32 = 377 [128..2048];

    // Eval
    material_scale_base: i32 = 971 [512..1024];
    material_scale_divisor: i32 = 17 [16..128];

    // SEE
    see_pawn_value: i32 = 258 [4..4096];
    see_knight_value: i32 = 750 [4..4096];
    see_bishop_value: i32 = 761 [4..4096];
    see_rook_value: i32 = 1244 [4..4096];
    see_queen_value: i32 = 1986 [4..4096];

    // History
    quiet_history_max_bonus: i32 = 1770 [1..4096];
    quiet_history_factor: i32 = 661 [1..2048];
    quiet_history_offset: i32 = 339 [1..2048];

    capture_history_max_bonus: i32 = 2086 [1..4096];
    capture_history_factor: i32 = 717 [1..2048];
    capture_history_offset: i32 = 155 [1..2048];

    continuation_history_max_bonus: i32 = 1252 [1..4096];
    continuation_history_factor: i32 = 384 [1..2048];
    continuation_history_offset: i32 = 111 [1..2048];

    pawn_correction_history_weight: i32 = 200 [48..256];
    major_correction_history_weight: i32 = 114 [48..256];
    minor_correction_history_weight: i32 = 178 [48..256];
    non_pawn_correction_history_weight: i32 = 97 [48..256];
    threat_correction_history_weight: i32 = 141 [48..256];
);

#[rustfmt::skip]
non_tunable_parameters!(
    // Time Management
    max_time_per_move: f32 = 0.9;
    default_moves_to_go: u32 = 20;
    increment_to_use: f32 = 0.8;
    soft_time_multiplier: f32 = 0.70;
    hard_time_multiplier: f32 = 0.50;

    best_move_stability_initial_depth: u8 = 5;
    node_tm_base: f32 = 2.63;
    node_tm_multiplier: f32 = 1.7;
    node_tm_min: f32 = 0.9;
);
