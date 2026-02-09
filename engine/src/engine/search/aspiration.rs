use crate::{
    chess::game::Game,
    engine::{
        eval::Eval,
        params::*,
        search::{SearchContext, negamax, principal_variation::PrincipalVariation, types::Depth},
    },
};

struct Window {
    alpha: Eval,
    beta: Eval,

    width: Eval,
}

fn clamp_alpha(eval: Eval) -> Eval {
    std::cmp::max(Eval::MIN, eval)
}

fn clamp_beta(eval: Eval) -> Eval {
    std::cmp::min(Eval::MAX, eval)
}

impl Window {
    pub fn no_window() -> Self {
        Self {
            alpha: Eval::MIN,
            beta: Eval::MAX,

            width: Eval(0),
        }
    }

    pub fn around(eval: Eval, width: Eval) -> Self {
        Self {
            alpha: clamp_alpha(eval - width),
            beta: clamp_beta(eval + width),

            width,
        }
    }

    pub fn is_in_use(&self) -> bool {
        self.width.0 > 0
    }

    pub fn widen_down(&mut self, eval: Eval) {
        self.beta = (self.alpha + self.beta) / 2;
        self.alpha = clamp_alpha(eval - self.width);
        self.increase_window_widening_rate();
    }

    pub fn widen_up(&mut self, eval: Eval) {
        self.beta = clamp_beta(eval + self.width);
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
        Window::no_window()
    } else {
        Window::around(
            eval.expect("Aspiration search should have an evaluation after it reaches min depth"),
            Eval(aspiration_window_size()),
        )
    };

    let mut reduction = 0;

    loop {
        let eval =
            negamax::negamax(game, window.alpha, window.beta, depth - reduction, 0, false, pv, ctx);

        if ctx.time_control.stopped() {
            return Eval::MIN;
        }

        if window.is_in_use() && eval <= window.alpha {
            window.widen_down(eval);
            reduction = 0;
        } else if window.is_in_use() && eval >= window.beta {
            window.widen_up(eval);
            reduction += 1;
        } else {
            return eval;
        }
    }
}
