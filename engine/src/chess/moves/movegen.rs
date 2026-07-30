use crate::chess::{
    Bitboard, Game, Move, PromotionPieceKind, Square, bitboards,
    bitboards::{back_rank, double_push_rank, pawn_back_rank},
    moves::{bishop_attacks, king_attacks, knight_attacks, rook_attacks},
    rays::{ray_between, ray_relative_antidiagonal, ray_relative_diagonal, ray_skewering},
    squares,
};

pub fn generate_legal_moves(game: &Game, mut f: impl FnMut(Move)) {
    generate_tacticals(game, &mut f);
    generate_quiets(game, &mut f);
}

pub fn generate_tacticals(game: &Game, f: &mut impl FnMut(Move)) {
    let (our_pieces, their_pieces) = game.board.occupancies(game.player);
    let all_pieces = our_pieces | their_pieces;
    let king = game.board.king_square(game.player);

    let number_of_checkers = game.checkers.count();

    // If we're in check by more than one attacker, we can only get out of check via a king move
    if number_of_checkers > 1 {
        generate_king_captures(game, king, their_pieces, f);
        return;
    }

    let dst_mask = if number_of_checkers == 1 {
        game.checkers
    } else {
        their_pieces
    };

    let promotion_squares = !our_pieces & back_rank(!game.player);

    let pawn_dst_mask = if number_of_checkers == 1 {
        let pin_ray = ray_between(king, game.checkers.single());
        promotion_squares & pin_ray
    } else {
        promotion_squares
    };

    let pinned = game.pinned[game.player];

    generate_pawn_tacticals(
        game,
        game.board.pawns(game.player),
        their_pieces,
        dst_mask | pawn_dst_mask,
        pinned,
        king,
        f,
    );

    generate_knight_captures(game.board.knights(game.player), dst_mask, pinned, f);

    generate_diagonal_slider_captures(
        game.board.diagonal_sliders(game.player),
        all_pieces,
        dst_mask,
        pinned,
        king,
        f,
    );
    generate_orthogonal_slider_captures(
        game.board.orthogonal_sliders(game.player),
        all_pieces,
        dst_mask,
        pinned,
        king,
        f,
    );
    generate_king_captures(game, king, their_pieces, f);
}

pub fn generate_quiets(game: &Game, f: &mut impl FnMut(Move)) {
    let (our_pieces, their_pieces) = game.board.occupancies(game.player);
    let all_pieces = our_pieces | their_pieces;
    let king = game.board.king_square(game.player);

    let number_of_checkers = game.checkers.count();

    // If we're in check by more than one attacker, we can only get out of check via a king move
    if number_of_checkers > 1 {
        generate_king_quiets(game, king, all_pieces, f);
        return;
    }

    let dst_mask = if number_of_checkers == 1 {
        let checker_sq = game.checkers.single();
        ray_between(checker_sq, king)
    } else {
        !(our_pieces | their_pieces)
    };

    let promotion_squares = !our_pieces & back_rank(!game.player);

    let pawn_dst_mask = if number_of_checkers == 1 {
        game.checkers & promotion_squares
    } else {
        promotion_squares
    };

    let pinned = game.pinned[game.player];

    generate_pawn_quiets(
        game,
        game.board.pawns(game.player),
        all_pieces,
        dst_mask | pawn_dst_mask,
        pinned,
        king,
        f,
    );
    generate_knight_quiets(game.board.knights(game.player), dst_mask, pinned, f);
    generate_diagonal_slider_quiets(
        game.board.diagonal_sliders(game.player),
        all_pieces,
        dst_mask,
        pinned,
        king,
        f,
    );
    generate_orthogonal_slider_quiets(
        game.board.orthogonal_sliders(game.player),
        all_pieces,
        dst_mask,
        pinned,
        king,
        f,
    );
    generate_king_quiets(game, king, all_pieces, f);

    if !game.checkers.any() {
        generate_castles(game, all_pieces, f);
    }
}

