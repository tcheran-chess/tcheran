use crate::chess::{
    bitboard::{Bitboard, bitboards},
    game::Game,
    movegen::{attackers, tables, tables::ray_between},
    moves::Move,
    piece::PromotionPieceKind,
    square::{Square, squares},
};

pub fn generate_legal_moves(game: &Game, mut f: impl FnMut(Move)) {
    generate_tacticals(game, &mut f);
    generate_quiets(game, &mut f);
}

pub fn generate_tacticals(game: &Game, f: &mut impl FnMut(Move)) {
    let all_pieces = game.board.occupancy();
    let their_pieces = game.board.occupancy_for(game.player.other());
    let king = game.board.king_square(game.player);

    let number_of_checkers = game.checkers.count();

    // If we're in check by more than one attacker, we can only get out of check via a king move
    if number_of_checkers > 1 {
        generate_king_captures(game, king, their_pieces, f);
        return;
    }

    let check_mask = if number_of_checkers == 1 {
        let checker_sq = game.checkers.single();
        ray_between(checker_sq, king) | game.checkers
    } else {
        Bitboard::FULL
    };

    let (orthogonal_pins, diagonal_pins) = (game.orthogonal_pins, game.diagonal_pins);

    generate_pawn_tacticals(
        game,
        game.board.pawns(game.player),
        their_pieces,
        all_pieces,
        check_mask,
        orthogonal_pins,
        diagonal_pins,
        f,
    );

    generate_knight_captures(
        game.board.knights(game.player),
        their_pieces,
        check_mask,
        orthogonal_pins,
        diagonal_pins,
        f,
    );
    generate_diagonal_slider_captures(
        game.board.diagonal_sliders(game.player),
        their_pieces,
        all_pieces,
        check_mask,
        orthogonal_pins,
        diagonal_pins,
        f,
    );
    generate_orthogonal_slider_captures(
        game.board.orthogonal_sliders(game.player),
        their_pieces,
        all_pieces,
        check_mask,
        orthogonal_pins,
        diagonal_pins,
        f,
    );
    generate_king_captures(game, king, their_pieces, f);
}

