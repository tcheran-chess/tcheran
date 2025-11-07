use std::cmp::max;

use super::{MAX_SEARCH_DEPTH, SearchContext};
use crate::{
    chess::{
        game::Game,
        moves::{Move, MoveList},
    },
    engine::{
        eval,
        eval::Eval,
        search::{
            move_picker::{GenStage, MovePicker},
            principal_variation::PrincipalVariation,
            quiescence::quiescence,
            tables::lmr_table::lmr_reduction,
        },
        see::see,
        tablebases::Wdl,
        transposition_table::NodeBound,
    },
};

pub struct DepthReduction(u8);

impl DepthReduction {
    #[inline]
    pub fn reduce_more_if(&mut self, predicate: bool) {
        self.0 = self.0.saturating_add(u8::from(predicate));
    }

    #[inline]
    pub fn reduce_less_if(&mut self, predicate: bool) {
        self.0 = self.0.saturating_sub(u8::from(predicate));
    }

    #[inline]
    pub fn value(&self) -> u8 {
        max(1, self.0)
    }
}

pub fn negamax(
    game: &mut Game,
    mut alpha: Eval,
    beta: Eval,
    mut depth: u8,
    plies: u8,
    pv: &mut PrincipalVariation,
    ctx: &mut SearchContext<'_>,
) -> Eval {
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
    let in_check = game.is_king_in_check();
    if in_check {
        depth += 1;
    }

    if depth == 0 {
        return quiescence(game, alpha, beta, plies, ctx);
    }

    ctx.max_depth_reached = ctx.max_depth_reached.max(plies);
    if !is_root {
        ctx.nodes_visited.incr();
    }

    // Check periodically to see if we're out of time.
    ctx.time_control.update(ctx.nodes_visited.get());
    if ctx.time_control.stopped() {
        return Eval::MIN;
    }

    if !is_root {
        if game.is_draw() {
            return Eval::DRAW;
        }

        if plies == MAX_SEARCH_DEPTH {
            return if in_check {
                Eval::DRAW
            } else {
                eval::eval(ctx.nnue, game.player)
            };
        }
    }

    let mut previous_best_move: Option<Move> = None;

    let tt_entry = match excluded_mv {
        Some(_) => None,
        None => ctx.tt.get(game.zobrist, plies),
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

    let tb_cardinality = ctx.tablebase.n_men();
    if !is_root && excluded_mv.is_none() && tb_cardinality > 0 {
        let piece_count = game.board.occupancy().count();

        if (piece_count < tb_cardinality || (piece_count <= tb_cardinality && depth >= 1))
            && let Some(wdl) = ctx.tablebase.wdl(game)
        {
            ctx.tbhits.incr();

            let score = match wdl {
                Wdl::Win => Eval::mate_in(plies),
                Wdl::Draw => Eval::DRAW,
                Wdl::Loss => Eval::mated_in(plies),
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
                    .insert(game.zobrist, tb_bound, None, score, Eval::NONE, depth, plies);

                return score;
            }

            if is_pv && tb_bound == NodeBound::Lower {
                alpha = alpha.max(score);
            }
        }
    }

    let eval = if excluded_mv.is_some() || in_check {
        Eval::NONE
    } else {
        match tt_entry {
            Some(ref e) if e.eval != Eval::NONE => e.eval,
            _ => {
                let e = eval::eval(ctx.nnue, game.player);

                ctx.tt
                    .insert(game.zobrist, NodeBound::None, None, Eval::NONE, e, 0, plies);

                e
            }
        }
    };

    ctx.stack.get(plies).eval = eval;

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

    if !is_root && !is_pv && !in_check && excluded_mv.is_none() {
        // Reverse futility pruning
        if depth <= ctx.params.reverse_futility_prune_depth
            && eval - ctx.params.reverse_futility_prune_margin_per_ply * i32::from(depth) > beta
        {
            return beta + (eval - beta) / 3;
        }

        // Null move pruning
        if eval >= beta
            // Don't let a player play a null move in response to a null move
            && ctx.stack.last(plies).is_some_and(|s| s.mv.is_some())
            && !game.zugzwang_likely()
        {
            ctx.tt.prefetch(game.approx_zobrist_after_null_move());

            let reduction = ctx.params.null_move_pruning_base_reduction
                + depth / ctx.params.null_move_pruning_reduction_factor;

            ctx.stack.get(plies).mv = None;

            game.make_null_move();

            let null_score = -negamax(
                game,
                -beta,
                -beta + Eval(1),
                depth.saturating_sub(reduction),
                plies + 1,
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
    }

    if !is_root && tt_entry.is_none() && depth >= ctx.params.iir_depth {
        depth -= 1;
    }

    // Singular extension
    let mut singular_extension = 0;

    let singular_extension_candidate = tt_entry
        .as_ref()
        .filter(|entry| {
            depth >= ctx.params.singular_extension_depth
                && !is_root
                && excluded_mv.is_none()
                && entry.bound != NodeBound::Upper
                && entry.depth >= depth - ctx.params.singular_extension_entry_depth_delta
                && !entry.score.is_mate()
        })
        .and_then(|entry| entry.best_move);

    if let Some(mv) = singular_extension_candidate {
        let mut se_pv = PrincipalVariation::new();
        let tt_score = tt_entry.as_ref().unwrap().score;

        let se_depth = (depth - 1) / 2;
        let se_beta = tt_score - ctx.params.singular_extension_margin * i32::from(depth);

        ctx.stack.get(plies).excluded_mv = Some(mv);
        let value = negamax(game, se_beta - Eval(1), se_beta, se_depth, plies, &mut se_pv, ctx);
        ctx.stack.get(plies).excluded_mv = None;

        if value < se_beta {
            singular_extension = 1;

            if !is_pv
                && value + ctx.params.double_extension_margin < se_beta
                && ctx.stack.get(plies).double_extensions <= ctx.params.double_extension_max
            {
                singular_extension = 2;
                ctx.stack.get(plies).double_extensions += 1;
            }
        } else if !is_pv && !value.is_mate() && value >= beta {
            return value;
        }
    }

    let mut tt_node_bound = NodeBound::Upper;
    let mut best_move = None;
    let mut best_eval = Eval::MIN;

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
            && depth <= ctx.params.futility_prune_depth
            && eval + ctx.params.futility_prune_max_move_value < alpha
        {
            continue;
        }

        if depth < ctx.params.see_prune_depth
            && moves.stage > GenStage::GoodTacticals
            && number_of_legal_moves > 0
            && !is_root
            && !is_pv
            && !best_eval.being_mated()
        {
            let lmr_depth =
                i32::from(depth.saturating_sub(lmr_reduction(depth, number_of_legal_moves)));

            let margin = if mv.is_quiet() {
                ctx.params.see_quiet_margin * lmr_depth * lmr_depth
            } else {
                ctx.params.see_capture_margin * lmr_depth
            };

            if !see(game, mv, margin) {
                continue;
            }
        }

        let lmp_moves = (ctx.params.lmp_move_threshold as usize
            + (depth as usize * depth as usize))
            / (1 + usize::from(!improving));

        if depth <= ctx.params.lmp_depth
            && !is_root
            && !is_pv
            && !in_check
            && number_of_legal_moves >= lmp_moves
            && moves.stage > GenStage::Killer
            && !best_eval.is_mate()
        {
            moves.yield_only_tacticals();
        }

        ctx.stack.get(plies).mv = Some((mv, game.board.piece_guaranteed_at(mv.src())));
        ctx.nnue.push(&game.board, mv);

        game.make_move(mv);
        number_of_legal_moves += 1;

        let extension = if Some(mv) == singular_extension_candidate {
            singular_extension
        } else {
            0
        };

        let move_score = if number_of_legal_moves == 1 {
            -negamax(
                game,
                -beta,
                -alpha,
                depth.saturating_add(extension) - 1,
                plies + 1,
                &mut node_pv,
                ctx,
            )
        } else {
            let reduction = if depth >= ctx.params.lmr_depth
                && number_of_legal_moves >= ctx.params.lmr_move_threshold
            {
                let mut reduction = DepthReduction(lmr_reduction(depth, number_of_legal_moves));

                reduction.reduce_less_if(in_check);

                reduction.reduce_more_if(!is_pv);

                reduction.value()
            } else {
                1
            };

            // We already found a good move (i.e. we raised alpha).
            // Now, we just need to prove that the other moves are worse.
            // We search them with a reduced window to prove that they are at least worse.
            let mut pvs_score = -negamax(
                game,
                -alpha - Eval(1),
                -alpha,
                depth.saturating_add(extension).saturating_sub(reduction),
                plies + 1,
                &mut node_pv,
                ctx,
            );

            // If we raised alpha, but we were searching with reduced depth, we probably want to double
            // check we didn't miss something, so search without the reduction.
            if pvs_score > alpha && reduction > 1 {
                pvs_score = -negamax(
                    game,
                    -alpha - Eval(1),
                    -alpha,
                    depth.saturating_add(extension) - 1,
                    plies + 1,
                    &mut node_pv,
                    ctx,
                );
            }

            // If searching at full depth STILL raised alpha, re-search with normal alpha/beta
            // bounds.
            if pvs_score > alpha && pvs_score < beta {
                -negamax(
                    game,
                    -beta,
                    -alpha,
                    depth.saturating_add(extension) - 1,
                    plies + 1,
                    &mut node_pv,
                    ctx,
                )
            } else {
                pvs_score
            }
        };

        game.undo_move();
        ctx.nnue.pop();

        if ctx.time_control.stopped() {
            return Eval::MIN;
        }

        if move_score > best_eval {
            best_move = Some(mv);
            best_eval = move_score;

            // Cutoff: This move is so good that our opponent won't let it be played.
            if move_score >= beta {
                tt_node_bound = NodeBound::Lower;
                break;
            }

            if move_score > alpha {
                alpha = move_score;
                tt_node_bound = NodeBound::Exact;
                pv.push(mv, &node_pv);
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

        return if game.is_king_in_check() {
            Eval::mated_in(plies)
        } else {
            Eval::DRAW
        };
    }

    if excluded_mv.is_none() {
        if tt_node_bound == NodeBound::Lower {
            let mv = best_move.unwrap();

            ctx.tables
                .capture_history
                .update(mv, game, depth, &captures_tried);

            // 'Killers': if a move was so good that it caused a beta cutoff,
            // but it wasn't a capture, we remember it so that we can try it
            // before other quiet moves.
            if !mv.is_capture() {
                ctx.tables.killer_moves.set(plies, mv);

                if let Some(last_ply) = ctx.stack.get_prev(plies, 1)
                    && let Some((last_move, last_moved)) = last_ply.mv
                {
                    ctx.tables.conthist.update(
                        game,
                        last_moved,
                        last_move,
                        mv,
                        depth,
                        &quiets_tried,
                    );
                }

                if let Some(ply_2) = ctx.stack.get_prev(plies, 2)
                    && let Some((last_move, last_moved)) = ply_2.mv
                {
                    ctx.tables.conthist.update(
                        game,
                        last_moved,
                        last_move,
                        mv,
                        depth,
                        &quiets_tried,
                    );
                }

                ctx.tables
                    .quiet_history
                    .update(game, mv, depth, &quiets_tried);
            }
        }

        ctx.tt
            .insert(game.zobrist, tt_node_bound, best_move, best_eval, eval, depth, plies);
    }

    best_eval
}
