pub mod move_picker;
mod principal_variation;
pub mod tables;
pub mod time_control;
pub mod types;

pub fn init() {
    tables::init();
}

use std::time::Instant;

pub use types::*;

use crate::{
    chess::{moves::MoveList, prelude::*},
    engine::{
        eval::{Eval, eval},
        options::EngineOptions,
        params::*,
        search::{
            move_picker::{GenStage, MovePicker},
            principal_variation::PrincipalVariation,
            tables::lmr_reduction,
            time_control::StopControl,
        },
        see::see,
        tablebases::{Tablebase, Wdl},
        transposition_table::NodeBound,
    },
};

pub const MAX_SEARCH_DEPTH: u8 = u8::MAX - 1;

// Size used for arrays that are indexed by ply. Add one extra above MAX_PLIES_SIZE for safety
// when we perform operations on 'the next ply'.
pub const MAX_PLIES_ARRAY_SIZE: usize = MAX_SEARCH_DEPTH as usize + 1;

pub fn search(
    game: &Game,
    persistent_state: &PersistentState,
    thread_data: &mut ThreadData,
    results: &SearchResults,
    time_control: TimeControl,
    stop_control: &StopControl,
    options: &EngineOptions,
    reporter: &dyn Reporter,
) -> SearchResult {
    let thread_id = thread_data.id;
    let is_main_thread = thread_id == 0;

    let (tables, stack, nnue) = thread_data.mut_refs();
    let mut ctx = SearchContext::new(
        thread_id,
        game,
        &persistent_state.tt,
        &persistent_state.tablebase,
        &persistent_state.node_counter,
        &persistent_state.tbhits_counter,
        tables,
        stack,
        nnue,
        time_control,
        stop_control.clone(),
        options,
    );

    let thread_result = iterative_deepening(
        // Give the search its own copy of the game so we don't get one returned in a dirty state
        // when the search aborts.
        &mut game.clone(),
        &mut ctx,
        reporter,
    );

    results.set(thread_id, &thread_result);

    if is_main_thread {
        // If we're the main thread, signal for all the other threads to stop and then wait until
        // they do.
        stop_control.stop();
        stop_control.wait_until_last();

        let mut result = best_result(results);

        let send_final_info =
             // We picked a different thread, so we want to report *that* thread's info
             result.id != thread_id
             // We did more searching since last reporting, always send a final info line
             // before reporting the best move so we have useful information such as the exact number of
             // nodes searched and the exact time used. This could be useful for debugging time issues or
             // reproducing a bug by playing exact nodes.
             // See https://github.com/AndyGrant/Ethereal/issues/214
             || ctx.was_hard_stopped
             // This will be the only info we send
             || ctx.options.minimal;

        stop_control.stopped();

        if send_final_info {
            // Refresh stats from the search context as they may have changed from result.stats if we
            // hard-stopped.
            result.stats = SearchStats::from_ctx(&ctx);
            reporter.report_search_progress(game, &result);
        }

        reporter.best_move(game, result.mv);
    } else {
        stop_control.stopped();
    }

    thread_result
}

fn best_result(results: &SearchResults) -> SearchResult {
    let results = results.get();

    // For now, we assume that the main thread produced the best result
    let result = &results[0];

    result.clone()
}

// Simple single-threaded search used by utilities like bench, tests and datagen
pub fn st_search(
    game: &Game,
    persistent_state: &PersistentState,
    time_control: TimeControl,
    reporter: &dyn Reporter,
) -> SearchResult {
    search(
        game,
        persistent_state,
        &mut ThreadData::new(0),
        &SearchResults::new(1),
        time_control,
        &StopControl::new(1),
        &EngineOptions::DEFAULT,
        reporter,
    )
}

