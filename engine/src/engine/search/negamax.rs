use super::{MAX_SEARCH_DEPTH, SearchContext};
use crate::{
    chess::{Game, Move, moves::MoveList},
    engine::{
        eval::{self, Eval},
        params::*,
        search::{
            move_picker::{GenStage, MovePicker},
            principal_variation::PrincipalVariation,
            quiescence::quiescence,
            tables::lmr_reduction,
            types::{Depth, DepthReduction, ScoreWindow},
        },
        see::see,
        transposition_table::NodeBound,
    },
};

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
                eval::eval(ctx.nnue, game)
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
        && excluded_mv.is_none()
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
    } else if excluded_mv.is_some() {
        (Eval::NONE, ctx.stack.get(plies).eval)
    } else if let Some(tt_entry) = &tt_entry {
        let raw_eval = if tt_entry.eval == Eval::NONE {
            eval::eval(ctx.nnue, game)
        } else {
            tt_entry.eval
        };

        let eval = (raw_eval + ctx.tables.corrhist.get(game, ctx, plies)).clamp_to_non_mate();

        (raw_eval, eval)
    } else {
        let raw_eval = eval::eval(ctx.nnue, game);
        let eval = (raw_eval + ctx.tables.corrhist.get(game, ctx, plies)).clamp_to_non_mate();

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
        && excluded_mv.is_none()
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

    let mut rfp_margin = 0;
    rfp_margin += depth * reverse_futility_prune_depth_margin();
    rfp_margin -= i32::from(improving) * reverse_futility_prune_improving_margin();

    // Reverse futility pruning
    if !is_root
        && !is_pv
        && !in_check
        && excluded_mv.is_none()
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
        && excluded_mv.is_none()
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
        && excluded_mv.is_none()
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
            return if null_score.is_decisive() {
                s.beta
            } else {
                null_score
            };
        }
    }

    if !is_root && tt_entry.is_none() && depth >= iir_depth() {
        depth -= 1;
    }

    // Singular extension
    let mut extension: i8 = 0;

    let singular_extension_candidate = tt_entry
        .as_ref()
        .filter(|entry| {
            depth >= singular_extension_depth()
                && !is_root
                && excluded_mv.is_none()
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
            ScoreWindow::new(se_beta - Eval(1), se_beta),
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

    ctx.tables.killer_moves.clear(plies + 1);

    let mut tt_node_bound = NodeBound::Upper;
    let mut best_move = None;
    let mut best_score = Eval::MIN;

    let mut moves = MovePicker::new(previous_best_move);
    let mut moves_tried = 0;
    let mut node_pv = PrincipalVariation::new();

    let mut tacticals_tried = MoveList::new();
    let mut quiets_tried = MoveList::new();

    while let Some(mv) = moves.next(game, ctx.tables, ctx.stack, plies) {
        if Some(mv) == excluded_mv {
            continue;
        }

        ctx.tt.prefetch(game.approx_zobrist_after(mv));

        node_pv.clear();

        let lmr_depth = depth - lmr_reduction(depth, moves_tried);

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

        let quiet_history = ctx.tables.quiet_history.get(game, mv)
            + ctx.tables.conthist.get(game, ctx.stack, plies, mv);

        if !is_root
            && !is_pv
            && !best_score.is_loss()
            && mv.is_quiet()
            && lmr_depth <= history_prune_depth()
            && quiet_history < history_prune_offset() + lmr_depth * history_prune_margin()
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
                let history_mod =
                    ctx.tables.tactical_history.get(game, mv) / see_prune_history_divisor();

                lmr_depth * see_capture_margin() - history_mod
            };

            if !see(game, mv, Eval(margin)) {
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
                let mut r = DepthReduction::new(lmr_reduction(depth, moves_tried));

                // Reducing more:
                r.reduce_more_if(cut_node, lmr_cut_node_factor());

                r.reduce_more_if(!is_pv, lmr_is_not_pv_factor());

                r.reduce_more_if(
                    ctx.stack.get(plies + 1).fail_highs > 2,
                    lmr_many_fail_highs_factor(),
                );

                // Reducing less:
                r.reduce_less_if(in_check, lmr_in_check_factor());

                r.reduce_less_if(!is_root && tt_pv, lmr_ttpv_factor());

                r.value()
            };

            let reduced_search_depth = search_depth - reduction;

            // We already found a good move (i.e. we raised alpha).
            // Now, we just need to prove that the other moves are worse.
            // We search them with a reduced window to prove that they are at least worse.
            score = -negamax(
                game,
                -s.zero_window_around_alpha(),
                reduced_search_depth,
                plies + 1,
                true,
                &mut node_pv,
                ctx,
            );

            // If we raised alpha, but we were searching with reduced depth, we probably want to double
            // check we didn't miss something, so search without the reduction.
            if score > s.alpha && reduction > 0 {
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

    if moves_tried == 0 {
        if excluded_mv.is_some() {
            return s.alpha;
        }

        return if game.in_check() {
            Eval::mated_in(plies)
        } else {
            Eval::DRAW
        };
    }

    best_score = best_score.clamp(syzygy_min, syzygy_max);

    if excluded_mv.is_none() {
        if tt_node_bound == NodeBound::Lower
            && let Some(mv) = best_move
        {
            ctx.tables
                .tactical_history
                .update(mv, game, depth, &tacticals_tried);

            // 'Killers': if a move was so good that it caused a beta cutoff,
            // but it wasn't a capture, we remember it so that we can try it
            // before other quiet moves.
            if mv.is_quiet() {
                ctx.tables.killer_moves.set(plies, mv);

                ctx.tables
                    .conthist
                    .update(game, ctx.stack, plies, mv, depth, &quiets_tried);

                ctx.tables
                    .quiet_history
                    .update(game, mv, depth, &quiets_tried);
            }
        }

        if !(in_check
            || best_move.is_some_and(|m| !m.is_quiet())
            || tt_node_bound == NodeBound::Lower && best_score <= eval
            || tt_node_bound == NodeBound::Upper && best_score >= eval)
        {
            ctx.tables
                .corrhist
                .update(game, ctx.stack, plies, depth, best_score - eval);
        }

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
