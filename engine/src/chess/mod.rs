pub mod arrayvec;
pub mod bitboard;
pub mod board;
pub mod direction;
pub mod fen;
pub mod game;
pub mod movegen;
pub mod moves;
pub mod perft;
pub mod piece;
pub mod player;
pub mod san;
pub mod square;
pub mod zobrist;

pub fn init() {
    movegen::init();
}
