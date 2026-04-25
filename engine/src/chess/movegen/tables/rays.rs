use crate::chess::{
    bitboard::Bitboard,
    movegen::tables::{bishop_attacks, rook_attacks},
    square::Square,
};

static mut BETWEEN_RAYS: [[Bitboard; Square::N]; Square::N] =
    [[Bitboard::EMPTY; Square::N]; Square::N];

static mut INTERSECTING_RAYS: [[Bitboard; Square::N]; Square::N] =
    [[Bitboard::EMPTY; Square::N]; Square::N];

static mut SKEWERING_RAYS: [[Bitboard; Square::N]; Square::N] =
    [[Bitboard::EMPTY; Square::N]; Square::N];

pub fn ray_between(from: Square, to: Square) -> Bitboard {
    unsafe { BETWEEN_RAYS[from][to] }
}

pub fn ray_intersecting(from: Square, to: Square) -> Bitboard {
    unsafe { INTERSECTING_RAYS[from][to] }
}

pub fn ray_skewering(from: Square, to: Square) -> Bitboard {
    unsafe { SKEWERING_RAYS[from][to] }
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
