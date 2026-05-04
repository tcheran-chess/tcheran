use crate::chess::{
    board::Board,
    game::{CastleRights, Game},
    piece::Piece,
    player::Player,
    square::{File, Rank, Square, squares},
};

#[derive(Debug)]
pub struct FenRank([Option<Piece>; File::N]);

#[derive(Debug)]
pub enum ParseError {
    InvalidPosition,
    InvalidPlayer,
    InvalidCastling,
    InvalidEnPassantTarget,
    InvalidHalfmoveClock,
    InvalidFullmoveNumber,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPosition => write!(f, "Invalid position"),
            Self::InvalidPlayer => write!(f, "Invalid player"),
            Self::InvalidCastling => write!(f, "Invalid castling"),
            Self::InvalidEnPassantTarget => write!(f, "Invalid en passant target"),
            Self::InvalidHalfmoveClock => write!(f, "Invalid halfmove clock"),
            Self::InvalidFullmoveNumber => write!(f, "Invalid fullmove number"),
        }
    }
}

fn fen_piece(input: char) -> Result<Piece, ()> {
    Ok(match input {
        'R' => Piece::WHITE_ROOK,
        'N' => Piece::WHITE_KNIGHT,
        'B' => Piece::WHITE_BISHOP,
        'Q' => Piece::WHITE_QUEEN,
        'K' => Piece::WHITE_KING,
        'P' => Piece::WHITE_PAWN,
        'r' => Piece::BLACK_ROOK,
        'n' => Piece::BLACK_KNIGHT,
        'b' => Piece::BLACK_BISHOP,
        'q' => Piece::BLACK_QUEEN,
        'k' => Piece::BLACK_KING,
        'p' => Piece::BLACK_PAWN,
        _ => return Err(()),
    })
}

fn fen_line(input: &str) -> Result<FenRank, ()> {
    let mut result: [Option<Piece>; File::N] = [None; File::N];

    let mut file = 0;

    for c in input.chars() {
        if file >= File::N {
            return Err(());
        }

        // If we see a number, skip that many squares, leaving them empty
        if c.is_numeric() {
            let number_of_empty_squares = c.to_string().parse::<usize>().unwrap();
            file += number_of_empty_squares;
        } else {
            let piece = fen_piece(c)?;
            result[file] = Some(piece);
            file += 1;
        }
    }

    Ok(FenRank(result))
}

fn fen_position(input: &str) -> Result<Board, ()> {
    let ranks = input.split('/');

    let mut all_pieces: [Option<Piece>; Square::N] = [None; Square::N];

    for (i, r) in ranks.into_iter().rev().enumerate() {
        if i > Rank::N {
            return Err(());
        }

        let rank = fen_line(r)?;
        all_pieces[i * File::N..(i + 1) * File::N].copy_from_slice(&rank.0);
    }

    all_pieces.try_into()
}