fn generate_pawn_tacticals(
    game: &Game,
    pawns: Bitboard,
    their_pieces: Bitboard,
    dst_mask: Bitboard,
    pinned: Bitboard,
    king: Square,
    f: &mut impl FnMut(Move),
) {
    let us = game.player;
    let them = !game.player;

    let left_pin_mask = ray_relative_antidiagonal(king, us);
    let right_pin_mask = ray_relative_diagonal(king, us);

    let unpinned_pawns = pawns & !pinned;
    let pinned_pawns = pawns & pinned;
    let can_attack_left = unpinned_pawns | (pinned_pawns & left_pin_mask);
    let can_attack_right = unpinned_pawns | (pinned_pawns & right_pin_mask);

    let left_attacks = can_attack_left.forward(us).west() & dst_mask;
    let right_attacks = can_attack_right.forward(us).east() & dst_mask;

    let promotion_rank = back_rank(them);

    // Left promotion captures
    for dst in left_attacks & their_pieces & promotion_rank {
        let src = dst.backward(us).east();

        f(Move::capture_promotion(src, dst, PromotionPieceKind::Queen));
        f(Move::capture_promotion(src, dst, PromotionPieceKind::Rook));
        f(Move::capture_promotion(src, dst, PromotionPieceKind::Knight));
        f(Move::capture_promotion(src, dst, PromotionPieceKind::Bishop));
    }

    // Right promotion captures
    for dst in right_attacks & their_pieces & promotion_rank {
        let src = dst.backward(us).west();

        f(Move::capture_promotion(src, dst, PromotionPieceKind::Queen));
        f(Move::capture_promotion(src, dst, PromotionPieceKind::Rook));
        f(Move::capture_promotion(src, dst, PromotionPieceKind::Knight));
        f(Move::capture_promotion(src, dst, PromotionPieceKind::Bishop));
    }

    let will_promote_rank = bitboards::pawn_back_rank(them);
    let promotion_destinations =
        (unpinned_pawns & will_promote_rank).forward(us) & dst_mask & !their_pieces;

    // Queen promotions
    for dst in promotion_destinations {
        let src = dst.backward(us);
        f(Move::push_promotion(src, dst, PromotionPieceKind::Queen));
    }

    for dst in left_attacks & !promotion_rank {
        let src = dst.backward(us).east();
        f(Move::capture(src, dst));
    }

    for dst in right_attacks & !promotion_rank {
        let src = dst.backward(us).west();
        f(Move::capture(src, dst));
    }

    // En-passant capture: Pawns either side of the en-passant pawn can capture
    if let Some(en_passant_target) = game.en_passant_target {
        let ep = en_passant_target.bb();
        let left_attacker = can_attack_left & ep.backward(us).east();
        let right_attacker = can_attack_right & ep.backward(us).west();

        for src in left_attacker | right_attacker {
            f(Move::en_passant(src, en_passant_target));
        }
    }
}

fn generate_pawn_quiets(
    game: &Game,
    pawns: Bitboard,
    all_pieces: Bitboard,
    dst_mask: Bitboard,
    pinned: Bitboard,
    king: Square,
    f: &mut impl FnMut(Move),
) {
    let us = game.player;
    let them = !game.player;

    let unpinned_pawns = pawns & !pinned;
    let pinned_pawns = pawns & pinned;

    let can_push = unpinned_pawns | (pinned_pawns & king.file().bitboard());

    let will_promote_rank = pawn_back_rank(them);
    let non_promotion_pushes = can_push & !will_promote_rank;
    let promotion_pushes = can_push & will_promote_rank;

    let single_push_destinations = non_promotion_pushes.forward(us) & !all_pieces;
    for dst in single_push_destinations & dst_mask {
        let src = dst.backward(us);
        f(Move::quiet(src, dst));
    }

    let double_push_destinations =
        (single_push_destinations & double_push_rank(us)).forward(us) & !all_pieces;

    for dst in double_push_destinations & dst_mask {
        let src = dst.backward(us).backward(us);
        f(Move::double_push(src, dst));
    }

    let underpromotion_destinations = promotion_pushes.forward(us) & !all_pieces;
    for dst in underpromotion_destinations & dst_mask {
        let src = dst.backward(us);
        f(Move::push_promotion(src, dst, PromotionPieceKind::Rook));
        f(Move::push_promotion(src, dst, PromotionPieceKind::Knight));
        f(Move::push_promotion(src, dst, PromotionPieceKind::Bishop));
    }
}

fn generate_knight_captures(
    knights: Bitboard,
    dst_mask: Bitboard,
    pinned: Bitboard,
    f: &mut impl FnMut(Move),
) {
    for knight in knights & !pinned {
        let destinations = knight_attacks(knight) & dst_mask;
        for dst in destinations {
            f(Move::capture(knight, dst));
        }
    }
}

fn generate_knight_quiets(
    knights: Bitboard,
    dst_mask: Bitboard,
    pinned: Bitboard,
    f: &mut impl FnMut(Move),
) {
    for knight in knights & !pinned {
        let destinations = knight_attacks(knight) & dst_mask;
        for dst in destinations {
            f(Move::quiet(knight, dst));
        }
    }
}

