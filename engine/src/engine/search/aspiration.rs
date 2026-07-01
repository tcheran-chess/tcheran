use crate::{
    chess::Game,
    engine::{
        eval::Eval,
        params::*,
        search::{
            SearchContext, negamax,
            principal_variation::PrincipalVariation,
            types::{Depth, ScoreWindow},
        },
    },
};

fn clamp_alpha(eval: Eval) -> Eval {
    std::cmp::max(Eval::MIN, eval)
}

fn clamp_beta(eval: Eval) -> Eval {
    std::cmp::min(Eval::MAX, eval)
}

fn increase_width(width: i32) -> i32 {
    width + width / 2
}

pub fn aspiration_search(
    game: &mut Game,
    depth: Depth,
    eval: Option<Eval>,
    pv: &mut PrincipalVariation,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    let mut width = aspiration_window_size();

    let mut window = if depth < aspiration_min_depth() || eval.is_some_and(Eval::is_decisive) {
        ScoreWindow::new(Eval::MIN, Eval::MAX)
    } else {
        let eval =
            eval.expect("Aspiration search should have an evaluation after it reaches min depth");
        ScoreWindow::new(clamp_alpha(eval - width), clamp_beta(eval + width))
    };

    let mut reduction = 0;

    loop {
        let eval = negamax::negamax(game, window, depth - reduction, 0, false, pv, ctx);

        if ctx.stopped() {
            return Eval::MIN;
        }

        if eval <= window.alpha {
            window.beta = (window.alpha + window.beta) / 2;
            window.alpha = clamp_alpha(eval - width);
            width = increase_width(width);
            reduction = 0;
        } else if eval >= window.beta {
            window.beta = clamp_beta(eval + width);
            width = increase_width(width);
            reduction += 1;
        } else {
            return eval;
        }
    }
}
