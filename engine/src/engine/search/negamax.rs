use super::{MAX_SEARCH_DEPTH, SearchContext};
use crate::{
    chess::{
        game::Game,
        moves::{Move, MoveList},
    },
    engine::{
        eval,
        eval::Eval,
        params::*,
        search::{
            move_picker::{GenStage, MovePicker},
            principal_variation::PrincipalVariation,
            quiescence::quiescence,
            tables::lmr_table::lmr_reduction,
            types::{Depth, DepthReduction},
        },
        see::see,
        tablebases::Wdl,
        transposition_table::NodeBound,
    },
};

pub fn negamax(
    game: &mut Game,
    mut alpha: Eval,
    mut beta: Eval,
    mut depth: Depth,
    plies: u8,
    cut_node: bool,
    pv: &mut PrincipalVariation,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    // Check periodically to see if we're out of time.
    ctx.time_control.update(ctx.nodes.get(), ctx.root_depth);
    if ctx.time_control.stopped() {
        return Eval::MIN;
    }

    let is_root = plies == 0;
    let is_pv = alpha != beta - Eval(1);
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
        return quiescence(game, alpha, beta, plies, ctx);
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
        alpha = alpha.max(Eval::mated_in(plies));
        beta = beta.min(Eval::mate_in(plies + 1));

        if alpha >= beta {
            return alpha;
        }
    }

    let mut previous_best_move: Option<Move> = None;

    let tt_entry = match excluded_mv {
        Some(_) => None,
        None => ctx.tt.get(game.hash, plies),
    };

    if let Some(ref tt_entry) = tt_entry {
        if !is_root && !is_pv && tt_entry.depth >= depth {
            let tt_score = tt_entry.score;

            match tt_entry.bound {
                NodeBound::Exact => return tt_score,
                NodeBound::Upper if tt_score <= alpha => return tt_score,
                NodeBound::Lower if tt_score >= beta => return tt_score,
                _ => {}
            }
        }

        previous_best_move = tt_entry.best_move;
    }

    let (mut syzygy_min, mut syzygy_max) = (Eval::mated_in(0), Eval::mate_in(0));

    let tb_cardinality = ctx.tablebase.n_men();
    if !is_root && excluded_mv.is_none() && tb_cardinality > 0 {
        let piece_count = game.board.occupancy().count();

        if piece_count <= tb_cardinality
            && let Some(wdl) = ctx.tablebase.wdl(game)
        {
            ctx.tbhits.incr();

            let score = match wdl {
                Wdl::Win => Eval::tb_mate_in(plies),
                Wdl::Draw => Eval::DRAW,
                Wdl::Loss => Eval::tb_mated_in(plies),
            };

            let tb_bound = match wdl {
                Wdl::Win => NodeBound::Lower,
                Wdl::Loss => NodeBound::Upper,
                Wdl::Draw => NodeBound::Exact,
            };

            if tb_bound == NodeBound::Exact
                || (tb_bound == NodeBound::Lower && score >= beta)
                || (tb_bound == NodeBound::Upper && score <= alpha)
            {
                ctx.tt
                    .insert(game.hash, tb_bound, None, score, Eval::NONE, depth, plies);

                return score;
            }

            if is_pv {
                if tb_bound == NodeBound::Upper {
                    syzygy_max = score;
                }

                if tb_bound == NodeBound::Lower {
                    alpha = alpha.max(score);
                    syzygy_min = score;
                }
            }
        }
    }

    let raw_eval = if excluded_mv.is_some() || in_check {
        Eval::NONE
    } else {
        match tt_entry {
            Some(ref e) if e.eval != Eval::NONE => e.eval,
            _ => {
                let e = eval::eval(ctx.nnue, game);

                ctx.tt
                    .insert(game.hash, NodeBound::None, None, Eval::NONE, e, Depth::ZERO, plies);

                e
            }
        }
    };

    let eval = if raw_eval == Eval::NONE {
        Eval::NONE
    } else {
        (raw_eval + ctx.tables.corrhist.get(game)).clamp_to_non_mate()
    };

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

    // Reverse futility pruning
    if !is_root
        && !is_pv
        && !in_check
        && excluded_mv.is_none()
        && depth <= reverse_futility_prune_depth()
        && eval - depth * reverse_futility_prune_margin_per_ply() > beta
    {
        return if !eval.is_decisive() && !beta.is_decisive() {
            beta + (eval - beta) / 3
        } else {
            eval
        };
    }

    // Razoring
    if !is_root
        && !is_pv
        && !in_check
        && excluded_mv.is_none()
        && depth <= razoring_depth()
        && alpha.0.abs() < 2000
        && eval + depth * razoring_margin() <= alpha
    {
        let qsearch_score = quiescence(game, alpha, alpha + 1, plies, ctx);
        if qsearch_score <= alpha {
            return qsearch_score;
        }
    }

    // Null move pruning
    if cut_node
        && !in_check
        && excluded_mv.is_none()
        && eval >= beta
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
            -beta,
            -beta + Eval(1),
            depth - reduction,
            plies + 1,
            false,
            &mut PrincipalVariation::new(),
            ctx,
        );

        game.undo_null_move();

        if ctx.time_control.stopped() {
            return Eval::MIN;
        }

        if null_score >= beta {
            return null_score;
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
        let mut se_pv = PrincipalVariation::new();
        let tt_score = tt_entry.as_ref().unwrap().score;

        let se_depth = (depth - 1) / 2u8;
        let se_beta = tt_score - depth * singular_extension_margin();

        ctx.stack.get(plies).excluded_mv = Some(mv);
        let value =
            negamax(game, se_beta - Eval(1), se_beta, se_depth, plies, cut_node, &mut se_pv, ctx);
        ctx.stack.get(plies).excluded_mv = None;

        if value < se_beta {
            extension = 1;

            if !is_pv
                && value + double_extension_margin() < se_beta
                && ctx.stack.get(plies).double_extensions <= double_extension_max()
            {
                extension = 2;
                ctx.stack.get(plies).double_extensions += 1;
            }
        } else if !is_pv && !value.is_decisive() && value >= beta {
            return value;
        } else if tt_score >= beta {
            extension = -1;
        }
    }

    ctx.tables.killer_moves.clear(plies + 1);

    let mut tt_node_bound = NodeBound::Upper;
    let mut best_move = None;
    let mut best_score = Eval::MIN;

    let mut moves = MovePicker::new(previous_best_move);
    let mut number_of_legal_moves = 0;
    let mut node_pv = PrincipalVariation::new();

    let mut captures_tried = MoveList::new();
    let mut quiets_tried = MoveList::new();

    while let Some(mv) = moves.next(game, ctx.tables, ctx.stack, plies) {
        if Some(mv) == excluded_mv {
            continue;
        }

        ctx.tt.prefetch(game.approx_zobrist_after(mv));

        node_pv.clear();

        // Futility pruning
        if number_of_legal_moves > 0
            && !is_pv
            && !mv.is_capture()
            && !in_check
            && depth <= futility_prune_depth()
            && eval + futility_prune_max_move_value() < alpha
        {
            continue;
        }

        if depth <= see_prune_depth()
            && moves.stage > GenStage::GoodTacticals
            && !is_root
            && !is_pv
            && !best_score.is_loss()
        {
            let lmr_depth = depth - lmr_reduction(depth, number_of_legal_moves);

            let margin = if mv.is_quiet() {
                lmr_depth * lmr_depth * see_quiet_margin()
            } else {
                let history_mod = if mv.is_capture() {
                    ctx.tables.capture_history.get(game, mv) / see_prune_history_divisor()
                } else {
                    0
                };

                lmr_depth * see_capture_margin() - history_mod
            };

            if !see(game, mv, Eval(margin)) {
                continue;
            }
        }

        let lmp_moves = (lmp_move_threshold() as usize + (depth.idx() * depth.idx()))
            / (1 + usize::from(!improving));

        if depth <= lmp_depth()
            && !is_root
            && !is_pv
            && !in_check
            && number_of_legal_moves >= lmp_moves
            && moves.stage > GenStage::Killer
            && !best_score.is_decisive()
        {
            moves.yield_only_tacticals();
        }

        let nodes_before = ctx.nodes.get();
        ctx.stack.get(plies).mv = Some((mv, game.board.piece_guaranteed_at(mv.from())));

        game.make_move_nnue(mv, ctx.nnue.next_changes());
        number_of_legal_moves += 1;

        // Only apply the extension to the singular move
        let extension = if Some(mv) == singular_extension_candidate {
            extension
        } else {
            0
        };

        let search_depth = depth + extension - 1;

        let move_score = if number_of_legal_moves == 1 {
            -negamax(
                game,
                -beta,
                -alpha,
                search_depth,
                plies + 1,
                !is_pv && !cut_node,
                &mut node_pv,
                ctx,
            )
        } else {
            let reduction =
                if depth >= lmr_depth() && number_of_legal_moves >= lmr_move_threshold() as usize {
                    let mut reduction =
                        DepthReduction::new(lmr_reduction(depth, number_of_legal_moves));

                    // Reducing more:
                    reduction.reduce_more_if(cut_node, lmr_cut_node_factor());

                    reduction.reduce_more_if(!is_pv, lmr_is_not_pv_factor());

                    reduction.reduce_more_if(
                        ctx.stack.get(plies + 1).fail_highs > 2,
                        lmr_many_fail_highs_factor(),
                    );

                    // Reducing less:
                    reduction.reduce_less_if(in_check, lmr_in_check_factor());

                    reduction.value()
                } else {
                    0
                };

            let reduced_search_depth = search_depth - reduction;

            // We already found a good move (i.e. we raised alpha).
            // Now, we just need to prove that the other moves are worse.
            // We search them with a reduced window to prove that they are at least worse.
            let mut pvs_score = -negamax(
                game,
                -alpha - Eval(1),
                -alpha,
                reduced_search_depth,
                plies + 1,
                true,
                &mut node_pv,
                ctx,
            );

            // If we raised alpha, but we were searching with reduced depth, we probably want to double
            // check we didn't miss something, so search without the reduction.
            if pvs_score > alpha && reduction > 0 {
                pvs_score = -negamax(
                    game,
                    -alpha - Eval(1),
                    -alpha,
                    search_depth,
                    plies + 1,
                    !cut_node,
                    &mut node_pv,
                    ctx,
                );
            }

            // If searching at full depth STILL raised alpha, re-search with normal alpha/beta
            // bounds.
            if pvs_score > alpha && pvs_score < beta {
                -negamax(game, -beta, -alpha, search_depth, plies + 1, false, &mut node_pv, ctx)
            } else {
                pvs_score
            }
        };

        game.undo_move();
        ctx.nnue.pop();

        if is_root {
            let nodes_for_this_move = ctx.nodes.get() - nodes_before;
            ctx.time_control.update_nodes_used(mv, nodes_for_this_move);
        }

        if ctx.time_control.stopped() {
            return Eval::MIN;
        }

        if move_score > best_score {
            best_score = move_score;

            if move_score > alpha {
                alpha = move_score;
                best_move = Some(mv);
                tt_node_bound = NodeBound::Exact;
                pv.push(mv, &node_pv);
            }

            // Cutoff: This move is so good that our opponent won't let it be played.
            if move_score >= beta {
                tt_node_bound = NodeBound::Lower;
                ctx.stack.get(plies).fail_highs += 1;
                break;
            }
        }

        // Only add to the tried lists if the move didn't cause a cutoff
        if mv.is_capture() {
            captures_tried.push(mv);
        }

        if mv.is_quiet() {
            quiets_tried.push(mv);
        }
    }

    if number_of_legal_moves == 0 {
        if excluded_mv.is_some() {
            return alpha;
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
                .capture_history
                .update(mv, game, depth, &captures_tried);

            // 'Killers': if a move was so good that it caused a beta cutoff,
            // but it wasn't a capture, we remember it so that we can try it
            // before other quiet moves.
            if !mv.is_capture() {
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
            || best_move.is_some_and(|m| m.is_capture() || m.is_promotion())
            || tt_node_bound == NodeBound::Lower && best_score <= eval
            || tt_node_bound == NodeBound::Upper && best_score >= eval)
        {
            ctx.tables.corrhist.update(game, depth, best_score - eval);
        }

        ctx.tt
            .insert(game.hash, tt_node_bound, best_move, best_score, raw_eval, depth, plies);
    }

    best_score
}
