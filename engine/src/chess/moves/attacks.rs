use crate::chess::{direction::Direction, moves::tables, prelude::*};

#[inline]
pub fn pawn_attacks(s: Square, player: Player) -> Bitboard {
    tables::lookup_pawn_attacks(s, player)
}

#[inline]
pub fn knight_attacks(s: Square) -> Bitboard {
    tables::lookup_knight_attacks(s)
}

#[inline]
pub const fn bishop_attacks(s: Square, pieces: Bitboard) -> Bitboard {
    tables::lookup_bishop_attacks(s, pieces)
}

#[inline]
pub const fn rook_attacks(s: Square, pieces: Bitboard) -> Bitboard {
    tables::lookup_rook_attacks(s, pieces)
}

#[inline]
pub const fn king_attacks(s: Square) -> Bitboard {
    tables::lookup_king_attacks(s)
}

pub const fn generate_pawn_attacks(square: Square, player: Player) -> Bitboard {
    let sq = square.bb();

    Bitboard::EMPTY
        .or(sq.forward(player).west())
        .or(sq.forward(player).east())
}

pub const fn generate_knight_attacks(square: Square) -> Bitboard {
    let sq = square.bb();

    // Going clockwise, starting at 12
    Bitboard::EMPTY
        .or(sq.north().north_east())
        .or(sq.east().north_east())
        .or(sq.east().south_east())
        .or(sq.south().south_east())
        .or(sq.south().south_west())
        .or(sq.west().south_west())
        .or(sq.west().north_west())
        .or(sq.north().north_west())
}

#[allow(unused, reason = "Here for completeness")]
pub fn generate_bishop_attacks(square: Square, pieces: Bitboard) -> Bitboard {
    generate_sliding_attacks(square, Direction::DIAGONAL, pieces)
}

#[allow(unused, reason = "Here for completeness")]
pub fn generate_rook_attacks(square: Square, pieces: Bitboard) -> Bitboard {
    generate_sliding_attacks(square, Direction::CARDINAL, pieces)
}

#[allow(unused, reason = "Here for completeness")]
fn generate_sliding_attacks(
    square: Square,
    directions: &[Direction],
    pieces: Bitboard,
) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;

    for direction in directions {
        let mut current_square = square.bb();

        // Until we're off the board or blocked
        while current_square.any() && !pieces.contains(current_square.single()) {
            current_square = current_square.in_direction(*direction);
            attacks |= current_square;
        }
    }

    attacks
}

pub const fn generate_king_attacks(square: Square) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let sq = square.bb();

    let mut i = 0;
    while i < Direction::ALL.len() {
        let dir = Direction::ALL[i];
        attacks.or_assign(sq.in_direction(dir));
        i += 1;
    }

    attacks
}

pub fn all_attackers_of(board: &Board, square: Square, occupied: Bitboard) -> Bitboard {
    let mut attackers = Bitboard::EMPTY;

    attackers |= pawn_attacks(square, White) & board.pawns(Black);
    attackers |= pawn_attacks(square, Black) & board.pawns(White);

    attackers |= knight_attacks(square) & board.all_knights();
    attackers |= bishop_attacks(square, occupied) & board.all_diagonal_sliders();
    attackers |= rook_attacks(square, occupied) & board.all_orthogonal_sliders();
    attackers |= king_attacks(square) & board.all_kings();

    attackers
}
