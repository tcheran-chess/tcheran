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

pub mod prelude {
    pub use super::{
        bitboard::Bitboard,
        board::Board,
        game::Game,
        moves::Move,
        piece::{Piece, PieceKind, PieceKind::*, PromotionPieceKind},
        player::{Player, Player::*},
        square::{File, Rank, Square},
        squares::all::*,
    };
}

pub use bitboard::bitboards;
pub use game::{CastleRights, CastleRightsSide, MoveObserver};
pub use square::{ranks, squares};

pub fn init() {
    moves::init();
    rays::init();
}
