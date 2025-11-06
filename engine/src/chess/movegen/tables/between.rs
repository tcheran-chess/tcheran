use crate::chess::{bitboard::Bitboard, direction::Direction, square::Square};

static BETWEEN_TABLE: [[Bitboard; Square::N]; Square::N] = {
    let mut arr = [[Bitboard::EMPTY; Square::N]; Square::N];

    let mut from_bb = Bitboard::FULL;
    while let Some(from) = from_bb.pop_square_inplace() {
        let mut to_bb = Bitboard::FULL;
        while let Some(to) = to_bb.pop_square_inplace() {
            let between_ray = match generate_squares_between(from, to) {
                Some(ray) => ray,
                None => Bitboard::EMPTY,
            };

            arr[from][to] = between_ray;
        }
    }

    arr
};

pub fn between(s1: Square, s2: Square) -> Bitboard {
    unsafe { BETWEEN_TABLE[s1][s2] }
}

// For this function to be const, we need to break the Square and Bitboard
// abstractions and work directly with integers.
// This is because various traits (PartialEq, BitOrAssign, etc.) are not yet const.
const fn generate_squares_between(s1: Square, s2: Square) -> Option<Bitboard> {
    let mut squares = Bitboard::EMPTY.as_u64();

    if s1.idx() == s2.idx() {
        return None;
    }

    // Same rank
    if s1.rank().idx() == s2.rank().idx() {
        let (current_square, end_square) = if s1.file().idx() < s2.file().idx() {
            (s1, s2)
        } else {
            (s2, s1)
        };

        let mut current_square = current_square.bb();
        let end_square = end_square.bb();

        current_square = current_square.east();

        while current_square.as_u64() != end_square.as_u64() {
            squares |= current_square.as_u64();
            current_square = current_square.east();
        }

        return Some(Bitboard::new(squares));
    }

    // Same file
    if s1.file().idx() == s2.file().idx() {
        let (current_square, end_square) = if s1.rank().idx() < s2.rank().idx() {
            (s1, s2)
        } else {
            (s2, s1)
        };

        let mut current_square = current_square.bb();
        let end_square = end_square.bb();

        current_square = current_square.north();

        while current_square.as_u64() != end_square.as_u64() {
            squares |= current_square.as_u64();
            current_square = current_square.north();
        }

        return Some(Bitboard::new(squares));
    }

    // Diagonal
    if s1.file().idx().abs_diff(s2.file().idx()) == s1.rank().idx().abs_diff(s2.rank().idx()) {
        let (start_square, end_square) = if s1.file().idx() < s2.file().idx() {
            (s1, s2)
        } else {
            (s2, s1)
        };

        // We're starting with the leftmost of the two squares.
        // If that square is below our end square, we need to move up and to the right.
        // If that square is above our end square, we need to move below and to the right.
        let direction = if start_square.rank().idx() < end_square.rank().idx() {
            Direction::NorthEast
        } else {
            Direction::SouthEast
        };

        let mut current_square = start_square.bb();
        current_square = current_square.in_direction(direction);

        while current_square.as_u64() != end_square.bb().as_u64() {
            squares |= current_square.as_u64();
            current_square = current_square.in_direction(direction);
        }

        return Some(Bitboard::new(squares));
    }

    // No path between these two squares
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::square::squares::all::*;

    #[test]
    fn test_between_on_rank() {
        assert_eq!(between(B4, G4), C4 | D4 | E4 | F4);
    }

    #[test]
    fn test_between_on_rank_for_full_rank() {
        assert_eq!(between(A1, H1), B1 | C1 | D1 | E1 | F1 | G1);
    }

    #[test]
    fn test_between_on_file() {
        assert_eq!(between(C2, C7), C3 | C4 | C5 | C6);
    }

    #[test]
    fn test_between_on_file_for_full_file() {
        assert_eq!(between(H1, H8), H2 | H3 | H4 | H5 | H6 | H7);
    }

    #[test]
    fn test_between_on_diagonal() {
        assert_eq!(between(A1, H8), B2 | C3 | D4 | E5 | F6 | G7);
    }

    #[test]
    fn test_between_on_diagonal_descending() {
        assert_eq!(between(A8, H1), B7 | C6 | D5 | E4 | F3 | G2);
    }
}
