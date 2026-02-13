#![expect(clippy::unreadable_literal, reason = "AS and BS are taken directly from WDL_model")]

use crate::{chess::board::Board, engine::eval::Eval};

const AS: [f64; 4] = [-33.91949376, 78.27279538, -140.19845918, 296.32468200];
const BS: [f64; 4] = [-0.13206392, 10.40258210, 18.27140769, 37.39334351];

#[expect(unused, reason = "Here for reference")]
const NORMALIZE_TO_PAWN_VALUE: i32 = 200;

fn material(board: &Board) -> i32 {
    i32::from(board.all_pawns().count())
        + 3 * i32::from(board.all_knights().count())
        + 3 * i32::from(board.all_bishops().count())
        + 5 * i32::from(board.all_rooks().count())
        + 9 * i32::from(board.all_queens().count())
}

fn compute_polynomial(params: &[f64; 4], m: f64) -> f64 {
    let [p0, p1, p2, p3] = params;
    ((p0 * m + p1) * m + p2) * m + p3
}

fn compute_win_rate(a: f64, b: f64, eval: f64) -> f64 {
    1.0 / (1.0 + (-(eval - a) / b).exp())
}

fn wdl_params(board: &Board) -> (f64, f64) {
    let material = material(board);
    let m = f64::from(material.clamp(16, 64)) / 58.0;

    (compute_polynomial(&AS, m), compute_polynomial(&BS, m))
}

#[expect(clippy::cast_possible_truncation, reason = "Approximate calculation")]
pub fn normalize(eval: Eval, board: &Board) -> Eval {
    // Don't normalize eval scores in datagen
    if cfg!(feature = "datagen") {
        return eval;
    }

    if eval.is_mate() {
        return eval;
    }

    let (a, _) = wdl_params(board);
    let eval = f64::from(eval.0);

    Eval((100.0 * eval / a).round() as i32)
}

pub fn wdl(eval: Eval, board: &Board) -> WdlProbabilities {
    if eval.mating() {
        return WdlProbabilities::DEFINITELY_WINNING;
    }

    if eval.being_mated() {
        return WdlProbabilities::DEFINITELY_LOSING;
    }

    let (a, b) = wdl_params(board);
    let eval = f64::from(eval.0);

    let win = compute_win_rate(a, b, eval);
    let loss = compute_win_rate(a, b, -eval);
    let draw = 1.0 - win - loss;

    WdlProbabilities { win, draw, loss }
}

#[derive(Debug)]
pub struct WdlProbabilities {
    pub win: f64,
    pub draw: f64,
    pub loss: f64,
}

impl WdlProbabilities {
    pub const DEFINITELY_WINNING: Self = Self {
        win: 1.0,
        draw: 0.0,
        loss: 0.0,
    };

    pub const DEFINITELY_LOSING: Self = Self {
        win: 0.0,
        draw: 0.0,
        loss: 1.0,
    };
}