pub fn probe_tb_at_root(
    game: &Game,
    tb: &Tablebase,
    time_control: &TimeControl,
) -> Option<SearchResult> {
    let mut game = game.clone();
    let best_move = tb.best_move(&game)?;

    let start_time = match time_control {
        TimeControl::Clocks { start_time, .. } | TimeControl::ExactTime { start_time, .. } => {
            *start_time
        }
        _ => Instant::now(),
    };

    let player = game.player;

    let mut pv = PrincipalVariation::new();

    let tb_score = tb
        .wdl(&game)
        .expect("In tablebase position, but unable to get tablebase score");

    let mut eval = None;

    for _ in 0..MAX_SEARCH_DEPTH {
        let tablebase_move = tb
            .best_move(&game)
            .expect("In tablebase position, but unable to get tablebase move");

        pv.append(tablebase_move);
        game.make_move(tablebase_move);

        // Check if this move terminated the game, and return an appropriate score
        let legal_moves = game.moves();
        let king_in_check = game.in_check();

        if legal_moves.is_empty() {
            eval = Some(if king_in_check {
                let plies = pv.len();

                if game.player == player {
                    Eval::mated_in(plies)
                } else {
                    Eval::mate_in(plies)
                }
            } else {
                Eval::DRAW
            });

            break;
        }
    }

    let elapsed = start_time.elapsed();
    let depth = pv.len();
    let score = eval.unwrap_or_else(|| match tb_score {
        Wdl::Win => Eval::tb_mate_in(MAX_SEARCH_DEPTH),
        Wdl::Draw => Eval::DRAW,
        Wdl::Loss => Eval::tb_mated_in(MAX_SEARCH_DEPTH),
    });

    Some(SearchResult {
        id: 0,
        mv: best_move,
        pv,
        depth,
        seldepth: depth,
        score,
        stats: SearchStats {
            time: elapsed,
            nodes: u64::from(depth),
            tbhits: u64::from(depth),
            hashfull: 0,
        },
    })
}

pub fn iterative_deepening(
    game: &mut Game,
    ctx: &mut SearchContext<'_>,
    reporter: &dyn Reporter,
) -> SearchResult {
    let mut result: Option<SearchResult> = None;

    ctx.max_depth_reached = 0;

    for depth in 1..=MAX_SEARCH_DEPTH {
        let depth = Depth::new(depth);

        if !ctx.should_start_new_search(depth) {
            break;
        }

        ctx.root_depth = depth;

        let previous_eval = result.as_ref().map(|r| r.score);

        let mut pv = PrincipalVariation::new();
        let eval = aspiration_search(game, depth, previous_eval, &mut pv, ctx);

        if ctx.stopped() {
            ctx.was_hard_stopped = true;
            break;
        }

        let new_best_move = *pv.first().unwrap_or_else(|| {
            panic!("No PV move at depth {} for position {}", depth, game.to_fen())
        });

        ctx.update_after_search(new_best_move, depth);

        let this_result = SearchResult {
            id: ctx.id,
            mv: new_best_move,
            depth: depth.as_u8(),
            seldepth: ctx.max_depth_reached,
            score: eval,
            pv: pv.clone(),
            stats: SearchStats::from_ctx(ctx),
        };

        if !ctx.options.minimal {
            reporter.report_search_progress(game, &this_result);
        }

        result = Some(this_result);
    }

    result.expect("Should always end iterative deepening with a result")
}

