use crate::{
    chess::{
        Game, Move, PieceKind, Player,
        moves::{all_attackers_of, bishop_attacks, rook_attacks},
        rays::ray_skewering,
    },
    engine::{eval::Eval, params::*},
};

static mut SEE_VALUES: [i32; PieceKind::N] = [
    see_pawn_value(),
    see_knight_value(),
    see_bishop_value(),
    see_rook_value(),
    see_queen_value(),
    10000,
];

pub const fn see_value(piece_kind: PieceKind) -> Eval {
    Eval(unsafe { SEE_VALUES[piece_kind as usize] })
}

pub fn init_see_values(
    pawn_value: i32,
    knight_value: i32,
    bishop_value: i32,
    rook_value: i32,
    queen_value: i32,
) {
    unsafe {
        SEE_VALUES[PieceKind::Pawn] = pawn_value;
        SEE_VALUES[PieceKind::Knight] = knight_value;
        SEE_VALUES[PieceKind::Bishop] = bishop_value;
        SEE_VALUES[PieceKind::Rook] = rook_value;
        SEE_VALUES[PieceKind::Queen] = queen_value;
    }
}

pub fn see(game: &Game, mv: Move, threshold: Eval) -> bool {
    if mv.is_castling() {
        return true;
    }

    let from = mv.from();
    let to = mv.to();
    let board = &game.board;
    let moved_piece = board.piece_guaranteed_at(from);

    // We have to beat the threshold in order to pass, which is the same as saying that
    //      score - threshold = 0
    // We can initialise the score to -threshold to avoid having to repeatedly subtract threshold
    let mut score = -threshold;

    // First, make the move and adjust the score accordingly
    //
    // If we captured a piece during the move, we score according to that piece's value
    score += board.captured_piece(mv).map_or(Eval(0), see_value);

    // If we promoted a pawn, we lose the pawn and gain the value of the piece we promoted to
    if let Some(promotion_piece) = mv.promotion() {
        score -= see_value(PieceKind::Pawn);
        score += see_value(promotion_piece.piece());
    }

    // The piece we just moved will be the first victim of the exchange on the target square
    let mut victim = match mv.promotion() {
        Some(promotion_piece) => promotion_piece.piece(),
        None => moved_piece.kind,
    };

    let mut occupied = board.occupancy();
    occupied.unset(from);
    occupied.set(to);

    if mv.is_en_passant() {
        occupied.unset(game.en_passant_target.unwrap());
    }

    let white_pinned = game.pinned[Player::White];
    let white_pin_ray = ray_skewering(board.king_square(Player::White), to);

    let black_pinned = game.pinned[Player::Black];
    let black_pin_ray = ray_skewering(board.king_square(Player::Black), to);

    let not_pinned = !(white_pinned | black_pinned)
        | (white_pinned & white_pin_ray)
        | (black_pinned & black_pin_ray);

    let mut diagonal_sliders = board.all_diagonal_sliders() & occupied;
    let mut orthorgonal_sliders = board.all_orthogonal_sliders() & occupied;

    let mut attackers = all_attackers_of(board, to, occupied) & occupied & not_pinned;

    let mut color = game.player;

    loop {
        color = color.other();

        // If we're winning and it's our turn, we've won the exchange
        if (color == game.player && score >= Eval(0)) || (color != game.player && score <= Eval(0))
        {
            break;
        }

        let my_attackers = attackers & board.occupancy_for(color);
        if my_attackers.is_empty() {
            break;
        }

        // Loop through potential attackers in order of value, and stop whenever we've found one
        // we can use
        let mut attacker_sq = None;
        for potential_attacker_kind in PieceKind::ALL {
            let mut potential_attacker_squares =
                my_attackers & board.pieces_of_kind(potential_attacker_kind, color);

            if potential_attacker_squares.any() {
                attacker_sq = Some(potential_attacker_squares.pop_lsb_inplace().single());
                break;
            }
        }

        let attacker_sq = attacker_sq.unwrap();
        let attacker = board.piece_guaranteed_at(attacker_sq).kind;

        // If we capture with a king and the opponent is attacking the square, we just captured into
        // check
        if attacker == PieceKind::King && (attackers & board.occupancy_for(color.other())).any() {
            break;
        }

        // The attacker that just captured is no longer on its old square
        occupied ^= attacker_sq.bb();
        attackers &= occupied;
        diagonal_sliders &= occupied;
        orthorgonal_sliders &= occupied;

        if attacker == PieceKind::Pawn
            || attacker == PieceKind::Bishop
            || attacker == PieceKind::Queen
        {
            attackers |= bishop_attacks(to, occupied) & diagonal_sliders;
        }

        if attacker == PieceKind::Rook || attacker == PieceKind::Queen {
            attackers |= rook_attacks(to, occupied) & orthorgonal_sliders;
        }

        if color == game.player {
            score += see_value(victim);
        } else {
            score -= see_value(victim);
        }

        // The attacker that just captured is the next piece to be captured
        victim = attacker;
    }

    score >= Eval(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{Game, Square, moves::MoveListExt, squares::all::*};

    fn use_basic_see_values() {
        super::init_see_values(100, 300, 300, 500, 900);
    }

    fn should_be_good_capture(fen: &str, mv: (Square, Square)) {
        crate::init();
        use_basic_see_values();

        let game = Game::from_valid_fen(fen);
        let mv = game.moves().expect_matching(mv.0, mv.1, None);

        assert!(see(&game, mv, Eval(0)));
    }

    fn should_be_bad_capture(fen: &str, mv: (Square, Square)) {
        crate::init();
        use_basic_see_values();

        let game = Game::from_valid_fen(fen);
        let mv = game.moves().expect_matching(mv.0, mv.1, None);

        assert!(!see(&game, mv, Eval(0)));
    }

    #[test]
    fn test_see_bad_captures() {
        should_be_bad_capture("8/k1q5/8/4p3/8/4Q3/K7/8 w - - 0 1", (E3, E5));
        should_be_bad_capture("8/k1q2n2/8/4p2R/8/2B1Q3/K7/8 w - - 0 1", (E3, E5));
        should_be_bad_capture("k7/2q2n2/8/4p2R/5P2/2B1Q3/8/6K1 w - - 0 1", (E3, E5));
    }

    #[test]
    fn test_see_good_captures() {
        should_be_good_capture("8/k1q5/8/4p3/8/4Q1B1/K7/8 w - - 0 1", (E3, E5));
        should_be_good_capture("k7/2q2n2/8/4p2R/5P2/2B1Q3/8/6K1 w - - 0 1", (C3, E5));
        should_be_good_capture("K7/8/8/3p4/4P3/8/8/7k w - - 0 1", (E4, E5));
    }

    // Test suite stolen from Simbelmyne, which was stolen from Carp
    #[test]
    fn test_see_carp() {
        #[rustfmt::skip]
        const SEE_SUITE: [(&str, &str, i32, bool); 11] = [
            ("1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - - 0 1", "e1e5", 0, true),
            ("1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1", "d3e5", 0, false),
            ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", "g2h3", 0, true),
            ("k3r3/8/8/4p3/8/2B5/1B6/K7 w - - 0 1", "c3e5", 0, true),
            ("4kbnr/p1P4p/b1q5/5pP1/4n3/5Q2/PP1PPP1P/RNB1KBNR w KQk f6 0 1", "g5f6", 0, true),
            ("6k1/1pp4p/p1pb4/6q1/3P1pRr/2P4P/PP1Br1P1/5RKN w - - 0 1", "f1f4", 0, false),
            ("6RR/4bP2/8/8/5r2/3K4/5p2/4k3 w - - 0 1", "f7f8q", 0, true),
            ("r1bqk1nr/pppp1ppp/2n5/1B2p3/1b2P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1", "e1h1", 0, true),
            ("4kbnr/p1P1pppp/b7/4q3/7n/8/PPQPPPPP/RNB1KBNR w KQk - 0 1", "c7c8q", 0, true),
            ("4kbnr/p1P1pppp/b7/4q3/7n/8/PP1PPPPP/RNBQKBNR w KQk - 0 1", "c7c8q", 0, false),
            ("3r3k/3r4/2n1n3/8/3p4/2PR4/1B1Q4/3R3K w - - 0 1", "d3d4", 0, false),
            // ("5rk1/1pp2q1p/p1pb4/8/3P1NP1/2P5/1P1BQ1P1/5RK1 b - - 0 1", "d6f4", 0, false),
            // ("5rk1/1pp2q1p/p1pb4/8/3P1NP1/2P5/1P1BQ1P1/5RK1 b - - 0 1", "d6f4", -100, true),
        ];

        crate::init();
        use_basic_see_values();

        for (fen, ucimv, threshold, result) in SEE_SUITE {
            println!("{fen}");
            let game = Game::from_valid_fen(fen);
            let moves = game.moves();

            let mv = moves.iter().find(|m| format!("{m:?}") == ucimv).unwrap();

            assert_eq!(see(&game, *mv, Eval(threshold)), result);
        }
    }

    // Test suite stolen from Starzix
    #[test]
    fn test_see_starzix() {
        crate::init();
        use_basic_see_values();

        #[rustfmt::skip]
        let suite: Vec<(&str, &str, i32, bool)> = vec![
            ("6k1/1pp4p/p1pb4/6q1/3P1pRr/2P4P/PP1Br1P1/5RKN w - - 0 1", "f1f4", -100, true),
            ("5rk1/1pp2q1p/p1pb4/8/3P1NP1/2P5/1P1BQ1P1/5RK1 b - - 0 1", "d6f4", 0, true),
            ("4R3/2r3p1/5bk1/1p1r3p/p2PR1P1/P1BK1P2/1P6/8 b - - 0 1", "h5g4", 0, true),
            ("4R3/2r3p1/5bk1/1p1r1p1p/p2PR1P1/P1BK1P2/1P6/8 b - - 0 1", "h5g4", 0, true),
            ("4r1k1/5pp1/nbp4p/1p2p2q/1P2P1b1/1BP2N1P/1B2QPPK/3R4 b - - 0 1", "g4f3", 0, true),
            ("2r1r1k1/pp1bppbp/3p1np1/q3P3/2P2P2/1P2B3/P1N1B1PP/2RQ1RK1 b - - 0 1", "d6e5", 100, true),
            ("7r/5qpk/p1Qp1b1p/3r3n/BB3p2/5p2/P1P2P2/4RK1R w - - 0 1", "e1e8", 0, true),
            ("6rr/6pk/p1Qp1b1p/2n5/1B3p2/5p2/P1P2P2/4RK1R w - - 0 1", "e1e8", -500, true),
            ("7r/5qpk/2Qp1b1p/1N1r3n/BB3p2/5p2/P1P2P2/4RK1R w - - 0 1", "e1e8", -500,  true),
            ("6RR/4bP2/8/8/5r2/3K4/5p2/4k3 w - - 0 1", "f7f8q", 200, true),
            ("6RR/4bP2/8/8/5r2/3K4/5p2/4k3 w - - 0 1", "f7f8n", 200, true),
            ("7R/5P2/8/8/6r1/3K4/5p2/4k3 w - - 0 1", "f7f8q", 800, true),
            ("7R/5P2/8/8/6r1/3K4/5p2/4k3 w - - 0 1", "f7f8b", 200, true),
            ("7R/4bP2/8/8/1q6/3K4/5p2/4k3 w - - 0 1", "f7f8r", -100, true),
            ("8/4kp2/2npp3/1Nn5/1p2PQP1/7q/1PP1B3/4KR1r b - - 0 1", "h1f1", 0, true),
            ("8/4kp2/2npp3/1Nn5/1p2P1P1/7q/1PP1B3/4KR1r b - - 0 1", "h1f1", 0, true),
            ("2r2r1k/6bp/p7/2q2p1Q/3PpP2/1B6/P5PP/2RR3K b - - 0 1", "c5c1", 100, true),
            ("r2qk1nr/pp2ppbp/2b3p1/2p1p3/8/2N2N2/PPPP1PPP/R1BQR1K1 w kq - 0 1", "f3e5", 100, true),
            ("6r1/4kq2/b2p1p2/p1pPb3/p1P2B1Q/2P4P/2B1R1P1/6K1 w - - 0 1", "f4e5", 0, true),
            ("3q2nk/pb1r1p2/np6/3P2Pp/2p1P3/2R4B/PQ3P1P/3R2K1 w - h6 0 1", "g5h6", 0, true),
            ("3q2nk/pb1r1p2/np6/3P2Pp/2p1P3/2R1B2B/PQ3P1P/3R2K1 w - h6 0 1", "g5h6", 100, true),
            ("2r4r/1P4pk/p2p1b1p/7n/BB3p2/2R2p2/P1P2P2/4RK2 w - - 0 1", "c3c8", 500, true),
            ("2r5/1P4pk/p2p1b1p/5b1n/BB3p2/2R2p2/P1P2P2/4RK2 w - - 0 1", "c3c8", 500, true),
            ("2r4k/2r4p/p7/2b2p1b/4pP2/1BR5/P1R3PP/2Q4K w - - 0 1", "c3c5", 300, true),
            ("8/pp6/2pkp3/4bp2/2R3b1/2P5/PP4B1/1K6 w - - 0 1", "g2c6", -200, true),
            ("4q3/1p1pr1k1/1B2rp2/6p1/p3PP2/P3R1P1/1P2R1K1/4Q3 b - - 0 1", "e6e4", -400, true),
            ("4q3/1p1pr1kb/1B2rp2/6p1/p3PP2/P3R1P1/1P2R1K1/4Q3 b - - 0 1", "h7e4", 100, true),
            ("3r3k/3r4/2n1n3/8/3p4/2PR4/1B1Q4/3R3K w - - 0 1", "d3d4", -100, true),
            ("1k1r4/1ppn3p/p4b2/4n3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1", "d3e5", 100, true),
            ("1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1", "d3e5", -200, true),
            ("rnb2b1r/ppp2kpp/5n2/4P3/q2P3B/5R2/PPP2PPP/RN1QKB2 w Q - 0 1", "h4f6", 100, true),
            ("r2q1rk1/2p1bppp/p2p1n2/1p2P3/4P1b1/1nP1BN2/PP3PPP/RN1QR1K1 b - - 0 1", "g4f3", 0, true),
            ("r1bqkb1r/2pp1ppp/p1n5/1p2p3/3Pn3/1B3N2/PPP2PPP/RNBQ1RK1 b kq - 0 1", "c6d4", 0, true),
            ("r1bq1r2/pp1ppkbp/4N1p1/n3P1B1/8/2N5/PPP2PPP/R2QK2R w KQ - 0 1", "e6g7", 0, true),
            ("r1bq1r2/pp1ppkbp/4N1pB/n3P3/8/2N5/PPP2PPP/R2QK2R w KQ - 0 1", "e6g7", 300, true),
            ("rnq1k2r/1b3ppp/p2bpn2/1p1p4/3N4/1BN1P3/PPP2PPP/R1BQR1K1 b kq - 0 1", "d6h2", -200, true),
            ("rn2k2r/1bq2ppp/p2bpn2/1p1p4/3N4/1BN1P3/PPP2PPP/R1BQR1K1 b kq - 0 1", "d6h2", 100, true),
            ("r2qkbn1/ppp1pp1p/3p1rp1/3Pn3/4P1b1/2N2N2/PPP2PPP/R1BQKB1R b KQq - 0 1", "g4f3", 100, true),
            ("rnbq1rk1/pppp1ppp/4pn2/8/1bPP4/P1N5/1PQ1PPPP/R1B1KBNR b KQ - 0 1", "b4c3", 0, true),
            ("r4rk1/3nppbp/bq1p1np1/2pP4/8/2N2NPP/PP2PPB1/R1BQR1K1 b - - 0 1", "b6b2", -800, true),
            ("r4rk1/1q1nppbp/b2p1np1/2pP4/8/2N2NPP/PP2PPB1/R1BQR1K1 b - - 0 1", "f6d5", -200, true),
            ("1r3r2/5p2/4p2p/2k1n1P1/2PN1nP1/1P3P2/8/2KR1B1R b - - 0 1", "b8b3", -400, true),
            ("1r3r2/5p2/4p2p/4n1P1/kPPN1nP1/5P2/8/2KR1B1R b - - 0 1", "b8b4", 100, true),
            ("2r2rk1/5pp1/pp5p/q2p4/P3n3/1Q3NP1/1P2PP1P/2RR2K1 b - - 0 1", "c8c1", 0, true),
            ("5rk1/5pp1/2r4p/5b2/2R5/6Q1/R1P1qPP1/5NK1 b - - 0 1", "f5c2", -100, true),
            ("1r3r1k/p4pp1/2p1p2p/qpQP3P/2P5/3R4/PP3PP1/1K1R4 b - - 0 1", "a5a2", -800, true),
            ("1r5k/p4pp1/2p1p2p/qpQP3P/2P2P2/1P1R4/P4rP1/1K1R4 b - - 0 1", "a5a2", 100, true),
            ("r2q1rk1/1b2bppp/p2p1n2/1ppNp3/3nP3/P2P1N1P/BPP2PP1/R1BQR1K1 w - - 0 1", "d5e7", 0, true),
            ("rnbqrbn1/pp3ppp/3p4/2p2k2/4p3/3B1K2/PPP2PPP/RNB1Q1NR w - - 0 1", "d3e4", 100, true),
            ("rnb1k2r/p3p1pp/1p3p1b/7n/1N2N3/3P1PB1/PPP1P1PP/R2QKB1R w KQkq - 0 1", "e4d6", -200, true),
            ("r1b1k2r/p4npp/1pp2p1b/7n/1N2N3/3P1PB1/PPP1P1PP/R2QKB1R w KQkq - 0 1", "e4d6", 0, true),
            ("2r1k2r/pb4pp/5p1b/2KB3n/4N3/2NP1PB1/PPP1P1PP/R2Q3R w k - 0 1", "d5c6", -300, true),
            ("2r1k2r/pb4pp/5p1b/2KB3n/1N2N3/3P1PB1/PPP1P1PP/R2Q3R w k - 0 1", "d5c6", 0, true),
            ("2r1k3/pbr3pp/5p1b/2KB3n/1N2N3/3P1PB1/PPP1P1PP/R2Q3R w - - 0 1", "d5c6", -300, true),
            ("5k2/p2P2pp/8/1pb5/1Nn1P1n1/6Q1/PPP4P/R3K1NR w KQ - 0 1", "d7d8q", 800, true),
            ("r4k2/p2P2pp/8/1pb5/1Nn1P1n1/6Q1/PPP4P/R3K1NR w KQ - 0 1", "d7d8q", -100, true),
            ("5k2/p2P2pp/1b6/1p6/1Nn1P1n1/8/PPP4P/R2QK1NR w KQ - 0 1", "d7d8q", 200, true),
            ("4kbnr/p1P1pppp/b7/4q3/7n/8/PP1PPPPP/RNBQKBNR w KQk - 0 1", "c7c8q", -100, true),
            ("4kbnr/p1P1pppp/b7/4q3/7n/8/PPQPPPPP/RNB1KBNR w KQk - 0 1", "c7c8q", 200, true),
            ("4kbnr/p1P1pppp/b7/4q3/7n/8/PPQPPPPP/RNB1KBNR w KQk - 0 1", "c7c8q", 200, true),
            ("4kbnr/p1P4p/b1q5/5pP1/4n3/5Q2/PP1PPP1P/RNB1KBNR w KQk f6 0 1", "g5f6", 0, true),
            ("4kbnr/p1P4p/b1q5/5pP1/4n3/5Q2/PP1PPP1P/RNB1KBNR w KQk f6 0 1", "g5f6", 0, true),
            ("4kbnr/p1P4p/b1q5/5pP1/4n2Q/8/PP1PPP1P/RNB1KBNR w KQk f6 0 1", "g5f6", 0, true),
            ("1n2kb1r/p1P4p/2qb4/5pP1/4n2Q/8/PP1PPP1P/RNB1KBNR w KQk - 0 1", "c7b8q", 200, true),
            ("rnbqk2r/pp3ppp/2p1pn2/3p4/3P4/N1P1BN2/PPB1PPPb/R2Q1RK1 w kq - 0 1", "g1h2", 300, true),
            ("3N4/2K5/2n5/1k6/8/8/8/8 b - - 0 1", "c6d8", 0, true),
            ("3n3r/2P5/8/1k6/8/8/3Q4/4K3 w - - 0 1", "c7d8q", 700, true),
            ("r2n3r/2P1P3/4N3/1k6/8/8/8/4K3 w - - 0 1", "e6d8", 300, true),
            ("8/8/8/1k6/6b1/4N3/2p3K1/3n4 w - - 0 1", "e3d1", 0, true),
            ("8/8/1k6/8/8/2N1N3/4p1K1/3n4 w - - 0 1", "c3d1", 100, true),
            ("r1bqk1nr/pppp1ppp/2n5/1B2p3/1b2P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1", "e1h1", 0, true)
        ];

        for (fen, ucimv, threshold, result) in suite {
            println!("{fen}");
            let game = Game::from_valid_fen(fen);
            let moves = game.moves();

            let mv = moves.iter().find(|m| format!("{m:?}") == ucimv).unwrap();

            assert_eq!(see(&game, *mv, Eval(threshold)), result);
        }
    }
}
