#![allow(non_upper_case_globals, reason = "Looks nicer")]

use crate::engine::tuning::parameters;

#[rustfmt::skip]
parameters!(
    // Search
    aspiration_window_size: i32 = 16 [10..30];
    aspiration_min_depth: u8 = 5 [3..10];
    aspiration_max_reduction: u8 = 3 [2..10];

    // Whole node techniques
    reverse_futility_prune_depth_margin: i32 = 111 [16..256];
    reverse_futility_prune_improving_margin: i32 = 95 [16..256];
    reverse_futility_prune_depth: u8 = 5 [2..10];

    razoring_margin: i32 = 443 [8..512];
    razoring_depth: u8 = 5 [2..10];

    null_move_pruning_base_reduction: u8 = 5 [2..10];
    null_move_pruning_reduction_factor: u8 = 5 [2..10];

    iir_depth: u8 = 3 [2..10];

    singular_extension_depth: u8 = 5 [2..10];
    singular_extension_entry_depth_delta: u8 = 3 [2..10];
    singular_extension_margin: i32 = 2 [2..10];
    double_extension_margin: i32 = 18 [2..32];
    double_extension_max: u8 = 5 [2..10];

    // Move loop techniques
    futility_prune_base_value: i32 = 496 [128..1024];
    futility_prune_depth_multiplier: i32 = 148 [8..256];
    futility_prune_depth: u8 = 8 [4..12];

    history_prune_depth: u8 = 4 [2..16];
    history_prune_offset: i32 = 86 [-8192..512];
    history_prune_margin: i32 = -2787 [-8192..-1024];

    see_quiet_margin: i32 = -55 [-256..-8];
    see_capture_margin: i32 = -240 [-256..-8];
    see_prune_history_divisor: i32 = 12 [8..128];
    see_prune_depth: u8 = 10 [2..16];

    lmp_depth: u8 = 6 [1..10];
    lmp_move_threshold: u8 = 5 [2..16];

    lmr_base: i32 = 24 [16..256];
    lmr_factor: i32 = 225 [64..1028];
    lmr_start_depth: u8 = 2 [1..8];
    lmr_move_threshold: u8 = 2 [2..16];

    lmr_cut_node_factor: u32 = 400 [128..2048];
    lmr_is_not_pv_factor: u32 = 940 [128..2048];
    lmr_many_fail_highs_factor: u32 = 760 [128..2048];
    lmr_in_check_factor: u32 = 320 [128..2048];
    lmr_ttpv_factor: u32 = 1148 [128..2048];

    // Quiescence
    quiescence_futility_margin: i32 = 400 [64..1024];

    // Eval
    material_scale_base: i32 = 986 [512..1024];
    material_scale_divisor: i32 = 24 [16..128];

    // SEE
    see_pawn_value: i32 = 255 [4..4096];
    see_knight_value: i32 = 676 [4..4096];
    see_bishop_value: i32 = 792 [4..4096];
    see_rook_value: i32 = 1125 [4..4096];
    see_queen_value: i32 = 1905 [4..4096];

    // History
    quiet_history_max_bonus: i32 = 2054 [1..4096];
    quiet_history_factor: i32 = 637 [1..2048];
    quiet_history_offset: i32 = 307 [1..2048];

    capture_history_max_bonus: i32 = 2381 [1..4096];
    capture_history_factor: i32 = 681 [1..2048];
    capture_history_offset: i32 = 246 [1..2048];

    continuation_history_max_bonus: i32 = 1353 [1..4096];
    continuation_history_factor: i32 = 413 [1..2048];
    continuation_history_offset: i32 = 84 [1..2048];

    pawn_correction_history_weight: i32 = 196 [48..256];
    major_correction_history_weight: i32 = 101 [48..256];
    minor_correction_history_weight: i32 = 168 [48..256];
    non_pawn_correction_history_weight: i32 = 88 [48..256];
    threat_correction_history_weight: i32 = 149 [48..256];

    thread_vote_offset: i32 = 25 [2..32];

    // Time Management
    max_time_per_move: u32 = 90 [50..100];
    default_moves_to_go: u32 = 20 [8..32];
    increment_to_use: u32 = 80 [1..100];
    soft_time_multiplier: u32 = 70 [1..100];
    hard_time_multiplier: u32 = 50 [1..100];

    best_move_stability_initial_depth: u8 = 5 [4..8];

    node_tm_base: u32 = 263 [1..512];
    node_tm_multiplier: u32 = 170 [1..512];
    node_tm_min: u32 = 90 [1..100];
);