fn generate_diagonal_slider_captures(
    diagonal_sliders: Bitboard,
    all_pieces: Bitboard,
    dst_mask: Bitboard,
    pinned: Bitboard,
    king: Square,
    f: &mut impl FnMut(Move),
) {
    for src in diagonal_sliders & !pinned {
        let destinations = bishop_attacks(src, all_pieces) & dst_mask;
        for dst in destinations {
            f(Move::capture(src, dst));
        }
    }

    for src in diagonal_sliders & pinned {
        let pin_ray = ray_skewering(king, src);
        let destinations = bishop_attacks(src, all_pieces) & dst_mask & pin_ray;
        for dst in destinations {
            f(Move::capture(src, dst));
        }
    }
}

fn generate_diagonal_slider_quiets(
    diagonal_sliders: Bitboard,
    all_pieces: Bitboard,
    dst_mask: Bitboard,
    pinned: Bitboard,
    king: Square,
    f: &mut impl FnMut(Move),
) {
    for src in diagonal_sliders & !pinned {
        let destinations = bishop_attacks(src, all_pieces) & dst_mask;
        for dst in destinations {
            f(Move::quiet(src, dst));
        }
    }

    for src in diagonal_sliders & pinned {
        let pin_ray = ray_skewering(king, src);
        let destinations = bishop_attacks(src, all_pieces) & dst_mask & pin_ray;
        for dst in destinations {
            f(Move::quiet(src, dst));
        }
    }
}

fn generate_orthogonal_slider_captures(
    orthogonal_sliders: Bitboard,
    all_pieces: Bitboard,
    dst_mask: Bitboard,
    pinned: Bitboard,
    king: Square,
    f: &mut impl FnMut(Move),
) {
    for src in orthogonal_sliders & !pinned {
        let destinations = rook_attacks(src, all_pieces) & dst_mask;
        for dst in destinations {
            f(Move::capture(src, dst));
        }
    }

    for src in orthogonal_sliders & pinned {
        let pin_ray = ray_skewering(king, src);
        let destinations = rook_attacks(src, all_pieces) & dst_mask & pin_ray;
        for dst in destinations {
            f(Move::capture(src, dst));
        }
    }
}

fn generate_orthogonal_slider_quiets(
    orthogonal_sliders: Bitboard,
    all_pieces: Bitboard,
    dst_mask: Bitboard,
    pinned: Bitboard,
    king: Square,
    f: &mut impl FnMut(Move),
) {
    for src in orthogonal_sliders & !pinned {
        let destinations = rook_attacks(src, all_pieces) & dst_mask;
        for dst in destinations {
            f(Move::quiet(src, dst));
        }
    }

    for src in orthogonal_sliders & pinned {
        let pin_ray = ray_skewering(king, src);
        let destinations = rook_attacks(src, all_pieces) & dst_mask & pin_ray;
        for dst in destinations {
            f(Move::quiet(src, dst));
        }
    }
}

fn generate_king_captures(
    game: &Game,
    king: Square,
    their_pieces: Bitboard,
    f: &mut impl FnMut(Move),
) {
    let destinations = king_attacks(king) & their_pieces & !game.threats;
    for dst in destinations {
        f(Move::capture(king, dst));
    }
}

fn generate_king_quiets(game: &Game, king: Square, all_pieces: Bitboard, f: &mut impl FnMut(Move)) {
    let destinations = king_attacks(king) & !all_pieces & !game.threats;
    for dst in destinations {
        f(Move::quiet(king, dst));
    }
}

fn generate_castles(game: &Game, all_pieces: Bitboard, f: &mut impl FnMut(Move)) {
    let castle_rights_for_player = game.castle_rights[game.player];

    if let Some(kingside_dst) = castle_rights_for_player.king_side {
        generate_castle_move_for_side(
            game,
            all_pieces,
            f,
            kingside_dst,
            squares::kingside_rook_castle_end(game.player),
            squares::kingside_king_castle_end(game.player),
        );
    }

    if let Some(queenside_dst) = castle_rights_for_player.queen_side {
        generate_castle_move_for_side(
            game,
            all_pieces,
            f,
            queenside_dst,
            squares::queenside_rook_castle_end(game.player),
            squares::queenside_king_castle_end(game.player),
        );
    }
}

