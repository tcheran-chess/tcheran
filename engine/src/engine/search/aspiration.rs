use crate::{
    chess::game::Game,
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

struct AspirationWindow {
    s: ScoreWindow,
    width: Eval,
}

fn clamp_alpha(eval: Eval) -> Eval {
    std::cmp::max(Eval::MIN, eval)
}

fn clamp_beta(eval: Eval) -> Eval {
    std::cmp::min(Eval::MAX, eval)
}

impl AspirationWindow {
    pub fn no_window() -> Self {
        Self {
            s: ScoreWindow::new(Eval::MIN, Eval::MAX),
            width: Eval(0),
        }
    }

    pub fn around(eval: Eval, width: Eval) -> Self {
        Self {
            s: ScoreWindow::new(clamp_alpha(eval - width), clamp_beta(eval + width)),
            width,
        }
    }

    pub fn is_in_use(&self) -> bool {
        self.width.0 > 0
    }

    pub fn widen_down(&mut self, eval: Eval) {
        self.s.beta = (self.s.alpha + self.s.beta) / 2;
        self.s.alpha = clamp_alpha(eval - self.width);
        self.increase_window_widening_rate();
    }

    pub fn widen_up(&mut self, eval: Eval) {
        self.s.beta = clamp_beta(eval + self.width);
        self.increase_window_widening_rate();
    }

    fn increase_window_widening_rate(&mut self) {
        self.width = self.width + self.width / 2;
    }
}

pub fn aspiration_search(
    game: &mut Game,
    depth: Depth,
    eval: Option<Eval>,
    pv: &mut PrincipalVariation,
    ctx: &mut SearchContext<'_>,
) -> Eval {
    let mut window = if depth < aspiration_min_depth() || eval.is_some_and(Eval::is_decisive) {
        AspirationWindow::no_window()
    } else {
        AspirationWindow::around(
            eval.expect("Aspiration search should have an evaluation after it reaches min depth"),
            Eval(aspiration_window_size()),
        )
    };

    let mut reduction = 0;

    loop {
        let eval = negamax::negamax(game, window.s, depth - reduction, 0, false, pv, ctx);

        if ctx.time_control.stopped() {
            return Eval::MIN;
        }

        if window.is_in_use() && eval <= window.s.alpha {
            window.widen_down(eval);
            reduction = 0;
        } else if window.is_in_use() && eval >= window.s.beta {
            window.widen_up(eval);
            reduction += 1;
        } else {
            return eval;
        }
    }
}
