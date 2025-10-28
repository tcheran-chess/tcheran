mod attacks;
mod between;
mod king;
mod knights;
mod magics;
mod pawns;

pub use between::between;
pub use king::king_attacks;
pub use knights::knight_attacks;
pub use magics::{bishop_attacks, rook_attacks};
pub use pawns::pawn_attacks;

pub fn init() {
    magics::init();

    knights::init();
    king::init();

    between::init();
}