pub fn aspiration_search(
    game: &mut Game,
    depth: Depth,
    eval: Option<Eval>,
    pv: &mut PrincipalVariation,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    const CLAMP_ALPHA: fn(Eval) -> Eval = |eval: Eval| -> Eval { std::cmp::max(Eval::MIN, eval) };
    const CLAMP_BETA: fn(Eval) -> Eval = |eval: Eval| -> Eval { std::cmp::min(Eval::MAX, eval) };
    const INCREASE_WIDTH: fn(i32) -> i32 = |width: i32| -> i32 { width + width / 2 };

    let mut width = aspiration_window_size();

    let mut window = if depth < aspiration_min_depth() || eval.is_some_and(Eval::is_decisive) {
        ScoreWindow::new(Eval::MIN, Eval::MAX)
    } else {
        let eval =
            eval.expect("Aspiration search should have an evaluation after it reaches min depth");
        ScoreWindow::new(CLAMP_ALPHA(eval - width), CLAMP_BETA(eval + width))
    };

    let mut reduction = 0;

    loop {
        // This would only make a difference if aspiration_max_reduction > aspiration_min_depth
        // but would allow dropping directly into quiescence which we don't want.
        let search_depth = (depth - reduction).max(Depth::new(1));

        let eval = negamax(game, window, search_depth, 0, false, pv, ctx);

        if ctx.stopped() {
            return Eval::MIN;
        }

        if eval <= window.alpha {
            window.beta = (window.alpha + window.beta) / 2;
            window.alpha = CLAMP_ALPHA(eval - width);
            width = INCREASE_WIDTH(width);
            reduction = 0;
        } else if eval >= window.beta {
            window.beta = CLAMP_BETA(eval + width);
            width = INCREASE_WIDTH(width);
            reduction = (reduction + 1).min(aspiration_max_reduction());
        } else {
            return eval;
        }
    }
}

