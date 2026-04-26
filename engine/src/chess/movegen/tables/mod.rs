mod attacks;
mod king;
mod knights;
mod magics;
mod pawns;
mod rays;

pub use king::king_attacks;
pub use knights::knight_attacks;
pub use magics::{bishop_attacks, rook_attacks};
pub use pawns::pawn_attacks;
pub use rays::{
    ray_between, ray_intersecting, ray_relative_antidiagonal, ray_relative_diagonal, ray_skewering,
};

pub fn init() {
    magics::init();

    knights::init();
    king::init();
    pawns::init();

    rays::init();
}
