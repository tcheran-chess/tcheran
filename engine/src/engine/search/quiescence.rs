use super::{MAX_SEARCH_DEPTH, SearchContext};
use crate::{
    chess::game::Game,
    engine::{
        eval::{self, Eval},
        search::{
            move_picker::{GenStage, MovePicker},
            types::{Depth, ScoreWindow},
        },
        transposition_table::NodeBound,
    },
};

pub fn quiescence(
    game: &mut Game,
    mut s: ScoreWindow,
    plies: u8,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    // Check periodically to see if we're out of time.
    ctx.time_control.update(ctx.nodes.get(), ctx.root_depth);
    if ctx.time_control.stopped() {
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
                NodeBound::Upper if tt_score <= s.alpha => return tt_score,
                NodeBound::Lower if tt_score >= s.beta => return tt_score,
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
            .insert(game.hash, NodeBound::None, None, Eval::NONE, e, Depth::ZERO, plies);

        e
    };

    let eval = if in_check {
        Eval::MIN
    } else {
        (raw_eval + ctx.tables.corrhist.get(game)).clamp_to_non_mate()
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
        node_bound = NodeBound::Exact;
    }

    let mut best_score = eval;
    let mut best_move = None;
    let mut moves_tried = 0;

    let mut moves = MovePicker::new(previous_best_move);

    if !in_check {
        moves.skip_quiets();
    }

    while let Some(mv) = moves.next(game, ctx.tables, ctx.stack, plies) {
        moves_tried += 1;

        if moves.stage >= GenStage::BadTacticals {
            break;
        }

        // As long as we've found a move that gets us out of mate, we can stop looking at other quiets
        if mv.is_quiet() && !best_score.is_loss() {
            moves.skip_quiets();
            continue;
        }

        ctx.tt.prefetch(game.approx_zobrist_after(mv));

        ctx.stack.get(plies).mv = Some((mv, game.board.piece_guaranteed_at(mv.from())));

        game.make_move_nnue(mv, ctx.nnue.next_changes());

        let move_score = -quiescence(game, -s, plies + 1, ctx);

        game.undo_move();
        ctx.nnue.pop();

        if ctx.time_control.stopped() {
            return Eval::MIN;
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

    if in_check && moves_tried == 0 {
        return Eval::mated_in(plies);
    }

    ctx.tt
        .insert(game.hash, node_bound, best_move, best_score, raw_eval, Depth::new(0), plies);

    best_score
}