pub fn negamax(
    game: &mut Game,
    mut s: ScoreWindow,
    mut depth: Depth,
    plies: u8,
    cut_node: bool,
    pv: &mut PrincipalVariation,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    // Check periodically to see if we're out of time.
    if ctx.stopped() {
        return Eval::MIN;
    }

    let is_root = plies == 0;
    let is_pv = !s.is_zero_window();
    let excluded_mv = ctx.stack.get(plies).excluded_mv;
    let in_singular_search = excluded_mv.is_some();

    ctx.stack.get(plies).double_extensions = if is_root {
        0
    } else {
        ctx.stack.last(plies).unwrap().double_extensions
    };

    // Check extension: If we're about to finish searching, but we are in check, we
    // should keep going.
    let in_check = game.in_check();
    if in_check {
        depth += 1;
    }

    if depth == 0 {
        return quiescence(game, s, plies, ctx);
    }

    ctx.max_depth_reached = ctx.max_depth_reached.max(plies);
    if !is_root {
        ctx.nodes.incr();
    }

    if !is_root {
        if game.is_draw(plies) {
            return Eval::DRAW;
        }

        if plies == MAX_SEARCH_DEPTH {
            return if in_check {
                Eval::DRAW
            } else {
                eval(ctx.nnue, game)
            };
        }

        // Mate distance pruning
        s.clamp_alpha(Eval::mated_in(plies));
        s.clamp_beta(Eval::mate_in(plies + 1));

        if s.alpha >= s.beta {
            return s.alpha;
        }
    }

    let mut previous_best_move: Option<Move> = None;
    let mut tt_pv = is_pv;

    let tt_entry = match excluded_mv {
        Some(_) => None,
        None => ctx.tt.get(game.hash, plies),
    };

    if let Some(ref tt_entry) = tt_entry {
        if !is_root && !is_pv && tt_entry.depth >= depth {
            let tt_score = tt_entry.score;

            match tt_entry.bound {
                NodeBound::Exact => return tt_score,
                NodeBound::Upper if tt_score <= s.alpha => return tt_score,
                NodeBound::Lower if tt_score >= s.beta => return tt_score,
                _ => {}
            }
        }

        tt_pv |= tt_entry.was_pv;
        previous_best_move = tt_entry.best_move;
    }

    #[allow(unused_mut, reason = "Will be mutated if compiled with syzygy")]
    let (mut syzygy_min, mut syzygy_max) = (Eval::mated_in(0), Eval::mate_in(0));

    #[cfg(feature = "syzygy")]
    if !is_root
        && !in_singular_search
        && ctx.tablebase.can_probe(game)
        && let Some(wdl) = ctx.tablebase.wdl(game)
    {
        use crate::engine::tablebases::Wdl;

        ctx.tbhits.incr();

        let (score, bound) = match wdl {
            Wdl::Win => (Eval::tb_mate_in(plies), NodeBound::Lower),
            Wdl::Draw => (Eval::DRAW, NodeBound::Exact),
            Wdl::Loss => (Eval::tb_mated_in(plies), NodeBound::Upper),
        };

        if bound == NodeBound::Exact
            || (bound == NodeBound::Lower && score >= s.beta)
            || (bound == NodeBound::Upper && score <= s.alpha)
        {
            ctx.tt
                .insert(game.hash, bound, None, score, Eval::NONE, depth, plies, tt_pv);

            return score;
        }

        if is_pv {
            if bound == NodeBound::Upper {
                syzygy_max = score;
            }

            if bound == NodeBound::Lower {
                s.clamp_alpha(score);
                syzygy_min = score;
            }
        }
    }

    let (raw_eval, eval) = if in_check {
        (Eval::NONE, Eval::NONE)
    } else if in_singular_search {
        (Eval::NONE, ctx.stack.get(plies).eval)
    } else if let Some(tt_entry) = &tt_entry {
        let raw_eval = if tt_entry.eval == Eval::NONE {
            eval(ctx.nnue, game)
        } else {
            tt_entry.eval
        };

        let eval = correct_eval(game, raw_eval, ctx, plies);

        (raw_eval, eval)
    } else {
        let raw_eval = eval(ctx.nnue, game);
        let eval = correct_eval(game, raw_eval, ctx, plies);

        ctx.tt.insert(
            game.hash,
            NodeBound::None,
            None,
            Eval::NONE,
            raw_eval,
            Depth::ZERO,
            plies,
            tt_pv,
        );

        (raw_eval, eval)
    };

    let mut score_estimate = eval;
    if !in_check
        && !in_singular_search
        && let Some(ref tt_entry) = tt_entry
        && match tt_entry.bound {
            NodeBound::None => false,
            NodeBound::Exact => true,
            NodeBound::Lower => tt_entry.score > eval,
            NodeBound::Upper => tt_entry.score < eval,
        }
    {
        score_estimate = tt_entry.score;
    }

    ctx.stack.get(plies).eval = eval;
    ctx.stack.get(plies + 1).fail_highs = 0;

    let improving = if in_check {
        false
    } else if let Some(prev2) = ctx.stack.get_prev(plies, 2)
        && prev2.eval != Eval::NONE
    {
        eval > prev2.eval
    } else if let Some(prev4) = ctx.stack.get_prev(plies, 4)
        && prev4.eval != Eval::NONE
    {
        eval > prev4.eval
    } else {
        false
    };

    // Hindsight extension
    if !is_root
        && !in_check
        && !in_singular_search
        && let Some(last) = ctx.stack.last(plies)
        && last.reduction >= 3
        && last.eval != Eval::NONE
        && eval + last.eval < 0
    {
        depth += 1;
    }

    if !is_root
        && !is_pv
        && !in_check
        && !in_singular_search
        && depth >= hindsight_extension_depth()
        && let Some(last) = ctx.stack.last(plies)
        && last.reduction >= 1
        && last.eval != Eval::NONE
        && eval + last.eval > hindsight_extension_margin()
    {
        depth -= 1;
    }

    let mut rfp_margin = 0;
    rfp_margin += depth * reverse_futility_prune_depth_margin();
    rfp_margin -= i32::from(improving) * reverse_futility_prune_improving_margin();

    // Reverse futility pruning
    if !is_root
        && !is_pv
        && !in_check
        && !in_singular_search
        && depth <= reverse_futility_prune_depth()
        && score_estimate - rfp_margin >= s.beta
    {
        return if !score_estimate.is_decisive() && !s.beta.is_decisive() {
            s.beta + (score_estimate - s.beta) / 3
        } else {
            score_estimate
        };
    }

    // Razoring
    if !is_root
        && !is_pv
        && !in_check
        && !in_singular_search
        && depth <= razoring_depth()
        && s.alpha.0.abs() < 2000
        && eval + depth * razoring_margin() <= s.alpha
    {
        let qsearch_score = quiescence(game, s.zero_window_around_alpha(), plies, ctx);
        if qsearch_score <= s.alpha {
            return qsearch_score;
        }
    }

    // Null move pruning
    if cut_node
        && !in_check
        && !in_singular_search
        && plies >= ctx.min_nmp_ply
        && eval >= s.beta
        // Don't let a player play a null move in response to a null move
        && ctx.stack.last(plies).is_some_and(|s| s.mv.is_some())
        && !game.zugzwang_likely()
    {
        ctx.tt.prefetch(game.approx_zobrist_after_null_move());

        let reduction = Depth::new(null_move_pruning_base_reduction())
            + depth / null_move_pruning_reduction_factor();

        ctx.stack.get(plies).mv = None;

        game.make_null_move();

        let null_score = -negamax(
            game,
            -s.zero_window_around_beta(),
            depth - reduction,
            plies + 1,
            false,
            &mut PrincipalVariation::new(),
            ctx,
        );

        game.undo_null_move();

        if ctx.stopped() {
            return Eval::MIN;
        }

        if null_score >= s.beta {
            if depth <= 14 || ctx.min_nmp_ply > 0 {
                return if null_score.is_decisive() {
                    s.beta
                } else {
                    null_score
                };
            }

            ctx.min_nmp_ply = plies + u8::try_from(((depth - reduction) * 3) / 4).unwrap_or(0);
            let verify_null_score = negamax(
                game,
                s.zero_window_around_beta(),
                depth - reduction,
                plies,
                false,
                &mut PrincipalVariation::new(),
                ctx,
            );
            ctx.min_nmp_ply = 0;

            if ctx.stopped() {
                return Eval::MIN;
            }

            if verify_null_score >= s.beta {
                return verify_null_score;
            }
        }
    }

    if !is_root && tt_entry.is_none() && depth >= iir_depth() {
        depth -= 1;
    }

    // Probcut
    if !is_pv
        && !in_singular_search
        && !in_check
        && let Some(ref e) = tt_entry
        && e.score != Eval::NONE
        && !e.score.is_decisive()
        && !s.beta.is_decisive()
        && (e.bound == NodeBound::Lower || e.bound == NodeBound::Exact)
        && e.score >= (s.beta + probcut_margin()).clamp_to_non_mate()
        && e.depth >= depth - probcut_depth_diff()
    {
        return e.score;
    }

    // Singular extension
    let mut extension: i8 = 0;

    let singular_extension_candidate = tt_entry
        .as_ref()
        .filter(|entry| {
            depth >= singular_extension_depth()
                && !is_root
                && !in_singular_search
                && entry.bound != NodeBound::Upper
                && entry.depth >= depth - singular_extension_entry_depth_delta()
                && !entry.score.is_decisive()
        })
        .and_then(|entry| entry.best_move);

    if let Some(mv) = singular_extension_candidate {
        let tt_score = tt_entry.as_ref().unwrap().score;

        let se_beta = tt_score - depth * singular_extension_margin();
        let se_depth = (depth - 1) / 2u8;

        ctx.stack.get(plies).excluded_mv = Some(mv);
        let se_score = negamax(
            game,
            ScoreWindow::new(se_beta - 1, se_beta),
            se_depth,
            plies,
            cut_node,
            &mut PrincipalVariation::new(),
            ctx,
        );
        ctx.stack.get(plies).excluded_mv = None;

        if se_score < se_beta {
            extension = 1;

            if !is_pv
                && se_score < se_beta - double_extension_margin()
                && ctx.stack.get(plies).double_extensions <= double_extension_max()
            {
                extension = 2;
                ctx.stack.get(plies).double_extensions += 1;
            }
        } else if se_beta >= s.beta {
            return se_beta;
        } else if !is_pv && !se_score.is_decisive() && se_score >= s.beta {
            return se_score;
        } else if tt_score >= s.beta {
            extension = -1;
        }
    }

    let mut tt_node_bound = NodeBound::Upper;
    let mut best_move = None;
    let mut best_score = Eval::MIN;

    let mut moves = MovePicker::new(previous_best_move);
    let mut legal_moves = 0;
    let mut moves_tried = 0;
    let mut node_pv = PrincipalVariation::new();

    let mut tacticals_tried = MoveList::new();
    let mut quiets_tried = MoveList::new();

    while let Some(mv) = moves.next(game, ctx.tables, ctx.stack, plies) {
        legal_moves += 1;

        if Some(mv) == excluded_mv {
            continue;
        }

        ctx.tt.prefetch(game.approx_zobrist_after(mv));

        node_pv.clear();

        let history = if mv.is_quiet() {
            ctx.tables.quiet_history.get(game, mv)
                + ctx.tables.conthist.get(game, ctx.stack, plies, mv)
        } else {
            ctx.tables.tactical_history.get(game, mv)
        };

        let lmr_depth = depth - lmr_reduction(depth, moves_tried, mv.is_quiet());

        // Futility pruning
        if !is_root
            && !is_pv
            && !in_check
            && mv.is_quiet()
            && lmr_depth <= futility_prune_depth()
            && eval + futility_prune_base_value() + lmr_depth * futility_prune_depth_multiplier()
                <= s.alpha
            && !best_score.is_loss()
        {
            moves.skip_quiets();
            continue;
        }

        if !is_root
            && !is_pv
            && !best_score.is_loss()
            && mv.is_quiet()
            && lmr_depth <= history_prune_depth()
            && history < history_prune_offset() + lmr_depth * history_prune_margin()
        {
            moves.skip_quiets();
            continue;
        }

        if lmr_depth <= see_prune_depth()
            && moves.stage > GenStage::GoodTacticals
            && !is_root
            && !is_pv
            && !best_score.is_loss()
        {
            let margin = if mv.is_quiet() {
                lmr_depth * lmr_depth * see_quiet_margin()
            } else {
                lmr_depth * see_capture_margin() - (history / see_prune_history_divisor())
            };

            if !see(game, mv, margin) {
                continue;
            }
        }

        let lmp_moves = (lmp_move_threshold() as usize + (lmr_depth.idx() * lmr_depth.idx()))
            / (1 + usize::from(!improving));

        if lmr_depth <= lmp_depth()
            && !is_root
            && !is_pv
            && !in_check
            && !game.is_direct_check(mv)
            && mv.is_quiet()
            && moves_tried >= lmp_moves
            && !best_score.is_loss()
        {
            moves.skip_quiets();
            continue;
        }

        let nodes_before = ctx.nodes.get();
        ctx.stack.get(plies).mv = Some((mv, game.board.piece_guaranteed_at(mv.from())));

        game.make_move_observed(mv, ctx.nnue.next_changes());
        moves_tried += 1;

        // Only apply the extension to the singular move
        let extension = if Some(mv) == singular_extension_candidate {
            extension
        } else {
            0
        };

        let search_depth = depth + extension - 1;
        let mut score = Eval::NONE;

        if depth >= lmr_start_depth()
            && moves_tried >= lmr_move_threshold() as usize + usize::from(is_root)
        {
            let reduction = {
                let mut r = i32::from(lmr_reduction(depth, moves_tried, mv.is_quiet())) * 1024;

                // Reducing more
                r += i32::from(cut_node) * lmr_cut_node_factor();
                r += i32::from(!is_pv) * lmr_is_not_pv_factor();
                r += i32::from(ctx.stack.get(plies + 1).fail_highs > 2)
                    * lmr_many_fail_highs_factor();

                // Reducing less:
                r -= i32::from(in_check) * lmr_in_check_factor();
                r -= i32::from(!is_root && tt_pv) * lmr_ttpv_factor();

                r / 1024
            };

            let reduced_search_depth = Depth::new(
                (search_depth.as_i32() - reduction).clamp(1, search_depth.as_i32()) as u8,
            );

            // We already found a good move (i.e. we raised alpha).
            // Now, we just need to prove that the other moves are worse.
            // We search them with a reduced window to prove that they are at least worse.

            ctx.stack.get(plies).reduction = reduction;
            score = -negamax(
                game,
                -s.zero_window_around_alpha(),
                reduced_search_depth,
                plies + 1,
                true,
                &mut node_pv,
                ctx,
            );
            ctx.stack.get(plies).reduction = 0;

            // If we raised alpha, but we were searching with reduced depth, we probably want to double
            // check we didn't miss something, so search without the reduction.
            if score > s.alpha && search_depth > reduced_search_depth {
                score = -negamax(
                    game,
                    -s.zero_window_around_alpha(),
                    search_depth,
                    plies + 1,
                    !cut_node,
                    &mut node_pv,
                    ctx,
                );
            }
        } else if !is_pv || moves_tried > 1 {
            score = -negamax(
                game,
                -s.zero_window_around_alpha(),
                search_depth,
                plies + 1,
                !cut_node,
                &mut node_pv,
                ctx,
            );
        }

        if is_pv && (moves_tried == 1 || score > s.alpha) {
            score = -negamax(game, -s, search_depth, plies + 1, false, &mut node_pv, ctx);
        }

        game.undo_move();
        ctx.nnue.pop();

        if is_root {
            let nodes_for_this_move = ctx.nodes.get() - nodes_before;
            ctx.update_nodes_used(mv, nodes_for_this_move);
        }

        if ctx.stopped() {
            return Eval::MIN;
        }

        if score > best_score {
            best_score = score;

            if score > s.alpha {
                s.alpha = score;
                best_move = Some(mv);
                tt_node_bound = NodeBound::Exact;
                pv.push(mv, &node_pv);
            }

            // Cutoff: This move is so good that our opponent won't let it be played.
            if score >= s.beta {
                tt_node_bound = NodeBound::Lower;
                ctx.stack.get(plies).fail_highs += 1;
                break;
            }
        }

        // Only add to the tried lists if the move didn't cause a cutoff
        if !mv.is_quiet() {
            tacticals_tried.push(mv);
        }

        if mv.is_quiet() {
            quiets_tried.push(mv);
        }
    }

    if legal_moves == 0 {
        if in_singular_search {
            return s.alpha;
        }

        return if game.in_check() {
            Eval::mated_in(plies)
        } else {
            Eval::DRAW
        };
    }

    best_score = best_score.clamp(syzygy_min, syzygy_max);

    if tt_node_bound == NodeBound::Lower
        && let Some(mv) = best_move
    {
        let static_eval_failed_low = !in_check && eval <= s.alpha;
        let history_depth = depth + u8::from(static_eval_failed_low);

        ctx.tables
            .tactical_history
            .update(mv, game, history_depth, &tacticals_tried);

        if mv.is_quiet() {
            ctx.tables
                .conthist
                .update(game, ctx.stack, plies, mv, history_depth, &quiets_tried);

            ctx.tables
                .quiet_history
                .update(game, mv, history_depth, &quiets_tried);
        }
    }

    if !(in_singular_search
        || in_check
        || best_move.is_some_and(|m| !m.is_quiet())
        || tt_node_bound == NodeBound::Lower && best_score <= eval
        || tt_node_bound == NodeBound::Upper && best_score >= eval)
    {
        ctx.tables
            .corrhist
            .update(game, ctx.stack, plies, depth, best_score - eval);
    }

    if !in_singular_search {
        ctx.tt.insert(
            game.hash,
            tt_node_bound,
            best_move,
            best_score,
            raw_eval,
            depth,
            plies,
            tt_pv,
        );
    }

    best_score
}