fn fen_color(input: &str) -> Result<Player, ()> {
    Ok(match input {
        "w" => Player::White,
        "b" => Player::Black,
        _ => return Err(()),
    })
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum FenCastleRight {
    WhiteKingside,
    WhiteQueenside,
    BlackKingside,
    BlackQueenside,
}

fn standard_castle_right(input: char) -> Result<(FenCastleRight, Square), ()> {
    use FenCastleRight::*;

    Ok(match input {
        'K' => (WhiteKingside, squares::WHITE_STANDARD_KINGSIDE_ROOK_START),
        'Q' => (WhiteQueenside, squares::WHITE_STANDARD_QUEENSIDE_ROOK_START),
        'k' => (BlackKingside, squares::BLACK_STANDARD_KINGSIDE_ROOK_START),
        'q' => (BlackQueenside, squares::BLACK_STANDARD_QUEENSIDE_ROOK_START),
        _ => return Err(()),
    })
}

fn frc_castle_right(input: char, board: &Board) -> Result<(FenCastleRight, Square), ()> {
    use FenCastleRight::*;

    let back_ranks = [Rank::R1, Rank::R8];

    let leftmost_rook = |player: Player| {
        File::ALL
            .iter()
            .map(|f| Square::from_file_and_rank(*f, back_ranks[player]))
            .find(|s| board.rooks(player).contains(*s))
            .ok_or(())
    };

    let rightmost_rook = |player: Player| {
        File::ALL
            .iter()
            .rev()
            .map(|f| Square::from_file_and_rank(*f, back_ranks[player]))
            .find(|s| board.rooks(player).contains(*s))
            .ok_or(())
    };

    Ok(match input {
        'A'..='H' => {
            let king_file = board.king_square(Player::White).file();

            let file = fen_file(input.to_ascii_lowercase())?;
            let square = Square::from_file_and_rank(file, Rank::R1);

            let right = if file < king_file {
                WhiteQueenside
            } else {
                WhiteKingside
            };

            (right, square)
        }
        'a'..='h' => {
            let king_file = board.king_square(Player::Black).file();

            let file = fen_file(input.to_ascii_lowercase())?;
            let square = Square::from_file_and_rank(file, Rank::R8);

            let right = if file < king_file {
                BlackQueenside
            } else {
                BlackKingside
            };

            (right, square)
        }
        'K' => (WhiteKingside, rightmost_rook(Player::White)?),
        'Q' => (WhiteQueenside, leftmost_rook(Player::White)?),
        'k' => (BlackKingside, rightmost_rook(Player::Black)?),
        'q' => (BlackQueenside, leftmost_rook(Player::Black)?),
        _ => return Err(()),
    })
}

fn fen_castling(input: &str, frc: bool, board: &Board) -> Result<[CastleRights; Player::N], ()> {
    if input == "-" {
        return Ok([CastleRights::none(), CastleRights::none()]);
    }

    let mut white_castle_rights = CastleRights::none();
    let mut black_castle_rights = CastleRights::none();

    for c in input.chars() {
        let (right, square) = if !frc {
            standard_castle_right(c)?
        } else {
            frc_castle_right(c, board)?
        };

        match right {
            FenCastleRight::WhiteKingside => &mut white_castle_rights.king_side,
            FenCastleRight::WhiteQueenside => &mut white_castle_rights.queen_side,
            FenCastleRight::BlackKingside => &mut black_castle_rights.king_side,
            FenCastleRight::BlackQueenside => &mut black_castle_rights.queen_side,
        }
        .replace(square);
    }

    Ok([white_castle_rights, black_castle_rights])
}

fn fen_file(input: char) -> Result<File, ()> {
    Ok(match input {
        'a' => File::A,
        'b' => File::B,
        'c' => File::C,
        'd' => File::D,
        'e' => File::E,
        'f' => File::F,
        'g' => File::G,
        'h' => File::H,
        _ => return Err(()),
    })
}

fn fen_square(input: &str) -> Result<Square, ()> {
    if input.len() != 2 {
        return Err(());
    }

    let mut chars = input.chars();
    let file = chars.next().unwrap();
    let rank = chars.next().unwrap();

    let file = fen_file(file)?;

    let rank = match rank {
        '1' => Rank::R1,
        '2' => Rank::R2,
        '3' => Rank::R3,
        '4' => Rank::R4,
        '5' => Rank::R5,
        '6' => Rank::R6,
        '7' => Rank::R7,
        '8' => Rank::R8,
        _ => return Err(()),
    };

    Ok(Square::from_file_and_rank(file, rank))
}

fn fen_en_passant_target(input: &str) -> Result<Option<Square>, ()> {
    if input == "-" {
        return Ok(None);
    }

    fen_square(input).map(Some)
}

fn fen_halfmove_clock(input: &str) -> Result<u32, ()> {
    input.parse::<u32>().map_err(|_| ())
}

fn fen_fullmove_number(input: &str) -> Result<u32, ()> {
    input.parse::<u32>().map_err(|_| ())
}

#[inline(always)]
fn plies_from_fullmove_number(fullmove_number: u32, player: Player) -> u32 {
    (fullmove_number - 1) * 2 + u32::from(player == Player::Black)
}

#[rustfmt::skip]
pub fn parse(input: &str, frc: bool) -> Result<Game, ParseError> {
    let mut tokens = input.split_whitespace();

    let Some(position) = tokens.next() else { return Err(ParseError::InvalidPosition); };
    let board = fen_position(position).map_err(|()| ParseError::InvalidPosition)?;

    let Some(player) = tokens.next() else { return Err(ParseError::InvalidPlayer); };
    let player = fen_color(player).map_err(|()| ParseError::InvalidPlayer)?;

    let Some(castle_rights) = tokens.next() else { return Err(ParseError::InvalidCastling); };
    let castle_rights = fen_castling(castle_rights, frc, &board).map_err(|()| ParseError::InvalidCastling)?;

    let Some(en_passant_target) = tokens.next() else { return Err(ParseError::InvalidEnPassantTarget); };
    let en_passant_target = fen_en_passant_target(en_passant_target).map_err(|()| ParseError::InvalidEnPassantTarget)?;

    let halfmove_clock = match tokens.next() {
        Some(c) => {
            match fen_halfmove_clock(c) {
                Ok(c) => c,
                Err(()) => return Err(ParseError::InvalidHalfmoveClock),
            }
        },
        None => 0,
    };

    let fullmove_number = match tokens.next() {
        Some(c) => {
            match fen_fullmove_number(c) {
                Ok(c) => c,
                Err(()) => return Err(ParseError::InvalidFullmoveNumber),
            }
        },
        None => 1,
    };

    let plies = plies_from_fullmove_number(fullmove_number, player);

    Ok(Game::from_state(
        board,
        player,
        castle_rights,
        en_passant_target,
        halfmove_clock,
        plies,
        frc,
    ))
}

#[cfg(test)]
mod tests {
    use Player::*;

    use super::*;

    #[test]
    fn parse_startpos() {
        crate::init();

        let game_result = parse("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", false);
        assert!(game_result.is_ok());

        let game = game_result.unwrap();
        let default_game = Game::default();

        assert_eq!(game.board.pawns(White), default_game.board.pawns(White));
        assert_eq!(game.board.knights(White), default_game.board.knights(White));
        assert_eq!(game.board.bishops(White), default_game.board.bishops(White));
        assert_eq!(game.board.rooks(White), default_game.board.rooks(White));
        assert_eq!(game.board.queens(White), default_game.board.queens(White));
        assert_eq!(game.board.king(White), default_game.board.king(White));

        assert_eq!(game.board.pawns(Black), default_game.board.pawns(Black));
        assert_eq!(game.board.knights(Black), default_game.board.knights(Black));
        assert_eq!(game.board.bishops(Black), default_game.board.bishops(Black));
        assert_eq!(game.board.rooks(Black), default_game.board.rooks(Black));
        assert_eq!(game.board.queens(Black), default_game.board.queens(Black));
        assert_eq!(game.board.king(Black), default_game.board.king(Black));

        assert_eq!(game.plies, 0);
    }

    #[test]
    fn parse_kiwipete() {
        crate::init();

        assert!(
            parse("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", false)
                .is_ok()
        );
    }

    #[test]
    fn parse_kiwipete_without_halfmove_and_fullmove() {
        crate::init();

        assert!(
            parse("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ", false)
                .is_ok()
        );
    }

    #[test]
    fn parse_simple_frc() {
        crate::init();

        assert!(
            parse("rnbq1rk1/pp2bppp/2pppn2/8/8/1PN1P1Q1/P1PPBPPP/R1B1K1NR b HA - 0 7", true)
                .is_ok()
        );
    }

    #[test]
    fn plies_from_fullmove_number() {
        assert_eq!(super::plies_from_fullmove_number(1, Player::White), 0);
        assert_eq!(super::plies_from_fullmove_number(1, Player::Black), 1);
        assert_eq!(super::plies_from_fullmove_number(2, Player::White), 2);
        assert_eq!(super::plies_from_fullmove_number(2, Player::Black), 3);
    }
}
