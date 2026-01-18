use super::{MAX_SEARCH_DEPTH, SearchContext};
use crate::{
    chess::game::Game,
    engine::{eval, eval::Eval, search::move_picker::MovePicker, transposition_table::NodeBound},
};

pub fn quiescence(
    game: &mut Game,
    mut alpha: Eval,
    beta: Eval,
    plies: u8,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    // Check periodically to see if we're out of time.
    ctx.time_control.update(ctx.nodes.get(), ctx.root_depth);
    if ctx.time_control.stopped() {
        return Eval::MIN;
    }

    let is_pv = alpha != beta - Eval(1);

    ctx.max_depth_reached = ctx.max_depth_reached.max(plies);
    ctx.nodes.incr();

    if game.is_draw(plies) {
        return Eval::DRAW;
    }

    let in_check = game.is_king_in_check();

    if plies == MAX_SEARCH_DEPTH {
        return if in_check {
            Eval::DRAW
        } else {
            eval::eval(ctx.nnue, game)
        };
    }

    let tt_entry = ctx.tt.get(game.hash, plies);
    let mut previous_best_move = None;

    if let Some(ref tt_entry) = tt_entry {
        if !is_pv {
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

    let mut node_bound = NodeBound::Upper;

    let raw_eval = if in_check {
        Eval::MIN
    } else if let Some(tt_entry) = tt_entry {
        if tt_entry.eval == Eval::NONE {
            eval::eval(ctx.nnue, game)
        } else {
            tt_entry.eval
        }
    } else {
        let e = eval::eval(ctx.nnue, game);

        ctx.tt
            .insert(game.hash, NodeBound::None, None, Eval::NONE, e, 0, plies);

        e
    };

    let eval = if in_check {
        Eval::MIN
    } else if raw_eval == Eval::NONE {
        Eval::NONE
    } else {
        (raw_eval + ctx.tables.corrhist.get(game)).clamp_to_non_mate()
    };

    if eval >= beta {
        return if !eval.is_mate() && !beta.is_mate() {
            (eval + beta) / 2
        } else {
            eval
        };
    }

    if eval > alpha {
        alpha = eval;
        node_bound = NodeBound::Exact;
    }

    let mut best_eval = eval;
    let mut best_move = None;
    let mut moves_tried = 0;

    let mut moves = MovePicker::new_loud(previous_best_move, !in_check);
    while let Some(mv) = moves.next(game, ctx.tables, ctx.stack, plies) {
        moves_tried += 1;

        // As long as we've found a move that gets us out of mate, we can stop looking at other quiets
        if mv.is_quiet() && !best_eval.being_mated() {
            moves.yield_only_tacticals();
            continue;
        }

        ctx.tt.prefetch(game.approx_zobrist_after(mv));

        ctx.nnue.push(&game.board, mv);
        game.make_move(mv);

        let move_score = -quiescence(game, -beta, -alpha, plies + 1, ctx);

        game.undo_move();
        ctx.nnue.pop();

        if ctx.time_control.stopped() {
            return Eval::MIN;
        }

        if move_score > best_eval {
            best_eval = move_score;

            // Cutoff: This move is so good that our opponent won't let it be played.
            if move_score >= beta {
                node_bound = NodeBound::Lower;
                break;
            }

            if move_score > alpha {
                best_move = Some(mv);
                node_bound = NodeBound::Exact;
                alpha = move_score;
            }
        }
    }

    if in_check && moves_tried == 0 {
        return Eval::mated_in(plies);
    }

    ctx.tt
        .insert(game.hash, node_bound, best_move, best_eval, raw_eval, 0, plies);

    best_eval
}
