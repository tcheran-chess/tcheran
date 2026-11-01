use crate::chess::{
    notations::san::sanmove::{SanAmbiguityResolution, SanFrom, SanMove, SanStatus},
    prelude::*,
};

pub fn to_san_move(game: &Game, mv: Move) -> SanMove {
    let from = mv.from();
    let to = mv.to();

    let piece = game.board.piece_guaranteed_at(from);

    if mv.is_castling()
        && let Some(side) = game.castle_rights[game.player].side_for_rook_sq(to)
    {
        return SanMove::Castle(side);
    }

    let mut game_after_move = game.clone();
    game_after_move.make_move(mv);
    let places_opponent_in_check = game_after_move.in_check();

    let ambiguity_resolution = ambiguity_resolution(game, mv);

    let from = match piece.kind {
        Pawn => SanFrom::Pawn(from.file()),
        kind => SanFrom::NonPawn(kind),
    };

    let is_capture = mv.is_capture();

    let promotion = mv.promotion();

    let status = if places_opponent_in_check {
        Some(if game_after_move.moves().is_empty() {
            SanStatus::Checkmate
        } else {
            SanStatus::Check
        })
    } else {
        None
    };

    SanMove::Move {
        from,
        ambiguity_resolution,
        to,
        promotion,
        is_capture,
        status,
    }
}

fn ambiguity_resolution(game: &Game, mv: Move) -> Option<SanAmbiguityResolution> {
    let from = mv.from();
    let to = mv.to();

    let piece = game.board.piece_guaranteed_at(from);
    if piece.kind == Pawn || piece.kind == King {
        return None;
    }

    let potentially_ambiguous_moves: Vec<Move> = game
        .moves()
        .iter()
        .filter(|&m| {
            // A move is potentially ambiguous if:

            // It moves to the same square our move
            m.to() == to &&

                // The kind of piece being moved is the same
                game.board.piece_guaranteed_at(m.from()).kind == piece.kind &&

                // It's not the exact same move
                *m != mv
        })
        .copied()
        .collect();

    if potentially_ambiguous_moves.is_empty() {
        return None;
    }

    let ambiguity_by_file = potentially_ambiguous_moves
        .iter()
        .any(|m| m.from().file() == mv.from().file());

    let ambiguity_by_rank = potentially_ambiguous_moves
        .iter()
        .any(|m| m.from().rank() == mv.from().rank());

    Some(match (ambiguity_by_file, ambiguity_by_rank) {
        (false, _) => SanAmbiguityResolution::File(from.file()),
        (true, false) => SanAmbiguityResolution::Rank(from.rank()),
        (true, true) => SanAmbiguityResolution::Square(from),
    })
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::chess::{moves::MoveListExt, notations::fen};

    fn test_san_string(fen: &'static str, mv: (Square, Square), expected_san: &'static str) {
        let game = Game::from_valid_fen(fen);
        let (from, to) = mv;
        let mv = game.moves().expect_matching(from, to);
        let san_mv = to_san_move(&game, mv);

        assert_eq!(san_mv.to_string(), expected_san);
    }

    fn test_san_string_with_promotion(
        fen: &'static str,
        mv: (Square, Square, PromotionPieceKind),
        expected_san: &'static str,
    ) {
        let game = Game::from_valid_fen(fen);
        let (from, to, promotion) = mv;
        let mv = game
            .moves()
            .expect_matching_with_promotion(from, to, Some(promotion));
        let san_mv = to_san_move(&game, mv);

        assert_eq!(san_mv.to_string(), expected_san);
    }

    #[test]
    fn san_simple_pawn_move() {
        test_san_string(fen::START_POS, (E2, E4), "e4");
    }

    #[test]
    fn san_simple_pawn_capture() {
        test_san_string(
            "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2",
            (E4, D5),
            "exd5",
        );
    }

    #[test]
    fn san_simple_knight_move() {
        test_san_string(fen::START_POS, (B1, C3), "Nc3");
    }

    #[test]
    fn san_test_en_passant() {
        test_san_string(
            "rnbqkbnr/ppp2ppp/4p3/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
            (E5, D6),
            "exd6",
        );
    }

    #[test]
    fn san_promotion() {
        let promotion_fen = "k7/7P/8/8/8/8/8/7K w - - 0 1";

        test_san_string_with_promotion(promotion_fen, (H7, H8, PromotionPieceKind::Knight), "h8=N");
        test_san_string_with_promotion(promotion_fen, (H7, H8, PromotionPieceKind::Bishop), "h8=B");
        test_san_string_with_promotion(promotion_fen, (H7, H8, PromotionPieceKind::Rook), "h8=R+");
        test_san_string_with_promotion(promotion_fen, (H7, H8, PromotionPieceKind::Queen), "h8=Q+");
    }

    #[test]
    fn san_file_ambiguity() {
        let fen = "R6R/8/8/8/8/8/8/1k4K1 w - - 0 1";
        let game = Game::from_valid_fen(fen);
        let ambiguous_move = (A8, B8);

        assert_matches!(
            ambiguity_resolution(
                &game,
                game.moves()
                    .expect_matching(ambiguous_move.0, ambiguous_move.1)
            ),
            Some(SanAmbiguityResolution::File(_))
        );

        test_san_string(fen, ambiguous_move, "Rab8+");
    }

    #[test]
    fn san_rank_ambiguity() {
        let fen = "R7/8/8/8/8/1k4K1/8/R7 w - - 0 1";
        let game = Game::from_valid_fen(fen);
        let ambiguous_move = (A1, A3);

        assert_matches!(
            ambiguity_resolution(
                &game,
                game.moves()
                    .expect_matching(ambiguous_move.0, ambiguous_move.1)
            ),
            Some(SanAmbiguityResolution::Rank(_))
        );

        test_san_string(fen, ambiguous_move, "R1a3+");
    }

    #[test]
    fn san_exact_ambiguity() {
        let fen = "1k1K4/8/8/8/4Q2Q/8/8/7Q w - - 0 1";
        let game = Game::from_valid_fen(fen);
        let ambiguous_move = (H4, E1);

        assert_matches!(
            ambiguity_resolution(
                &game,
                game.moves()
                    .expect_matching(ambiguous_move.0, ambiguous_move.1)
            ),
            Some(SanAmbiguityResolution::Square(_))
        );

        test_san_string(fen, ambiguous_move, "Qh4e1");
    }

    #[test]
    fn san_castling() {
        let fen = "1k6/8/8/8/8/8/8/R3K2R w KQ - 0 1";

        test_san_string(fen, (E1, H1), "O-O");
        test_san_string(fen, (E1, A1), "O-O-O");
    }

    #[test]
    fn san_plus_for_check() {
        test_san_string_with_promotion(
            "k7/6P1/8/8/8/8/8/K7 w - - 0 1",
            (G7, G8, PromotionPieceKind::Queen),
            "g8=Q+",
        );
    }

    #[test]
    fn san_ambiguity_20261016() {
        test_san_string(
            "2r2r1k/4ppnp/2bp2pN/p4PP1/1p2P2P/1Pq5/P1PQ1N2/1K1RR3 w - - 1 34",
            (F2, G4),
            "Nfg4",
        );
    }
}
