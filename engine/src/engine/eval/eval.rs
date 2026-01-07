use crate::{
    chess::game::Game,
    engine::{
        eval::{Eval, nnue::NetworkStack},
        params::{
            material_scale_base, material_scale_divisor, see_bishop_value, see_knight_value,
            see_queen_value, see_rook_value,
        },
    },
};

fn scale_eval(eval: Eval, game: &Game) -> Eval {
    let material = i32::from(game.board.all_knights().count()) * see_knight_value()
        + i32::from(game.board.all_bishops().count()) * see_bishop_value()
        + i32::from(game.board.all_rooks().count()) * see_rook_value()
        + i32::from(game.board.all_queens().count()) * see_queen_value();

    let scale = material_scale_base() + material / material_scale_divisor();

    Eval((eval.0 * scale) / 1024)
}

pub fn eval(nnue: &mut NetworkStack, game: &Game) -> Eval {
    let eval = nnue.evaluate(game);

    #[cfg(not(feature = "datagen"))]
    let eval = scale_eval(eval, game);

    eval
}
