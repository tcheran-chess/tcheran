mod bitboard;
mod board;
mod direction;
mod game;
pub mod moves;
pub mod notations;
pub mod perft;
mod piece;
mod player;
pub mod rays;
mod square;
pub mod zobrist;

pub use bitboard::{Bitboard, bitboards};
pub use board::Board;
pub use direction::Direction;
pub use game::{CastleRights, CastleRightsSide, Game, MoveObserver};
pub use moves::Move;
pub use piece::{Piece, PieceKind, PromotionPieceKind};
pub use player::Player;
pub use square::{File, Rank, Square, ranks, squares};

pub fn init() {
    moves::init();
    rays::init();
}