fn generate_castle_move_for_side(
    game: &Game,
    all_pieces: Bitboard,
    f: &mut impl FnMut(Move),
    rook: Square,
    rook_dst: Square,
    king_dst: Square,
) {
    if game.pinned[game.player].contains(rook) {
        return;
    }

    let king = game.board.king_square(game.player);

    let required_safe_squares = ray_between(king, king_dst) | king.bb() | king_dst.bb();
    let required_empty_squares = required_safe_squares | ray_between(king, rook) | rook_dst.bb();

    let blockers = all_pieces ^ king.bb() ^ rook.bb();

    if (required_empty_squares & blockers).is_empty()
        && (required_safe_squares & game.threats).is_empty()
    {
        f(Move::castles(king, rook));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{
        moves::{MoveList, MoveListExt},
        square::squares::all::*,
    };

    #[inline(always)]
    fn should_allow_move(fen: &str, mv: (Square, Square)) {
        crate::init();
        let game = Game::from_valid_fen(fen);
        let mut movelist = MoveList::new();
        generate_legal_moves(&game, |m| movelist.push(m));

        assert!(movelist.iter().any(|m| (m.from(), m.to()) == mv));
    }

    #[inline(always)]
    fn should_not_allow_move(fen: &str, mv: (Square, Square)) {
        crate::init();
        let game = Game::from_valid_fen(fen);
        let mut movelist = MoveList::new();
        generate_legal_moves(&game, |m| movelist.push(m));

        assert!(movelist.iter().all(|m| (m.from(), m.to()) != mv));
    }

    #[test]
    fn test_simple_rook_move() {
        should_allow_move("rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 2", (A1, A2));
    }

    #[test]
    fn test_simple_bishop_move() {
        let fen = "rnbqkbnr/1ppppp1p/p5p1/8/8/1P6/PBPPPPPP/RN1QKBNR w KQkq - 0 3";
        should_allow_move(fen, (B2, C3));
        should_allow_move(fen, (B2, H8));
    }

    #[test]
    fn test_cant_capture_own_king() {
        should_not_allow_move(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            (F1, G1),
        );
    }

    #[test]
    fn test_kiwipete_en_passant_bug() {
        should_allow_move(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/Pp2P3/2N2Q1p/1PPBBPPP/R3K2R b KQkq a3 0 1",
            (B4, A3),
        );
    }

    #[test]
    fn test_pawn_push_along_pin_bug() {
        should_allow_move("rnb1kbnr/pppp1ppp/4pq2/8/8/5P2/PPPPPKPP/RNBQ1BNR w kq - 2 3", (F3, F4));
    }

    #[test]
    fn test_forbid_en_passant_revealed_check() {
        crate::init();

        let mut game = Game::from_valid_fen("8/8/8/8/k3p2Q/8/3P4/3K4 w - - 0 1");
        game.make_move(game.moves().expect_matching(D2, D4, None));

        assert!(game.moves().iter().all(|m| (m.from(), m.to()) != (E4, D3)));
    }

    #[test]
    fn test_forbid_pushing_pawn_into_pinning_piece() {
        should_not_allow_move(
            "rnbq2kr/pp1Pbppp/2p3Q1/8/2B5/8/PPP1NnPP/RNB1K2R b KQ - 4 9",
            (F7, G6),
        );
    }

    #[test]
    fn test_en_passant_bug_20230308() {
        should_allow_move("rnbqkbnr/2pppppp/p7/Pp6/8/8/1PPPPPPP/RNBQKBNR w KQkq b6 0 3", (A5, B6));
    }

    #[test]
    fn test_en_passant_bug_20251130() {
        // Thanks to Werner for reporting: https://talkchess.com/viewtopic.php?p=986038
        should_allow_move(
            "5r2/1p3k2/pBp1p1b1/3rq1b1/PPR1pPpp/4Q1P1/4P1BP/5RK1 b - f3 0 28",
            (G4, F3),
        );
    }

    #[test]
    fn test_castling_bug_20260209() {
        should_not_allow_move(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q2/PPP1BPpP/R1B1K2R w KQkq - 0 2",
            (E1, A1),
        );
    }

    #[test]
    fn test_en_passant_diagonal_pin_20260413() {
        crate::init();

        let mut game =
            Game::from_valid_fen("q7/2p3k1/3b2p1/pp1Pnp1p/3Q3P/P1P3P1/1P3PK1/2NB4 b - - 1 33");
        game.make_move(game.moves().expect_matching(C7, C5, None));

        assert!(game.moves().iter().any(|m| (m.from(), m.to()) == (D5, C6)));
    }

    #[test]
    fn test_allow_pawn_to_capture_along_pin_ray_20260426() {
        should_allow_move(
            "r3k2r/Pppp1ppp/1b3nbN/nPP5/BB2P3/5N2/qp1P2PP/R2Q1RK1 w kq - 0 2",
            (C5, B6),
        );
    }
}