pub fn quiescence(
    game: &mut Game,
    mut s: ScoreWindow,
    plies: u8,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    // Check periodically to see if we're out of time.
    if ctx.stopped() {
        return Eval::MIN;
    }

    let is_pv = !s.is_zero_window();

    ctx.max_depth_reached = ctx.max_depth_reached.max(plies);
    ctx.nodes.incr();

    if game.is_draw(plies) {
        return Eval::DRAW;
    }

    let in_check = game.in_check();

    if plies == MAX_SEARCH_DEPTH {
        return if in_check {
            Eval::DRAW
        } else {
            eval(ctx.nnue, game)
        };
    }

    let tt_entry = ctx.tt.get(game.hash, plies);
    let mut previous_best_move = None;
    let mut tt_pv = is_pv;

    if let Some(ref tt_entry) = tt_entry {
        if !is_pv {
            let tt_score = tt_entry.score;

            match tt_entry.bound {
                NodeBound::Exact => return tt_score,
                NodeBound::Upper if tt_score <= s.alpha => return tt_score,
                NodeBound::Lower if tt_score >= s.beta => return tt_score,
                _ => {}
            }
        }

        tt_pv |= tt_entry.was_pv;
        previous_best_move = tt_entry.best_move;
    }

    let (raw_eval, eval) = if in_check {
        (Eval::NONE, Eval::NONE)
    } else if let Some(tt_entry) = tt_entry {
        let raw_eval = if tt_entry.eval == Eval::NONE {
            eval(ctx.nnue, game)
        } else {
            tt_entry.eval
        };

        let eval = correct_eval(game, raw_eval, ctx, plies);

        (raw_eval, eval)
    } else {
        let raw_eval = eval(ctx.nnue, game);
        let eval = correct_eval(game, raw_eval, ctx, plies);

        ctx.tt.insert(
            game.hash,
            NodeBound::None,
            None,
            Eval::NONE,
            raw_eval,
            Depth::ZERO,
            plies,
            tt_pv,
        );

        (raw_eval, eval)
    };

    if eval >= s.beta {
        return if !eval.is_decisive() && !s.beta.is_decisive() {
            (eval + s.beta) / 2
        } else {
            eval
        };
    }

    if eval > s.alpha {
        s.alpha = eval;
    }

    let mut best_score = eval;
    let mut best_move = None;
    let mut node_bound = NodeBound::Upper;
    let mut legal_moves = 0;
    let mut moves_tried = 0;
    let futility_score = eval + quiescence_futility_margin();

    let mut moves = MovePicker::new(previous_best_move);

    if !in_check {
        moves.skip_quiets();
    }

    while let Some(mv) = moves.next(game, ctx.tables, ctx.stack, plies) {
        legal_moves += 1;

        if !best_score.is_loss() && moves.stage >= GenStage::BadTacticals {
            break;
        }

        if !best_score.is_loss() && !in_check && moves_tried >= quiescence_lmp_move_threshold() {
            break;
        }

        if !best_score.is_loss() && !in_check && futility_score <= s.alpha && !see(game, mv, 1) {
            if best_score < futility_score {
                best_score = futility_score;
            }

            continue;
        }

        ctx.tt.prefetch(game.approx_zobrist_after(mv));

        ctx.stack.get(plies).mv = Some((mv, game.board.piece_guaranteed_at(mv.from())));

        game.make_move_observed(mv, ctx.nnue.next_changes());
        moves_tried += 1;

        let move_score = -quiescence(game, -s, plies + 1, ctx);

        game.undo_move();
        ctx.nnue.pop();

        if ctx.stopped() {
            return Eval::MIN;
        }

        if mv.is_quiet() && !move_score.is_loss() {
            moves.skip_quiets();
        }

        if move_score > best_score {
            best_score = move_score;

            if move_score > s.alpha {
                best_move = Some(mv);
                node_bound = NodeBound::Exact;
                s.alpha = move_score;
            }

            // Cutoff: This move is so good that our opponent won't let it be played.
            if move_score >= s.beta {
                node_bound = NodeBound::Lower;
                break;
            }
        }
    }

    if in_check && legal_moves == 0 {
        return Eval::mated_in(plies);
    }

    ctx.tt.insert(
        game.hash,
        node_bound,
        best_move,
        best_score,
        raw_eval,
        Depth::new(0),
        plies,
        tt_pv,
    );

    best_score
}

fn correct_eval(game: &Game, raw_eval: Eval, ctx: &SearchContext<'_>, plies: u8) -> Eval {
    (raw_eval + ctx.tables.corrhist.get(game, ctx, plies)).clamp_to_non_mate()
}
