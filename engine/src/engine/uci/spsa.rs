use crate::{
    chess::piece::PieceKind,
    engine::{
        search::Params,
        uci::options::{UciOption, UciOptionType},
    },
};

#[allow(clippy::allow_attributes, reason = "May be unused in non-SPSA builds")]
#[allow(unused, reason = "May be unused in non-SPSA builds")]
pub fn spsa_options() -> Vec<UciOption> {
    let p = Params::default();

    vec![
        UciOption::spin("AspirationMinDepth", |options, _state, value| {
            options.params.aspiration_min_depth = value.as_depth();
        })
        .default(p.aspiration_min_depth)
        .with_bounds(1, 10)
        .disable()
        .build(),
        //
        UciOption::spin("AspirationWindowSize", |options, _state, value| {
            options.params.aspiration_window_size = value.as_eval();
        })
        .default(p.aspiration_window_size)
        .with_bounds(20, 100)
        .disable()
        .build(),
        //
        UciOption::spin("NullMovePruningBaseReduction", |options, _state, value| {
            options.params.null_move_pruning_base_reduction = value.as_depth();
        })
        .default(p.null_move_pruning_base_reduction)
        .with_bounds(1, 10)
        .with_spsa_step(1.0)
        .build(),
        //
        UciOption::spin("NullMovePruningReductionFactor", |options, _state, value| {
            options.params.null_move_pruning_reduction_factor = value.as_depth();
        })
        .default(p.null_move_pruning_reduction_factor)
        .with_bounds(1, 10)
        .with_spsa_step(1.0)
        .build(),
        //
        UciOption::spin("FutilityPruningDepth", |options, _state, value| {
            options.params.futility_prune_depth = value.as_depth();
        })
        .default(p.futility_prune_depth)
        .with_bounds(1, 10)
        .disable()
        .build(),
        //
        UciOption::spin("FutilityPruneMaxMoveValue", |options, _state, value| {
            options.params.futility_prune_max_move_value = value.as_eval();
        })
        .default(p.futility_prune_max_move_value)
        .with_bounds(1, 1024)
        .with_spsa_step(10.0)
        .build(),
        //
        UciOption::spin("SeePawnValue", |options, _state, value| {
            options.params.see_values[PieceKind::Pawn] = value.as_eval();
        })
        .default(p.see_values[PieceKind::Pawn])
        .with_bounds(1, 2048)
        .with_spsa_step(16.0)
        .build(),
        //
        UciOption::spin("SeeKnightValue", |options, _state, value| {
            options.params.see_values[PieceKind::Knight] = value.as_eval();
        })
        .default(p.see_values[PieceKind::Knight])
        .with_bounds(1, 2048)
        .with_spsa_step(16.0)
        .build(),
        //
        UciOption::spin("SeeBishopValue", |options, _state, value| {
            options.params.see_values[PieceKind::Bishop] = value.as_eval();
        })
        .default(p.see_values[PieceKind::Bishop])
        .with_bounds(1, 2048)
        .with_spsa_step(16.0)
        .build(),
        //
        UciOption::spin("SeeRookValue", |options, _state, value| {
            options.params.see_values[PieceKind::Rook] = value.as_eval();
        })
        .default(p.see_values[PieceKind::Rook])
        .with_bounds(1, 2048)
        .with_spsa_step(16.0)
        .build(),
        //
        UciOption::spin("SeeQueenValue", |options, _state, value| {
            options.params.see_values[PieceKind::Queen] = value.as_eval();
        })
        .default(p.see_values[PieceKind::Queen])
        .with_bounds(1, 2048)
        .with_spsa_step(16.0)
        .build(),
        //
        UciOption::spin("SeePruneDepth", |options, _state, value| {
            options.params.see_prune_depth = value.as_depth();
        })
        .default(p.see_prune_depth)
        .with_bounds(1, 10)
        .disable()
        .build(),
        //
        UciOption::spin("SeeQuietMargin", |options, _state, value| {
            options.params.see_quiet_margin = value.as_eval();
        })
        .default(p.see_quiet_margin)
        .with_bounds(-512, -1)
        .with_spsa_step(5.0)
        .build(),
        //
        UciOption::spin("SeeCaptureMargin", |options, _state, value| {
            options.params.see_capture_margin = value.as_eval();
        })
        .default(p.see_capture_margin)
        .with_bounds(-512, -1)
        .with_spsa_step(5.0)
        .build(),
        //
        UciOption::spin("GoodTacticalSeeBound", |options, _state, value| {
            options.params.good_tactical_see_bound = value.as_eval();
        })
        .default(p.good_tactical_see_bound)
        .with_bounds(-1024, 1024)
        .with_spsa_step(50.0)
        .build(),
        //
        UciOption::spin("QsGoodTacticalSeeBound", |options, _state, value| {
            options.params.qs_good_tactical_see_bound = value.as_eval();
        })
        .default(p.qs_good_tactical_see_bound)
        .with_bounds(-1024, 1024)
        .with_spsa_step(50.0)
        .build(),
        //
        UciOption::spin("ReverseFutilityPruneDepth", |options, _state, value| {
            options.params.reverse_futility_prune_depth = value.as_depth();
        })
        .default(p.reverse_futility_prune_depth)
        .with_bounds(1, 10)
        .disable()
        .build(),
        //
        UciOption::spin("ReverseFutilityPruneMarginPerPly", |options, _state, value| {
            options.params.reverse_futility_prune_margin_per_ply = value.as_eval();
        })
        .default(p.reverse_futility_prune_margin_per_ply)
        .with_bounds(1, 256)
        .with_spsa_step(10.0)
        .build(),
        //
        UciOption::spin("LmrDepth", |options, _state, value| {
            options.params.lmr_depth = value.as_depth();
        })
        .default(p.lmr_depth)
        .with_bounds(1, 10)
        .disable()
        .build(),
        //
        UciOption::spin("LmrMoveThreshold", |options, _state, value| {
            options.params.lmr_move_threshold = value.as_usize();
        })
        .default(p.lmr_move_threshold)
        .with_bounds(1, 10)
        .with_spsa_step(1.0)
        .build(),
        //
        UciOption::spin("IIR_Depth", |options, _state, value| {
            options.params.iir_depth = value.as_depth();
        })
        .default(p.iir_depth)
        .with_bounds(1, 10)
        .disable()
        .build(),
        //
        UciOption::spin("SingularExtensionDepth", |options, _state, value| {
            options.params.singular_extension_depth = value.as_depth();
        })
        .default(p.singular_extension_depth)
        .with_bounds(1, 10)
        .disable()
        .build(),
        //
        UciOption::spin("SingularExtensionEntryDepthDelta", |options, _state, value| {
            options.params.singular_extension_entry_depth_delta = value.as_depth();
        })
        .default(p.singular_extension_entry_depth_delta)
        .with_bounds(1, 10)
        .with_spsa_step(1.0)
        .build(),
        //
        UciOption::spin("SingularExtensionMargin", |options, _state, value| {
            options.params.singular_extension_margin = value.as_eval();
        })
        .default(p.singular_extension_margin)
        .with_bounds(1, 512)
        .with_spsa_step(1.0)
        .build(),
        //
        UciOption::spin("DoubleExtensionMargin", |options, _state, value| {
            options.params.double_extension_margin = value.as_eval();
        })
        .default(p.double_extension_margin)
        .with_bounds(1, 512)
        .with_spsa_step(1.0)
        .build(),
        //
        UciOption::spin("DoubleExtensionMax", |options, _state, value| {
            options.params.double_extension_max = value.as_usize();
        })
        .default(p.double_extension_max)
        .with_bounds(1, 20)
        .with_spsa_step(1.0)
        .build(),
        //
    ]
}

pub fn print_spsa_input() {
    let options = spsa_options();

    for UciOption { name, t } in options {
        match t {
            UciOptionType::Spin {
                default,
                min,
                max,
                set_fn: _,

                spsa_step,
                spsa_disabled,
            } => {
                if spsa_disabled {
                    continue;
                }

                let spsa_step = spsa_step.unwrap_or_else(|| panic!("No SPSA step set for {name}"));

                println!("{name}, int, {default}, {min}, {max}, {spsa_step:.1}, 0.002");
            }
            _ => panic!("Invalid SPSA option: {name}"),
        }
    }
}
