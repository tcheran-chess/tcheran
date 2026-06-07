use crate::chess::{Bitboard, Board, Direction, Player, Square, moves::tables};

#[inline]
pub fn pawn_attacks(s: Square, player: Player) -> Bitboard {
    tables::lookup_pawn_attacks(s, player)
}

#[inline]
pub fn knight_attacks(s: Square) -> Bitboard {
    tables::lookup_knight_attacks(s)
}

#[inline]
pub fn bishop_attacks(s: Square, pieces: Bitboard) -> Bitboard {
    tables::lookup_bishop_attacks(s, pieces)
}

#[inline]
pub fn rook_attacks(s: Square, pieces: Bitboard) -> Bitboard {
    tables::lookup_rook_attacks(s, pieces)
}

#[inline]
pub fn king_attacks(s: Square) -> Bitboard {
    tables::lookup_king_attacks(s)
}

pub fn generate_pawn_attacks(square: Square, player: Player) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let sq = square.bb();

    attacks |= sq.forward(player).west();
    attacks |= sq.forward(player).east();

    attacks
}

pub fn generate_knight_attacks(square: Square) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let sq = square.bb();

    // Going clockwise, starting at 12
    attacks |= sq.north().north_east();
    attacks |= sq.east().north_east();
    attacks |= sq.east().south_east();
    attacks |= sq.south().south_east();
    attacks |= sq.south().south_west();
    attacks |= sq.west().south_west();
    attacks |= sq.west().north_west();
    attacks |= sq.north().north_west();

    attacks
}

pub fn generate_bishop_attacks(square: Square, pieces: Bitboard) -> Bitboard {
    generate_sliding_attacks(square, Direction::DIAGONAL, pieces)
}

pub fn generate_rook_attacks(square: Square, pieces: Bitboard) -> Bitboard {
    generate_sliding_attacks(square, Direction::CARDINAL, pieces)
}

fn generate_sliding_attacks(
    square: Square,
    directions: &[Direction],
    pieces: Bitboard,
) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;

    for direction in directions {
        let mut current_square = square.bb();

        // Until we're off the board
        while current_square.any() {
            current_square = current_square.in_direction(*direction);
            attacks |= current_square;

            // Future squares blocked
            if (pieces & current_square).any() {
                break;
            }
        }
    }

    attacks
}

pub fn generate_king_attacks(square: Square) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let sq = square.bb();

    for direction in Direction::ALL {
        attacks |= sq.in_direction(*direction);
    }

    attacks
}

pub fn all_attackers_of(board: &Board, square: Square, occupied: Bitboard) -> Bitboard {
    use Player::*;

    let mut attackers = Bitboard::EMPTY;

    attackers |= pawn_attacks(square, White) & board.pawns(Black);
    attackers |= pawn_attacks(square, Black) & board.pawns(White);

    attackers |= knight_attacks(square) & board.all_knights();
    attackers |= bishop_attacks(square, occupied) & board.all_diagonal_sliders();
    attackers |= rook_attacks(square, occupied) & board.all_orthogonal_sliders();
    attackers |= king_attacks(square) & board.all_kings();

    attackers
}
