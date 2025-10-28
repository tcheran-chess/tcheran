use crate::chess::{bitboard::Bitboard, direction::Direction, player::Player, square::Square};

// In order for these functions to be const, they need to work with u64s instead of BitBoards,
// until we can make BitBoards' trait impls const.

pub const fn generate_pawn_attacks(square: Square, player: Player) -> Bitboard {
    let mut attacks = Bitboard::EMPTY.as_u64();
    let sq = square.bb();

    attacks |= sq.forward(player).west().as_u64();
    attacks |= sq.forward(player).east().as_u64();

    Bitboard::new(attacks)
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

pub const fn generate_king_attacks(square: Square) -> Bitboard {
    let mut attacks = Bitboard::EMPTY.as_u64();
    let sq = square.bb();

    // To be const, we can't loop over Direction::ALL
    attacks |= sq.in_direction(Direction::North).as_u64();
    attacks |= sq.in_direction(Direction::NorthEast).as_u64();
    attacks |= sq.in_direction(Direction::East).as_u64();
    attacks |= sq.in_direction(Direction::SouthEast).as_u64();
    attacks |= sq.in_direction(Direction::South).as_u64();
    attacks |= sq.in_direction(Direction::SouthWest).as_u64();
    attacks |= sq.in_direction(Direction::West).as_u64();
    attacks |= sq.in_direction(Direction::NorthWest).as_u64();

    Bitboard::new(attacks)
}
