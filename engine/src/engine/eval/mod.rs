pub mod nnue;
mod player_eval;
pub mod wdl;
mod white_eval;

pub use player_eval::Eval;
pub use white_eval::WhiteEval;

use crate::{chess::player::Player, engine::eval::nnue::NetworkStack};

pub fn eval(nnue: &mut NetworkStack, player: Player) -> Eval {
    nnue.evaluate(player)
}
