use std::collections::HashSet;

use crate::chess::{
    game::Game,
    moves::{Move, MoveListExt},
    piece::{PieceKind, PromotionPieceKind},
    san::constants,
    square::{File, Rank, Square, squares},
};

enum AmbiguityResolution {
    None,
    File(File),
    Rank(Rank),
    Exact(File, Rank),
}

impl AmbiguityResolution {
    fn satisfied_by(&self, mv: Move) -> bool {
        match self {
            Self::None => true,
            Self::File(file) => mv.src().file() == *file,
            Self::Rank(rank) => mv.src().rank() == *rank,
            Self::Exact(file, rank) => mv.src().file() == *file && mv.src().rank() == *rank,
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    InvalidFile,
    InvalidRank,
    InvalidPromotionPiece,
    InvalidAmbiguityResolution,
    InvalidMove,
    NoXInCaptureMove,
}

fn parse_ambiguity_resolution(chars: &[char]) -> Result<AmbiguityResolution, ParseError> {
    match chars.len() {
        0 => Ok(AmbiguityResolution::None),
        1 => {
            let c = chars[0];

            if let Ok(file) = parse_file(c) {
                return Ok(AmbiguityResolution::File(file));
            }

            if let Ok(rank) = parse_rank(c) {
                return Ok(AmbiguityResolution::Rank(rank));
            }

            Err(ParseError::InvalidAmbiguityResolution)
        }
        2 => {
            let file = chars[0];
            let rank = chars[1];

            let file = parse_file(file)?;
            let rank = parse_rank(rank)?;

            Ok(AmbiguityResolution::Exact(file, rank))
        }
        _ => Err(ParseError::InvalidAmbiguityResolution),
    }
}

fn parse_file(c: char) -> Result<File, ParseError> {
    Ok(match c {
        'a' => File::A,
        'b' => File::B,
        'c' => File::C,
        'd' => File::D,
        'e' => File::E,
        'f' => File::F,
        'g' => File::G,
        'h' => File::H,
        _ => return Err(ParseError::InvalidFile),
    })
}

fn parse_rank(c: char) -> Result<Rank, ParseError> {
    Ok(match c {
        '1' => Rank::R1,
        '2' => Rank::R2,
        '3' => Rank::R3,
        '4' => Rank::R4,
        '5' => Rank::R5,
        '6' => Rank::R6,
        '7' => Rank::R7,
        '8' => Rank::R8,
        _ => return Err(ParseError::InvalidRank),
    })
}

fn parse_promotion_piece(piece: &str) -> Result<PromotionPieceKind, ParseError> {
    if piece.len() != 1 {
        return Err(ParseError::InvalidPromotionPiece);
    }

    let char = piece.chars().next().unwrap();
    Ok(match char {
        'Q' => PromotionPieceKind::Queen,
        'R' => PromotionPieceKind::Rook,
        'N' => PromotionPieceKind::Knight,
        'B' => PromotionPieceKind::Bishop,
        _ => return Err(ParseError::InvalidPromotionPiece),
    })
}

fn parse_piece(c: char) -> Option<PieceKind> {
    match c {
        'K' => Some(PieceKind::King),
        'Q' => Some(PieceKind::Queen),
        'R' => Some(PieceKind::Rook),
        'B' => Some(PieceKind::Bishop),
        'N' => Some(PieceKind::Knight),
        'P' => Some(PieceKind::Pawn),
        _ => None,
    }
}

fn parse_source_square(game: &Game, src: &str, dst: Square) -> Result<Square, ParseError> {
    let piece_moves: Vec<(PieceKind, Move)> = game
        .moves()
        .iter()
        .map(|mv| (game.board.piece_guaranteed_at(mv.src()).kind, *mv))
        .collect();

    // Pawn move
    if src.is_empty() {
        let matching_source_squares: HashSet<Square> = piece_moves
            .into_iter()
            .filter(|&(piece, mv)| piece == PieceKind::Pawn && mv.dst() == dst)
            .map(|(_, mv)| mv.src())
            .collect();

        if matching_source_squares.len() != 1 {
            return Err(ParseError::InvalidMove);
        }

        return Ok(*matching_source_squares.iter().next().unwrap());
    }

    let src_chars: Vec<char> = src.chars().collect();
    let (first_char, rest) = src_chars.split_first().unwrap();

    if let Some(moved_piece) = parse_piece(*first_char) {
        let ambiguity_resolution = parse_ambiguity_resolution(rest)?;

        let matching_source_squares: Vec<Square> = piece_moves
            .into_iter()
            .filter(|&(piece, mv)| {
                piece == moved_piece && mv.dst() == dst && ambiguity_resolution.satisfied_by(mv)
            })
            .map(|(_, mv)| mv.src())
            .collect();

        if matching_source_squares.len() != 1 {
            return Err(ParseError::InvalidMove);
        }

        return Ok(*matching_source_squares.first().unwrap());
    }

    let ambiguity_resolution = parse_ambiguity_resolution(&src_chars)?;

    // If the SAN didn't specify the piece that was going to be moved but rather the square, then
    // we moved a pawn, so restrict to pawn moves only.
    let matching_source_squares: Vec<Square> = piece_moves
        .into_iter()
        .filter(|&(piece, mv)| {
            piece == PieceKind::Pawn && mv.dst() == dst && ambiguity_resolution.satisfied_by(mv)
        })
        .map(|(_, mv)| mv.src())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if matching_source_squares.len() != 1 {
        return Err(ParseError::InvalidMove);
    }

    Ok(*matching_source_squares.first().unwrap())
}

fn parse_destination_square(sq: &str) -> Result<Square, ParseError> {
    assert_eq!(sq.len(), 2);

    let mut chars = sq.chars();
    let file = parse_file(chars.next().unwrap())?;
    let rank = parse_rank(chars.next().unwrap())?;

    Ok(Square::from_file_and_rank(file, rank))
}

fn parse_move_squares(game: &Game, mv: &str) -> Result<(Square, Square), ParseError> {
    let (src, dst) = mv.split_at(mv.len() - 2);

    let dst = parse_destination_square(dst)?;
    let src = parse_source_square(game, src, dst)?;

    Ok((src, dst))
}

fn parse_capture_squares(game: &Game, mv: &str) -> Result<(Square, Square), ParseError> {
    let (src, dst) = mv
        .split_once(constants::CAPTURE)
        .ok_or(ParseError::NoXInCaptureMove)?;

    let dst = parse_destination_square(dst)?;
    let src = parse_source_square(game, src, dst)?;

    Ok((src, dst))
}

fn parse_squares(game: &Game, mv: &str) -> Result<(Square, Square), ParseError> {
    let is_capture_move = mv.contains(constants::CAPTURE);
    if is_capture_move {
        return parse_capture_squares(game, mv);
    }

    parse_move_squares(game, mv)
}

pub fn parse_move(game: &Game, mv: &str) -> Result<Move, ParseError> {
    if mv == constants::KINGSIDE_CASTLE {
        return Ok(game.moves().expect_matching(
            squares::king_start(game.player),
            squares::kingside_castle_dest(game.player),
            None,
        ));
    }

    if mv == constants::QUEENSIDE_CASTLE {
        return Ok(game.moves().expect_matching(
            squares::king_start(game.player),
            squares::queenside_castle_dest(game.player),
            None,
        ));
    }

    let mv = mv
        .trim_end_matches(constants::CHECK)
        .trim_end_matches(constants::CHECKMATE);

    let (mv, promotion) = if mv.contains(constants::PROMOTION) {
        let (rest, promotion_piece) = mv
            .split_once(constants::PROMOTION)
            .ok_or(ParseError::InvalidPromotionPiece)?;
        let promoted_to = parse_promotion_piece(promotion_piece)?;
        (rest, Some(promoted_to))
    } else {
        (mv, None)
    };

    let (src, dst) = parse_squares(game, mv)?;

    Ok(game.moves().expect_matching(src, dst, promotion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{fen, game::Game, piece::PromotionPieceKind, square::squares::all::*};

    fn test_parse_san(fen: &'static str, expected_mv: (Square, Square), san: &'static str) {
        crate::init();

        let game = Game::from_fen(fen).unwrap();
        let mv = parse_move(&game, san).unwrap();

        let expected_mv = game
            .moves()
            .expect_matching(expected_mv.0, expected_mv.1, None);

        assert_eq!(mv, expected_mv);
    }

    fn test_parse_san_with_promotion(
        fen: &'static str,
        expected_mv: (Square, Square, PromotionPieceKind),
        san: &'static str,
    ) {
        crate::init();

        let game = Game::from_fen(fen).unwrap();
        let mv = parse_move(&game, san).unwrap();

        let expected_mv =
            game.moves()
                .expect_matching(expected_mv.0, expected_mv.1, Some(expected_mv.2));

        assert_eq!(mv, expected_mv);
    }

    #[test]
    fn san_simple_pawn_move() {
        test_parse_san(fen::START_POS, (E2, E4), "e4");
    }

    #[test]
    fn san_simple_pawn_capture() {
        test_parse_san(
            "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2",
            (E4, D5),
            "exd5",
        );
    }

    #[test]
    fn san_simple_knight_move() {
        test_parse_san(fen::START_POS, (B1, C3), "Nc3");
    }

    #[test]
    fn san_test_en_passant() {
        test_parse_san(
            "rnbqkbnr/ppp2ppp/4p3/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
            (E5, D6),
            "exd6",
        );
    }

    #[test]
    fn san_promotion() {
        let promotion_fen = "k7/7P/8/8/8/8/8/7K w - - 0 1";

        test_parse_san_with_promotion(promotion_fen, (H7, H8, PromotionPieceKind::Knight), "h8=N");
        test_parse_san_with_promotion(promotion_fen, (H7, H8, PromotionPieceKind::Bishop), "h8=B");
        test_parse_san_with_promotion(promotion_fen, (H7, H8, PromotionPieceKind::Rook), "h8=R+");
        test_parse_san_with_promotion(promotion_fen, (H7, H8, PromotionPieceKind::Queen), "h8=Q+");
    }

    #[test]
    fn san_file_ambiguity() {
        test_parse_san("R6R/8/8/8/8/8/8/1k4K1 w - - 0 1", (A8, B8), "Rab8+");
    }

    #[test]
    fn san_rank_ambiguity() {
        test_parse_san("R7/8/8/8/8/1k4K1/8/R7 w - - 0 1", (A1, A3), "R1a3+");
    }

    #[test]
    fn san_exact_ambiguity() {
        test_parse_san("1k1K4/8/8/8/4Q2Q/8/8/7Q w - - 0 1", (H4, E1), "Qh4e1");
    }

    #[test]
    fn san_castling() {
        let fen = "1k6/8/8/8/8/8/8/R3K2R w KQ - 0 1";

        test_parse_san(fen, (E1, G1), "O-O");
        test_parse_san(fen, (E1, C1), "O-O-O");
    }

    #[test]
    fn san_plus_for_check() {
        test_parse_san_with_promotion(
            "k7/6P1/8/8/8/8/8/K7 w - - 0 1",
            (G7, G8, PromotionPieceKind::Queen),
            "g8=Q+",
        );
    }

    #[test]
    fn pawn_capture_bug_from_pgn_conversion_20260115() {
        // cxd4 was interpreted as potentially referring to a knight capture due to not filtering to pawn moves
        test_parse_san(
            "r2qk2r/pp2b3/2n1bppn/2p1p2p/P1PP3P/1P1P1NP1/1B3PB1/RN1Q1RK1 b kq - 0 13",
            (C5, D4),
            "cxd4",
        );
    }

    #[test]
    fn promotion_duplicate_square_ambiguity_20260115() {
        // When trying to parse out the source square, we considered the fact we saw the same square four
        // times as ambiguous when we should have deduplicated (since we have four possible promotion moves
        // from the same square)
        test_parse_san_with_promotion(
            "8/5p2/8/3k4/K3p1p1/3r4/p7/1R6 b - - 0 69",
            (A2, B1, PromotionPieceKind::Rook),
            "axb1=R",
        );
    }
}
