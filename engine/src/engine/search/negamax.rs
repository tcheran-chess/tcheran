use std::cmp::max;

use super::{MAX_SEARCH_DEPTH, SearchContext, params};
use crate::{
    chess::{game::Game, moves::Move},
    engine::{
        eval,
        eval::Eval,
        search::{
            move_picker::MovePicker, principal_variation::PrincipalVariation,
            quiescence::quiescence, tables::lmr_table::lmr_reduction,
        },
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
) -> Result<Eval, ()> {
    let is_root = plies == 0;
    let is_pv = alpha != beta - Eval(1);

    // Check periodically to see if we're out of time. If we are, we shouldn't continue the search
    // so we return Err to signal to the caller that the search did not complete.
    if ctx.time_control.should_stop(ctx.nodes_visited) {
        return Err(());
    }

    ctx.max_depth_reached = ctx.max_depth_reached.max(plies);

    if !is_root && game.is_draw() {
        return Ok(Eval::DRAW);
    }

    // Check extension: If we're about to finish searching, but we are in check, we
    // should keep going.
    let in_check = game.is_king_in_check();
    if in_check {
        depth += 1;
    }

    if depth == 0 || plies == MAX_SEARCH_DEPTH {
        return quiescence(game, alpha, beta, plies, ctx);
    }

    if !is_root {
        ctx.nodes_visited += 1;
    }

    let mut previous_best_move: Option<Move> = None;

    let tt_entry = ctx.tt.get(game.zobrist, plies);
    if let Some(ref tt_entry) = tt_entry {
        if !is_root && !is_pv && tt_entry.depth >= depth {
            let tt_score = tt_entry.score;

            match tt_entry.bound {
                NodeBound::Exact => return Ok(tt_score),
                NodeBound::Upper if tt_score <= alpha => return Ok(tt_score),
                NodeBound::Lower if tt_score >= beta => return Ok(tt_score),
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
            ctx.tbhits += 1;

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
                ctx.tt.insert(
                    game.zobrist,
                    tb_bound,
                    None,
                    score,
                    Eval::NONE,
                    depth,
                    plies,
                );

                return Ok(score);
            }

            if is_pv && tb_bound == NodeBound::Lower {
                alpha = alpha.max(score);
            }
        }
    }

    let eval = match tt_entry {
        Some(ref e) if e.eval != Eval::NONE => e.eval,
        _ => {
            let e = eval::eval(game);

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
            return Ok(beta);
        }

        // Null move pruning
        if depth >= params::NULL_MOVE_PRUNING_DEPTH_LIMIT
            && eval >= beta
            // Don't let a player play a null move in response to a null move
            && game.history.last().is_none_or(|m| m.mv.is_some())
        {
            game.make_null_move();

            let null_score = -negamax(
                game,
                -beta,
                -beta + Eval(1),
                depth - 1 - params::NULL_MOVE_PRUNING_DEPTH_REDUCTION,
                plies + 1,
                &mut PrincipalVariation::new(),
                ctx,
            )?;

            game.undo_null_move();

            if null_score >= beta {
                return Ok(null_score);
            }
        }
    }

    let mut tt_node_bound = NodeBound::Upper;
    let mut best_move = None;
    let mut best_eval = Eval::MIN;

    let mut moves = MovePicker::new(previous_best_move);
    let mut number_of_legal_moves = 0;
    let mut node_pv = PrincipalVariation::new();

    while let Some(mv) = moves.next(game, ctx, plies) {
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

        game.make_move(mv);
        number_of_legal_moves += 1;

        let move_score = if number_of_legal_moves == 1 {
            -negamax(game, -beta, -alpha, depth - 1, plies + 1, &mut node_pv, ctx)?
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
            let pvs_score = -negamax(
                game,
                -alpha - Eval(1),
                -alpha,
                depth.saturating_sub(reduction),
                plies + 1,
                &mut node_pv,
                ctx,
            )?;

            // Turns out the move we just searched could be better than our current PV, so we re-search
            // with the normal alpha/beta bounds.
            if pvs_score > alpha && pvs_score < beta {
                -negamax(game, -beta, -alpha, depth - 1, plies + 1, &mut node_pv, ctx)?
            } else {
                pvs_score
            }
        };

        game.undo_move();

        if move_score > best_eval {
            best_move = Some(mv);
            best_eval = move_score;
        }

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

    if number_of_legal_moves == 0 {
        return Ok(if game.is_king_in_check() {
            Eval::mated_in(plies)
        } else {
            Eval::DRAW
        });
    }

    if tt_node_bound == NodeBound::Lower {
        let mv = best_move.unwrap();

        // 'Killers': if a move was so good that it caused a beta cutoff,
        // but it wasn't a capture, we remember it so that we can try it
        // before other quiet moves.
        if !mv.is_capture() {
            ctx.killer_moves.try_push(plies, mv);

            if let Some(previous_move) = game.history.last().and_then(|h| h.mv) {
                ctx.countermove_table.set(game.player, previous_move, mv);
            }

            ctx.history_table.add_bonus_for(game.player, mv, depth);
        }
    }

    ctx.tt.insert(
        game.zobrist,
        tt_node_bound,
        best_move,
        best_eval,
        eval,
        depth,
        plies,
    );

    Ok(best_eval)
}
