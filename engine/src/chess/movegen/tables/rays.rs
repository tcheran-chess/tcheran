use crate::chess::{
    bitboard::Bitboard,
    direction::Direction,
    movegen::tables::{bishop_attacks, rook_attacks},
    player::Player,
    square::Square,
};

static mut BETWEEN_RAYS: [[Bitboard; Square::N]; Square::N] =
    [[Bitboard::EMPTY; Square::N]; Square::N];

static mut INTERSECTING_RAYS: [[Bitboard; Square::N]; Square::N] =
    [[Bitboard::EMPTY; Square::N]; Square::N];

static mut SKEWERING_RAYS: [[Bitboard; Square::N]; Square::N] =
    [[Bitboard::EMPTY; Square::N]; Square::N];

static mut DIAGONAL_RAYS: [Bitboard; Square::N] = [Bitboard::EMPTY; Square::N];
static mut ANTIDIAGONAL_RAYS: [Bitboard; Square::N] = [Bitboard::EMPTY; Square::N];

pub fn ray_between(from: Square, to: Square) -> Bitboard {
    unsafe { BETWEEN_RAYS[from][to] }
}

pub fn ray_intersecting(from: Square, to: Square) -> Bitboard {
    unsafe { INTERSECTING_RAYS[from][to] }
}

pub fn ray_skewering(from: Square, to: Square) -> Bitboard {
    unsafe { SKEWERING_RAYS[from][to] }
}

pub fn ray_diagonal(square: Square) -> Bitboard {
    unsafe { DIAGONAL_RAYS[square] }
}

pub fn ray_antidiagonal(square: Square) -> Bitboard {
    unsafe { ANTIDIAGONAL_RAYS[square] }
}

pub fn ray_relative_diagonal(square: Square, player: Player) -> Bitboard {
    match player {
        Player::White => ray_diagonal(square),
        Player::Black => ray_antidiagonal(square),
    }
}

pub fn ray_relative_antidiagonal(square: Square, player: Player) -> Bitboard {
    match player {
        Player::White => ray_antidiagonal(square),
        Player::Black => ray_diagonal(square),
    }
}

fn generate_ray_between(from: Square, to: Square) -> Bitboard {
    let from_mask = from.bb();
    let to_mask = to.bb();

    let all_rook_attacks = rook_attacks(from, Bitboard::EMPTY);
    let all_bishop_attacks = bishop_attacks(from, Bitboard::EMPTY);

    if all_rook_attacks.contains(to) {
        return rook_attacks(from, to_mask) & rook_attacks(to, from_mask);
    }

    if all_bishop_attacks.contains(to) {
        return bishop_attacks(from, to_mask) & bishop_attacks(to, from_mask);
    }

    Bitboard::EMPTY
}

fn generate_ray_intersecting(from: Square, to: Square) -> Bitboard {
    let from_mask = from.bb();
    let to_mask = to.bb();

    let all_rook_attacks = rook_attacks(from, Bitboard::EMPTY);
    let all_bishop_attacks = bishop_attacks(from, Bitboard::EMPTY);

    if all_rook_attacks.contains(to) {
        return (from_mask | rook_attacks(from, Bitboard::EMPTY))
            & (to_mask | rook_attacks(to, Bitboard::EMPTY));
    }

    if all_bishop_attacks.contains(to) {
        return (from_mask | bishop_attacks(from, Bitboard::EMPTY))
            & (to_mask | bishop_attacks(to, Bitboard::EMPTY));
    }

    Bitboard::EMPTY
}

fn generate_ray_skewering(from: Square, to: Square) -> Bitboard {
    let from_mask = from.bb();
    let to_mask = to.bb();

    let all_rook_attacks = rook_attacks(from, Bitboard::EMPTY);
    let all_bishop_attacks = bishop_attacks(from, Bitboard::EMPTY);

    if all_rook_attacks.contains(to) {
        return rook_attacks(from, Bitboard::EMPTY) & (to_mask | rook_attacks(to, from_mask));
    }

    if all_bishop_attacks.contains(to) {
        return bishop_attacks(from, Bitboard::EMPTY) & (to_mask | bishop_attacks(to, from_mask));
    }

    Bitboard::EMPTY
}

fn generate_diagonal_ray(square: Square) -> Bitboard {
    find_diagonal(square, Direction::SouthWest, Direction::NorthEast)
}

fn generate_antidiagonal_ray(square: Square) -> Bitboard {
    find_diagonal(square, Direction::NorthWest, Direction::SouthEast)
}

fn find_diagonal(square: Square, go_left: Direction, go_right: Direction) -> Bitboard {
    let mut left_bound = square.bb();
    let mut right_bound = square.bb();

    while let new_left_bound = left_bound.in_direction(go_left)
        && new_left_bound != Bitboard::EMPTY
    {
        left_bound = new_left_bound;
    }

    while let new_right_bound = right_bound.in_direction(go_right)
        && new_right_bound != Bitboard::EMPTY
    {
        right_bound = new_right_bound;
    }

    unsafe { BETWEEN_RAYS[left_bound.single()][right_bound.single()] | left_bound | right_bound }
}

pub fn init() {
    for s1 in Bitboard::FULL {
        for s2 in Bitboard::FULL {
            if s1 == s2 {
                continue;
            }

            unsafe {
                BETWEEN_RAYS[s1][s2] = generate_ray_between(s1, s2);
                INTERSECTING_RAYS[s1][s2] = generate_ray_intersecting(s1, s2);
                SKEWERING_RAYS[s1][s2] = generate_ray_skewering(s1, s2);
            }
        }
    }

    for s in Bitboard::FULL {
        unsafe {
            DIAGONAL_RAYS[s] = generate_diagonal_ray(s);
            ANTIDIAGONAL_RAYS[s] = generate_antidiagonal_ray(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::square::squares::all::*;

    #[test]
    fn test_between_on_rank() {
        crate::init();
        assert_eq!(ray_between(B4, G4), C4 | D4 | E4 | F4);
    }

    #[test]
    fn test_between_on_rank_for_full_rank() {
        crate::init();
        assert_eq!(ray_between(A1, H1), B1 | C1 | D1 | E1 | F1 | G1);
    }

    #[test]
    fn test_between_on_file() {
        crate::init();
        assert_eq!(ray_between(C2, C7), C3 | C4 | C5 | C6);
    }

    #[test]
    fn test_between_on_file_for_full_file() {
        crate::init();
        assert_eq!(ray_between(H1, H8), H2 | H3 | H4 | H5 | H6 | H7);
    }

    #[test]
    fn test_between_on_diagonal() {
        crate::init();
        assert_eq!(ray_between(A1, H8), B2 | C3 | D4 | E5 | F6 | G7);
    }

    #[test]
    fn test_between_on_diagonal_descending() {
        crate::init();
        assert_eq!(ray_between(A8, H1), B7 | C6 | D5 | E4 | F3 | G2);
    }
}
