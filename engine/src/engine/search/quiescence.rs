use super::{MAX_SEARCH_DEPTH, SearchContext};
use crate::{
    chess::game::Game,
    engine::{eval, eval::Eval, search::move_picker::MovePicker},
};

pub fn quiescence(
    game: &mut Game,
    mut alpha: Eval,
    beta: Eval,
    plies: u8,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    // Check periodically to see if we're out of time.
    ctx.time_control.update(ctx.nodes_visited);
    if ctx.time_control.stopped() {
        return Eval::MIN;
    }

    ctx.max_depth_reached = ctx.max_depth_reached.max(plies);
    ctx.nodes_visited += 1;

    if plies == MAX_SEARCH_DEPTH {
        return eval::eval(ctx.nnue, game.player);
    }

    if game.is_draw() {
        return Eval::DRAW;
    }

    let eval = eval::eval(ctx.nnue, game.player);

    if eval >= beta {
        return eval;
    }

    if eval > alpha {
        alpha = eval;
    }

    let mut best_eval = eval;

    let mut moves = MovePicker::new_loud();
    while let Some(mv) = moves.next(game, ctx, plies) {
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
        }

        // Cutoff: This move is so good that our opponent won't let it be played.
        if move_score >= beta {
            break;
        }

        if move_score > alpha {
            alpha = move_score;
        }
    }

    best_eval
}
