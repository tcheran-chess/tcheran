use crate::chess::{
    Board, CastleRights, File, Game, Piece, PieceKind, Player, Square, ranks::back_rank,
};

fn place_in_nth_available(
    n: usize,
    rank: &mut [Option<PieceKind>; File::N],
    piece: PieceKind,
) -> File {
    assert!(n < File::N);

    let mut x = 0;

    for (i, file) in File::ALL.iter().enumerate() {
        if rank[i].is_none() {
            if x == n {
                rank[i] = Some(piece);
                return *file;
            }

            x += 1;
        }
    }

    panic!();
}

fn idx_to_backrank(idx: usize) -> ([PieceKind; File::N], [File; 2]) {
    use File::*;
    use PieceKind::*;

    let mut backrank = [None; File::N];

    let (n2, b1) = (idx / 4, idx % 4);
    let (n3, b2) = (n2 / 4, n2 % 4);
    let (n4, q) = (n3 / 6, n3 % 6);

    let b1_file = match b1 {
        0 => B,
        1 => D,
        2 => F,
        3 => H,
        _ => unreachable!(),
    };

    let b2_file = match b2 {
        0 => A,
        1 => C,
        2 => E,
        3 => G,
        _ => unreachable!(),
    };

    let (knight1_idx, knight2_idx) = match n4 {
        0 => (0, 0),
        1 => (0, 1),
        2 => (0, 2),
        3 => (0, 3),
        4 => (1, 1),
        5 => (1, 2),
        6 => (1, 3),
        7 => (2, 2),
        8 => (2, 3),
        9 => (3, 3),
        _ => unreachable!(),
    };

    backrank[b1_file] = Some(Bishop);
    backrank[b2_file] = Some(Bishop);
    place_in_nth_available(q, &mut backrank, Queen);
    place_in_nth_available(knight1_idx, &mut backrank, Knight);
    place_in_nth_available(knight2_idx, &mut backrank, Knight);
    let r1 = place_in_nth_available(0, &mut backrank, Rook);
    place_in_nth_available(0, &mut backrank, King);
    let r2 = place_in_nth_available(0, &mut backrank, Rook);

    let backrank = backrank.map(|p| p.unwrap());
    let rook_indices = [File::from_idx(r1 as u8), File::from_idx(r2 as u8)];

    (backrank, rook_indices)
}

pub fn from_idxs(white_idx: usize, black_idx: usize) -> (Board, [CastleRights; Player::N]) {
    assert!(white_idx < 960);
    assert!(black_idx < 960);

    let mut board = Game::new().board;

    let white_rank = back_rank(Player::White);
    let black_rank = back_rank(Player::Black);

    let (white_backrank, [white_queenside_rook, white_kingside_rook]) = idx_to_backrank(white_idx);
    let (black_backrank, [black_queenside_rook, black_kingside_rook]) = idx_to_backrank(black_idx);

    let white_backrank = white_backrank.map(|p| Piece::new(Player::White, p));
    let black_backrank = black_backrank.map(|p| Piece::new(Player::Black, p));

    for file in File::ALL {
        let white_sq = Square::from_file_and_rank(file, white_rank);
        let black_sq = Square::from_file_and_rank(file, black_rank);

        board.remove_at(white_sq);
        board.remove_at(black_sq);

        board.set_at(white_sq, white_backrank[file]);
        board.set_at(black_sq, black_backrank[file]);
    }

    let white_castle_rights = CastleRights {
        king_side: Some(Square::from_file_and_rank(white_kingside_rook, white_rank)),
        queen_side: Some(Square::from_file_and_rank(white_queenside_rook, white_rank)),
    };

    let black_castle_rights = CastleRights {
        king_side: Some(Square::from_file_and_rank(black_kingside_rook, black_rank)),
        queen_side: Some(Square::from_file_and_rank(black_queenside_rook, black_rank)),
    };

    (board, [white_castle_rights, black_castle_rights])
}

#[cfg(test)]
mod tests {
    use crate::chess::game::Game;

    fn check_indexes_match_fen(white_idx: usize, black_idx: usize, fen: &str) {
        let game = Game::new_dfrc(white_idx, black_idx);
        assert_eq!(game.to_fen(), fen);
    }

    fn check_dfrc_idx_matches_fen(idx: usize, fen: &str) {
        let (white_idx, black_idx) = unpack_dfrc_idx(idx);
        check_indexes_match_fen(white_idx, black_idx, fen);
    }

    fn unpack_dfrc_idx(idx: usize) -> (usize, usize) {
        let black_idx = idx / 960;
        let white_idx = idx % 960;
        (white_idx, black_idx)
    }

    #[test]
    fn test_dfrc_from_idx() {
        crate::init();

        check_dfrc_idx_matches_fen(0, "bbqnnrkr/pppppppp/8/8/8/8/PPPPPPPP/BBQNNRKR w HFhf - 0 1");
        check_dfrc_idx_matches_fen(
            23484,
            "nbqnbrkr/pppppppp/8/8/8/8/PPPPPPPP/RBNNQKBR w HAhf - 0 1",
        );
        check_dfrc_idx_matches_fen(
            92382,
            "bbqnrnkr/pppppppp/8/8/8/8/PPPPPPPP/NQRKNBBR w HChe - 0 1",
        );
    }
}
