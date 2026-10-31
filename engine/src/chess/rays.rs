use crate::chess::{
    direction::Direction,
    moves::{bishop_attacks, rook_attacks},
    prelude::*,
};

static BETWEEN_RAYS: [[Bitboard; Square::N]; Square::N] = const {
    let mut rays = [[Bitboard::EMPTY; Square::N]; Square::N];

    let mut i = 0;
    while i < Square::N {
        let mut j = 0;
        while j < Square::N {
            rays[i][j] =
                generate_ray_between(Square::from_index(i as u8), Square::from_index(j as u8));
            j += 1;
        }
        i += 1;
    }

    rays
};

static INTERSECTING_RAYS: [[Bitboard; Square::N]; Square::N] = const {
    let mut rays = [[Bitboard::EMPTY; Square::N]; Square::N];

    let mut i = 0;
    while i < Square::N {
        let mut j = 0;
        while j < Square::N {
            rays[i][j] =
                generate_ray_intersecting(Square::from_index(i as u8), Square::from_index(j as u8));
            j += 1;
        }
        i += 1;
    }

    rays
};

static SKEWERING_RAYS: [[Bitboard; Square::N]; Square::N] = const {
    let mut rays = [[Bitboard::EMPTY; Square::N]; Square::N];

    let mut i = 0;
    while i < Square::N {
        let mut j = 0;
        while j < Square::N {
            rays[i][j] =
                generate_ray_skewering(Square::from_index(i as u8), Square::from_index(j as u8));
            j += 1;
        }
        i += 1;
    }

    rays
};

static DIAGONAL_RAYS: [Bitboard; Square::N] = const {
    let mut rays = [Bitboard::EMPTY; Square::N];

    let mut square = 0;
    while square < Square::N {
        rays[square] = generate_diagonal_ray(Square::from_index(square as u8));
        square += 1;
    }

    rays
};

static ANTIDIAGONAL_RAYS: [Bitboard; Square::N] = const {
    let mut rays = [Bitboard::EMPTY; Square::N];

    let mut square = 0;
    while square < Square::N {
        rays[square] = generate_antidiagonal_ray(Square::from_index(square as u8));
        square += 1;
    }

    rays
};

pub fn ray_between(from: Square, to: Square) -> Bitboard {
    BETWEEN_RAYS[from][to]
}

pub fn ray_intersecting(from: Square, to: Square) -> Bitboard {
    INTERSECTING_RAYS[from][to]
}

pub fn ray_skewering(from: Square, to: Square) -> Bitboard {
    SKEWERING_RAYS[from][to]
}

pub fn ray_diagonal(square: Square) -> Bitboard {
    DIAGONAL_RAYS[square]
}

pub fn ray_antidiagonal(square: Square) -> Bitboard {
    ANTIDIAGONAL_RAYS[square]
}

pub fn ray_relative_diagonal(square: Square, player: Player) -> Bitboard {
    match player {
        White => ray_diagonal(square),
        Black => ray_antidiagonal(square),
    }
}

pub fn ray_relative_antidiagonal(square: Square, player: Player) -> Bitboard {
    match player {
        White => ray_antidiagonal(square),
        Black => ray_diagonal(square),
    }
}

const fn generate_ray_between(from: Square, to: Square) -> Bitboard {
    let from_mask = from.bb();
    let to_mask = to.bb();

    let all_rook_attacks = rook_attacks(from, Bitboard::EMPTY);
    let all_bishop_attacks = bishop_attacks(from, Bitboard::EMPTY);

    if all_rook_attacks.contains(to) {
        return Bitboard::new(
            rook_attacks(from, to_mask).as_u64() & rook_attacks(to, from_mask).as_u64(),
        );
    }

    if all_bishop_attacks.contains(to) {
        return Bitboard::new(
            bishop_attacks(from, to_mask).as_u64() & bishop_attacks(to, from_mask).as_u64(),
        );
    }

    Bitboard::EMPTY
}

const fn generate_ray_intersecting(from: Square, to: Square) -> Bitboard {
    let from_mask = from.bb();
    let to_mask = to.bb();

    let all_rook_attacks = rook_attacks(from, Bitboard::EMPTY);
    let all_bishop_attacks = bishop_attacks(from, Bitboard::EMPTY);

    if all_rook_attacks.contains(to) {
        return Bitboard::new(
            (from_mask.as_u64() | rook_attacks(from, Bitboard::EMPTY).as_u64())
                & (to_mask.as_u64() | rook_attacks(to, Bitboard::EMPTY).as_u64()),
        );
    }

    if all_bishop_attacks.contains(to) {
        return Bitboard::new(
            (from_mask.as_u64() | bishop_attacks(from, Bitboard::EMPTY).as_u64())
                & (to_mask.as_u64() | bishop_attacks(to, Bitboard::EMPTY).as_u64()),
        );
    }

    Bitboard::EMPTY
}

const fn generate_ray_skewering(from: Square, to: Square) -> Bitboard {
    let from_mask = from.bb();
    let to_mask = to.bb();

    let all_rook_attacks = rook_attacks(from, Bitboard::EMPTY);
    let all_bishop_attacks = bishop_attacks(from, Bitboard::EMPTY);

    if all_rook_attacks.contains(to) {
        return Bitboard::new(
            all_rook_attacks.as_u64() & (to_mask.as_u64() | rook_attacks(to, from_mask).as_u64()),
        );
    }

    if all_bishop_attacks.contains(to) {
        return Bitboard::new(
            all_bishop_attacks.as_u64()
                & (to_mask.as_u64() | bishop_attacks(to, from_mask).as_u64()),
        );
    }

    Bitboard::EMPTY
}

const fn generate_diagonal_ray(square: Square) -> Bitboard {
    find_diagonal(square, Direction::SouthWest, Direction::NorthEast)
}

const fn generate_antidiagonal_ray(square: Square) -> Bitboard {
    find_diagonal(square, Direction::NorthWest, Direction::SouthEast)
}

const fn find_diagonal(square: Square, go_left: Direction, go_right: Direction) -> Bitboard {
    let mut left_bound = square.bb();
    let mut right_bound = square.bb();

    loop {
        let new_left_bound = left_bound.in_direction(go_left);
        if !new_left_bound.any() {
            break;
        }
        left_bound = new_left_bound;
    }

    loop {
        let new_right_bound = right_bound.in_direction(go_right);
        if !new_right_bound.any() {
            break;
        }
        right_bound = new_right_bound;
    }

    let left = left_bound.single().idx() as usize;
    let right = right_bound.single().idx() as usize;
    Bitboard::new(BETWEEN_RAYS[left][right].as_u64() | left_bound.as_u64() | right_bound.as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_between_on_rank() {
        assert_eq!(ray_between(B4, G4), C4 | D4 | E4 | F4);
    }

    #[test]
    fn test_between_on_rank_for_full_rank() {
        assert_eq!(ray_between(A1, H1), B1 | C1 | D1 | E1 | F1 | G1);
    }

    #[test]
    fn test_between_on_file() {
        assert_eq!(ray_between(C2, C7), C3 | C4 | C5 | C6);
    }

    #[test]
    fn test_between_on_file_for_full_file() {
        assert_eq!(ray_between(H1, H8), H2 | H3 | H4 | H5 | H6 | H7);
    }

    #[test]
    fn test_between_on_diagonal() {
        assert_eq!(ray_between(A1, H8), B2 | C3 | D4 | E5 | F6 | G7);
    }

    #[test]
    fn test_between_on_diagonal_descending() {
        assert_eq!(ray_between(A8, H1), B7 | C6 | D5 | E4 | F3 | G2);
    }
}
