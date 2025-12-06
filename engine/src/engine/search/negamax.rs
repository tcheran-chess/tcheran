use std::cmp::max;

use super::{MAX_SEARCH_DEPTH, SearchContext, params};
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
    #[expect(unused, reason = "No LMR conditions yet")]
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
                eval::eval(&mut ctx.nnue, game.player)
            };
        }
    }

    let mut previous_best_move: Option<Move> = None;

    let tt_entry = ctx.tt.get(game.zobrist, plies);
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
    if !is_root && tb_cardinality > 0 {
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

    let eval = match tt_entry {
        Some(ref e) if e.eval != Eval::NONE => e.eval,
        _ => {
            let e = eval::eval(&mut ctx.nnue, game.player);

            ctx.tt
                .insert(game.zobrist, NodeBound::None, None, Eval::NONE, e, 0, plies);

            e
        }
    };

    if !is_root && !is_pv && !in_check {
        // Reverse futility pruning
        if depth <= params::REVERSE_FUTILITY_PRUNE_DEPTH
            && eval - params::REVERSE_FUTILITY_PRUNE_MARGIN_PER_PLY * i32::from(depth) > beta
        {
            return beta;
        }

        // Null move pruning
        if eval >= beta
            // Don't let a player play a null move in response to a null move
            && game.history.last().is_none_or(|m| m.mv.is_some())
            && !game.zugzwang_likely()
        {
            let reduction = params::NULL_MOVE_PRUNING_BASE_REDUCTION
                + depth / params::NULL_MOVE_PRUNING_REDUCTION_FACTOR;

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

    let mut tt_node_bound = NodeBound::Upper;
    let mut best_move = None;
    let mut best_eval = Eval::MIN;

    let mut moves = MovePicker::new(previous_best_move);
    let mut number_of_legal_moves = 0;
    let mut node_pv = PrincipalVariation::new();

    let mut captures_tried = MoveList::new();
    let mut quiets_tried = MoveList::new();

    while let Some(mv) = moves.next(game, ctx.tables, plies) {
        node_pv.clear();

        // Futility pruning
        if number_of_legal_moves > 0
            && !is_pv
            && !mv.is_capture()
            && !in_check
            && depth <= params::FUTILITY_PRUNE_DEPTH
            && eval + params::FUTILITY_PRUNE_MAX_MOVE_VALUE < alpha
        {
            continue;
        }

        if depth < params::SEE_PRUNE_DEPTH
            && moves.stage > GenStage::GoodTacticals
            && number_of_legal_moves > 0
            && !is_root
            && !is_pv
            && !best_eval.being_mated()
        {
            let lmr_depth =
                i32::from(depth.saturating_sub(lmr_reduction(depth, number_of_legal_moves)));

            let margin = if mv.is_quiet() {
                params::SEE_QUIET_MARGIN * lmr_depth * lmr_depth
            } else {
                params::SEE_CAPTURE_MARGIN * lmr_depth
            };

            if !see(game, mv, margin) {
                continue;
            }
        }

        let lmp_moves = params::LMP_MOVE_THRESHOLD as usize + (depth as usize * depth as usize);

        if depth <= params::LMP_DEPTH
            && !is_root
            && !is_pv
            && !in_check
            && number_of_legal_moves >= lmp_moves
            && moves.stage >= GenStage::CounterMove
            && !best_eval.is_mate()
        {
            moves.yield_only_tacticals();
        }

        ctx.nnue.push(&game.board, mv);
        game.make_move(mv);
        number_of_legal_moves += 1;

        let move_score = if number_of_legal_moves == 1 {
            -negamax(game, -beta, -alpha, depth - 1, plies + 1, &mut node_pv, ctx)
        } else {
            let reduction = if depth >= params::LMR_DEPTH
                && number_of_legal_moves >= params::LMR_MOVE_THRESHOLD
            {
                let mut reduction = DepthReduction(lmr_reduction(depth, number_of_legal_moves));

                reduction.reduce_less_if(in_check);

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
                depth.saturating_sub(reduction),
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
                    depth - 1,
                    plies + 1,
                    &mut node_pv,
                    ctx,
                );
            }

            // If searching at full depth STILL raised alpha, re-search with normal alpha/beta
            // bounds.
            if pvs_score > alpha && pvs_score < beta {
                -negamax(game, -beta, -alpha, depth - 1, plies + 1, &mut node_pv, ctx)
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
        return if game.is_king_in_check() {
            Eval::mated_in(plies)
        } else {
            Eval::DRAW
        };
    }

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

            if let Some(previous_move) = game.history.last().and_then(|h| h.mv) {
                ctx.tables.countermoves.set(game.player, previous_move, mv);
            }

            ctx.tables
                .quiet_history
                .update(game, mv, depth, &quiets_tried);
        }
    }

    ctx.tt
        .insert(game.zobrist, tt_node_bound, best_move, best_eval, eval, depth, plies);

    best_eval
}
