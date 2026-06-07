mod king;
mod knights;
mod magics;
mod pawns;

pub use king::lookup_king_attacks;
pub use knights::lookup_knight_attacks;
pub use magics::{lookup_bishop_attacks, lookup_rook_attacks};
pub use pawns::lookup_pawn_attacks;

pub fn init() {
    magics::init();

    knights::init();
    king::init();
    pawns::init();
}
