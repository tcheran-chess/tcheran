mod attacks;
mod movegen;
mod tables;
mod types;

pub use attacks::{
    all_attackers_of, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks,
};
pub use movegen::{generate_legal_moves, generate_quiets, generate_tacticals};
pub use types::{Flags, MAX_LEGAL_MOVES, Move, MoveList, MoveListExt};

pub fn init() {
    tables::init();
}
