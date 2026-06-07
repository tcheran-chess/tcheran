pub mod arrayvec;
pub mod bitboard;
pub mod board;
pub mod direction;
pub mod game;
pub mod moves;
pub mod notations;
pub mod perft;
pub mod piece;
pub mod player;
pub mod rays;
pub mod square;
pub mod zobrist;

pub fn init() {
    moves::init();
    rays::init();
}
