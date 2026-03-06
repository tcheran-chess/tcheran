use std::process::ExitCode;

use rand::prelude::*;

type ZobristComponent = u64;

const PIECE_N: usize = 6;
const SQUARE_N: usize = 64;
const PLAYER_N: usize = 2;
const CASTLE_RIGHTS_SIDE_N: usize = 2;

fn suffix(n: usize, bound: usize) -> &'static str {
    if n == bound - 1 { "" } else { ", " }
}

#[expect(
    unused_assignments,
    reason = "We define some components as 0 first to ensure we keep the order"
)]
#[expect(clippy::needless_range_loop, reason = "Feels more readable this way")]
fn main() -> ExitCode {
    let mut random = StdRng::seed_from_u64(0);

    let mut piece_square: [[[ZobristComponent; PIECE_N]; SQUARE_N]; PLAYER_N] =
        [[[0; PIECE_N]; SQUARE_N]; PLAYER_N];

    let mut castling: [[ZobristComponent; CASTLE_RIGHTS_SIDE_N]; PLAYER_N] =
        [[0; CASTLE_RIGHTS_SIDE_N]; PLAYER_N];

    let mut en_passant_square: [ZobristComponent; SQUARE_N] = [0; SQUARE_N];
    let mut side_to_play: ZobristComponent = 0;

    //
    // Generate components
    //

    for player in 0..PLAYER_N {
        for square in 0..SQUARE_N {
            for piece in 0..PIECE_N {
                piece_square[player][square][piece] = random.next_u64();
            }
        }

        for castle_rights in 0..CASTLE_RIGHTS_SIDE_N {
            castling[player][castle_rights] = random.next_u64();
        }
    }

    for square in &mut en_passant_square {
        *square = random.next_u64();
    }

    side_to_play = random.next_u64();

    //
    // Output components
    //

    println!(
        "#[expect(clippy::unreadable_literal, reason = \"Zobrist components are not supposed to be human readable\")]"
    );
    println!("#[rustfmt::skip]");
    println!("mod components {{");
    println!("    use super::*;");
    println!();
    println!("    pub type ZobristComponent = u64;");
    println!();

    println!(
        "    pub const PIECE_SQUARE: [[[ZobristComponent; PieceKind::N]; Square::N]; Player::N] = ["
    );
    for player in &piece_square {
        println!("        [");

        for square in player {
            print!("            [");

            for (i, piece) in square.iter().enumerate() {
                print!("{:#018x}{}", piece, suffix(i, PIECE_N));
            }

            println!("],");
        }

        println!("        ],");
    }
    println!("    ];");

    println!();

    println!("    pub const CASTLING: [[ZobristComponent; CastleRightsSide::N]; Player::N] = [");
    for player in &castling {
        print!("        [");

        for (i, castle_rights) in player.iter().enumerate() {
            print!("{:#018x}{}", castle_rights, suffix(i, CASTLE_RIGHTS_SIDE_N));
        }

        println!("],");
    }
    println!("    ];");

    println!();

    println!("    pub const EN_PASSANT_SQUARE: [ZobristComponent; Square::N] = [");
    for square in &en_passant_square {
        println!("        {square:#018x},");
    }
    println!("    ];");

    println!();

    println!("    pub const SIDE_TO_PLAY: ZobristComponent = {side_to_play:#018x};");

    println!("}}");

    ExitCode::SUCCESS
}