pub fn generate_quiets(game: &Game, f: &mut impl FnMut(Move)) {
    let all_pieces = game.board.occupancy();
    let king = game.board.king_square(game.player);

    let number_of_checkers = game.checkers.count();

    // If we're in check by more than one attacker, we can only get out of check via a king move
    if number_of_checkers > 1 {
        generate_king_quiets(game, king, all_pieces, f);
        return;
    }

    let check_mask = if number_of_checkers == 1 {
        let checker_sq = game.checkers.single();
        ray_between(checker_sq, king) | game.checkers
    } else {
        Bitboard::FULL
    };

    let (orthogonal_pins, diagonal_pins) = (game.orthogonal_pins, game.diagonal_pins);

    generate_pawn_quiets(
        game,
        game.board.pawns(game.player),
        all_pieces,
        check_mask,
        orthogonal_pins,
        diagonal_pins,
        f,
    );
    generate_knight_quiets(
        game.board.knights(game.player),
        all_pieces,
        check_mask,
        orthogonal_pins,
        diagonal_pins,
        f,
    );
    generate_diagonal_slider_quiets(
        game.board.diagonal_sliders(game.player),
        all_pieces,
        check_mask,
        orthogonal_pins,
        diagonal_pins,
        f,
    );
    generate_orthogonal_slider_quiets(
        game.board.orthogonal_sliders(game.player),
        all_pieces,
        check_mask,
        orthogonal_pins,
        diagonal_pins,
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
    all_pieces: Bitboard,
    check_mask: Bitboard,
    orthogonal_pins: Bitboard,
    diagonal_pins: Bitboard,
    f: &mut impl FnMut(Move),
) {
    // Pawns that are pinned orthogonally would reveal the king by capturing diagonally
    let can_capture_pawns = pawns & !orthogonal_pins;

    // Pawns that are pinned diagonally would reveal the king by moving forward
    let can_move_pawns = pawns & !diagonal_pins;

    // Pawns can move onto empty squares, as long as they block check if in check
    let available_move_squares = !all_pieces & check_mask;
    let single_push_available_move_pawns = available_move_squares.backward(game.player);

    // Pawns can push once if they can move by pin rules, are not obstructed, and block check if in check
    let can_push_once_pawns = can_move_pawns & single_push_available_move_pawns;

    let capture_targets = their_pieces & check_mask;

    let will_promote_rank = bitboards::pawn_back_rank(game.player.other());

    // Promotion capture: Pawns on the enemy's start rank will promote when capturing
    for pawn in can_capture_pawns & will_promote_rank {
        let mut attacks = tables::pawn_attacks(pawn, game.player);

        if diagonal_pins.contains(pawn) {
            attacks &= diagonal_pins;
        }

        for target in attacks & capture_targets {
            f(Move::capture_promotion(pawn, target, PromotionPieceKind::Queen));
            f(Move::capture_promotion(pawn, target, PromotionPieceKind::Rook));
            f(Move::capture_promotion(pawn, target, PromotionPieceKind::Knight));
            f(Move::capture_promotion(pawn, target, PromotionPieceKind::Bishop));
        }
    }

    // Promotion push: Pawns on the enemy's start rank will promote when pushing
    for pawn in can_push_once_pawns & will_promote_rank {
        let target = pawn.forward(game.player);

        // Pawns cannot push forward if they are pinned orthogonally
        // There's no 'moving along the pin ray' for these pieces, since the target square is empty
        if !orthogonal_pins.contains(pawn) {
            f(Move::quiet_promotion(pawn, target, PromotionPieceKind::Queen));
        }
    }

    // Non-promoting captures: All pawns can capture diagonally
    for pawn in can_capture_pawns & !will_promote_rank {
        let mut attacks = tables::pawn_attacks(pawn, game.player);

        if diagonal_pins.contains(pawn) {
            attacks &= diagonal_pins;
        }

        for target in attacks & capture_targets {
            f(Move::capture(pawn, target));
        }
    }

    // En-passant capture: Pawns either side of the en-passant pawn can capture
    if let Some(en_passant_target) = game.en_passant_target {
        let potential_capturers =
            can_capture_pawns & tables::pawn_attacks(en_passant_target, game.player.other());

        for potential_capturer in potential_capturers {
            if !diagonal_pins.contains(potential_capturer)
                || diagonal_pins.contains(en_passant_target)
            {
                f(Move::en_passant(potential_capturer, en_passant_target));
            }
        }
    }
}

fn generate_pawn_quiets(
    game: &Game,
    pawns: Bitboard,
    all_pieces: Bitboard,
    check_mask: Bitboard,
    orthogonal_pins: Bitboard,
    diagonal_pins: Bitboard,
    f: &mut impl FnMut(Move),
) {
    // Pawns that are pinned diagonally would reveal the king by moving forward
    let can_move_pawns = pawns & !diagonal_pins;

    // Pawns can move onto empty squares, as long as they block check if in check
    let available_move_squares = !all_pieces & check_mask;
    let single_push_available_move_pawns = available_move_squares.backward(game.player);

    // Pawns can push once if they can move by pin rules, are not obstructed, and block check if in check
    let can_push_once_pawns = can_move_pawns & single_push_available_move_pawns;

    let will_promote_rank = bitboards::pawn_back_rank(game.player.other());

    // Promotion push: Pawns on the enemy's start rank will promote when pushing
    for pawn in can_push_once_pawns & will_promote_rank {
        let target = pawn.forward(game.player);

        // Pawns cannot push forward if they are pinned orthogonally
        // There's no 'moving along the pin ray' for these pieces, since the target square is empty
        if !orthogonal_pins.contains(pawn) {
            // Consider underpromoting pushes to be 'quiet' moves
            f(Move::quiet_promotion(pawn, target, PromotionPieceKind::Rook));
            f(Move::quiet_promotion(pawn, target, PromotionPieceKind::Knight));
            f(Move::quiet_promotion(pawn, target, PromotionPieceKind::Bishop));
        }
    }

    let back_rank = bitboards::pawn_back_rank(game.player);

    // Push: All pawns with an empty square in front of them can move forward
    for pawn in can_push_once_pawns & !will_promote_rank {
        let forward_one = pawn.forward(game.player);

        // Pawns cannot push forward if they are pinned orthogonally, unless they're moving along the pin ray
        if !orthogonal_pins.contains(pawn) || orthogonal_pins.contains(forward_one) {
            f(Move::quiet(pawn, forward_one));
        }
    }

    let double_push_blockers = all_pieces.backward(game.player);

    let can_push_twice_pawns = can_move_pawns
        & back_rank
        & !double_push_blockers
        & single_push_available_move_pawns.backward(game.player);

    // Double push: All pawns on the start rank with empty squares in front of them can move forward two squares
    for pawn in can_push_twice_pawns {
        let forward_two = pawn.forward(game.player).forward(game.player);

        // Pawns cannot push forward if they are pinned orthogonally, unless they are moving along the pin ray
        if !orthogonal_pins.contains(pawn) || orthogonal_pins.contains(forward_two) {
            f(Move::double_push(pawn, forward_two));
        }
    }
}

fn generate_knight_captures(
    knights: Bitboard,
    their_pieces: Bitboard,
    check_mask: Bitboard,
    orthogonal_pins: Bitboard,
    diagonal_pins: Bitboard,
    f: &mut impl FnMut(Move),
) {
    // Pinned knights can't move
    for knight in knights & !(orthogonal_pins | diagonal_pins) {
        let destinations = tables::knight_attacks(knight) & check_mask;

        let capture_destinations = destinations & their_pieces;
        for dst in capture_destinations {
            f(Move::capture(knight, dst));
        }
    }
}

fn generate_knight_quiets(
    knights: Bitboard,
    all_pieces: Bitboard,
    check_mask: Bitboard,
    orthogonal_pins: Bitboard,
    diagonal_pins: Bitboard,
    f: &mut impl FnMut(Move),
) {
    // Pinned knights can't move
    for knight in knights & !(orthogonal_pins | diagonal_pins) {
        let destinations = tables::knight_attacks(knight) & check_mask;

        let move_destinations = destinations & !all_pieces;
        for dst in move_destinations {
            f(Move::quiet(knight, dst));
        }
    }
}

fn generate_diagonal_slider_captures(
    diagonal_sliders: Bitboard,
    their_pieces: Bitboard,
    all_pieces: Bitboard,
    check_mask: Bitboard,
    orthogonal_pins: Bitboard,
    diagonal_pins: Bitboard,
    f: &mut impl FnMut(Move),
) {
    // Diagonal sliders which are pinned orthogonally would expose the king by moving
    for diagonal_slider in diagonal_sliders & !orthogonal_pins {
        let mut destinations = tables::bishop_attacks(diagonal_slider, all_pieces) & check_mask;

        // If the slider is pinned, it can only move along the pin ray
        if diagonal_pins.contains(diagonal_slider) {
            destinations &= diagonal_pins;
        }

        let capture_destinations = destinations & their_pieces;
        for dst in capture_destinations {
            f(Move::capture(diagonal_slider, dst));
        }
    }
}

fn generate_diagonal_slider_quiets(
    diagonal_sliders: Bitboard,
    all_pieces: Bitboard,
    check_mask: Bitboard,
    orthogonal_pins: Bitboard,
    diagonal_pins: Bitboard,
    f: &mut impl FnMut(Move),
) {
    // Diagonal sliders which are pinned orthogonally would expose the king by moving
    for diagonal_slider in diagonal_sliders & !orthogonal_pins {
        let mut destinations = tables::bishop_attacks(diagonal_slider, all_pieces) & check_mask;

        // If the slider is pinned, it can only move along the pin ray
        if diagonal_pins.contains(diagonal_slider) {
            destinations &= diagonal_pins;
        }

        let move_destinations = destinations & !all_pieces;
        for dst in move_destinations {
            f(Move::quiet(diagonal_slider, dst));
        }
    }
}

fn generate_orthogonal_slider_captures(
    orthogonal_sliders: Bitboard,
    their_pieces: Bitboard,
    all_pieces: Bitboard,
    check_mask: Bitboard,
    orthogonal_pins: Bitboard,
    diagonal_pins: Bitboard,
    f: &mut impl FnMut(Move),
) {
    // Orthogonal sliders which are pinned diagonally would expose the king by moving
    for orthogonal_slider in orthogonal_sliders & !diagonal_pins {
        let mut destinations = tables::rook_attacks(orthogonal_slider, all_pieces) & check_mask;

        if orthogonal_pins.contains(orthogonal_slider) {
            destinations &= orthogonal_pins;
        }

        let capture_destinations = destinations & their_pieces;
        for dst in capture_destinations {
            f(Move::capture(orthogonal_slider, dst));
        }
    }
}

fn generate_orthogonal_slider_quiets(
    orthogonal_sliders: Bitboard,
    all_pieces: Bitboard,
    check_mask: Bitboard,
    orthogonal_pins: Bitboard,
    diagonal_pins: Bitboard,
    f: &mut impl FnMut(Move),
) {
    // Orthogonal sliders which are pinned diagonally would expose the king by moving
    for orthogonal_slider in orthogonal_sliders & !diagonal_pins {
        let mut destinations = tables::rook_attacks(orthogonal_slider, all_pieces) & check_mask;

        if orthogonal_pins.contains(orthogonal_slider) {
            destinations &= orthogonal_pins;
        }

        let move_destinations = destinations & !all_pieces;
        for dst in move_destinations {
            f(Move::quiet(orthogonal_slider, dst));
        }
    }
}

fn generate_king_captures(
    game: &Game,
    king: Square,
    their_pieces: Bitboard,
    f: &mut impl FnMut(Move),
) {
    let destinations = tables::king_attacks(king);

    // When calculating the attacked squares, we need to remove our King from the board.
    // If we don't, squares behind the king look safe (since they are blocked by the king)
    // meaning we'd generate moves away from a slider while in check.
    let mut board_without_king = game.board.clone();
    board_without_king.remove_at(king);

    for dst in destinations & their_pieces {
        if attackers::generate_attackers_of(&board_without_king, game.player, dst).is_empty() {
            f(Move::capture(king, dst));
        }
    }
}

fn generate_king_quiets(game: &Game, king: Square, all_pieces: Bitboard, f: &mut impl FnMut(Move)) {
    let destinations = tables::king_attacks(king);

    // When calculating the attacked squares, we need to remove our King from the board.
    // If we don't, squares behind the king look safe (since they are blocked by the king)
    // meaning we'd generate moves away from a slider while in check.
    let mut board_without_king = game.board.clone();
    board_without_king.remove_at(king);

    for dst in destinations & !all_pieces {
        if attackers::generate_attackers_of(&board_without_king, game.player, dst).is_empty() {
            f(Move::quiet(king, dst));
        }
    }
}

fn generate_castles(game: &Game, all_pieces: Bitboard, f: &mut impl FnMut(Move)) {
    let castle_rights_for_player = game.castle_rights[game.player];

    if !game.is_frc {
        if let Some(kingside_dst) = castle_rights_for_player.king_side {
            generate_castle_move_for_side(
                game,
                all_pieces,
                f,
                kingside_dst,
                squares::kingside_king_castle_end(game.player),
            );
        }

        if let Some(queenside_dst) = castle_rights_for_player.queen_side {
            generate_castle_move_for_side(
                game,
                all_pieces,
                f,
                queenside_dst,
                squares::queenside_king_castle_end(game.player),
            );
        }
    } else {
        if let Some(kingside_dst) = castle_rights_for_player.king_side {
            generate_frc_castle_move_for_side(
                game,
                all_pieces,
                f,
                kingside_dst,
                squares::kingside_rook_castle_end(game.player),
                squares::kingside_king_castle_end(game.player),
            );
        }

        if let Some(queenside_dst) = castle_rights_for_player.queen_side {
            generate_frc_castle_move_for_side(
                game,
                all_pieces,
                f,
                queenside_dst,
                squares::queenside_rook_castle_end(game.player),
                squares::queenside_king_castle_end(game.player),
            );
        }
    }
}

fn generate_castle_move_for_side(
    game: &Game,
    all_pieces: Bitboard,
    f: &mut impl FnMut(Move),
    rook: Square,
    king_dst: Square,
) {
    let king = game.board.king_square(game.player);

    let required_empty_squares = ray_between(king, rook);
    let required_safe_squares = ray_between(king, king_dst) | king_dst.bb();

    if (required_empty_squares & all_pieces).is_empty()
        && (required_safe_squares & game.threats).is_empty()
    {
        f(Move::castles(king, rook));
    }
}

fn generate_frc_castle_move_for_side(
    game: &Game,
    all_pieces: Bitboard,
    f: &mut impl FnMut(Move),
    rook: Square,
    rook_dst: Square,
    king_dst: Square,
) {
    if game.orthogonal_pins.contains(rook) {
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
        let game = Game::from_fen(fen).unwrap();
        let mut movelist = MoveList::new();
        generate_legal_moves(&game, |m| movelist.push(m));

        assert!(movelist.iter().any(|m| (m.from(), m.to()) == mv));
    }

    #[inline(always)]
    fn should_not_allow_move(fen: &str, mv: (Square, Square)) {
        crate::init();
        let game = Game::from_fen(fen).unwrap();
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

        let mut game = Game::from_fen("8/8/8/8/k3p2Q/8/3P4/3K4 w - - 0 1").unwrap();
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
        )
    }

    #[test]
    fn test_en_passant_diagonal_pin_20260413() {
        crate::init();

        let mut game =
            Game::from_fen("q7/2p3k1/3b2p1/pp1Pnp1p/3Q3P/P1P3P1/1P3PK1/2NB4 b - - 1 33").unwrap();
        game.make_move(game.moves().expect_matching(C7, C5, None));

        assert!(game.moves().iter().any(|m| (m.from(), m.to()) == (D5, C6)));
    }
}
