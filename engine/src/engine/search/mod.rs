mod aspiration;
mod iterative_deepening;
pub mod move_picker;
mod negamax;
mod principal_variation;
mod quiescence;
mod search;
pub mod tables;
pub mod time_control;

pub use search::*;

pub fn init() {
    tables::init();
}
